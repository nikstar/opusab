use crate::{
    backend::{self, Progress},
    error::fail,
    model::{self, Book, Channels, MetadataPatch, Preset, Recommendation},
    ogg, probe,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Clone, Debug, clap::Args)]
pub struct ConversionOptions {
    /// Output .opus path. Existing source Opus files are edited in place unless re-encoding.
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    /// JSON metadata patch, or - for stdin. Relative cover paths resolve beside this file.
    #[arg(short, long)]
    pub metadata: Option<PathBuf>,
    /// Target VBR bitrate in kbit/s (6..510). Omit to use the recommendation.
    #[arg(short,long,value_parser=clap::value_parser!(u32).range(6..=510))]
    pub bitrate: Option<u32>,
    #[arg(long, value_enum, default_value = "speech")]
    pub preset: Preset,
    #[arg(long, value_enum, default_value = "keep")]
    pub channels: Channels,
    /// Encoder workers. 0 selects up to 8 available cores.
    #[arg(short, long, default_value_t = 0)]
    pub jobs: usize,
    /// Extra space for future metadata edits, in KiB.
    #[arg(long,default_value_t=256,value_parser=clap::value_parser!(u32).range(0..=8192))]
    pub padding_kib: u32,
    /// Allow replacing an existing output. Source audio is never overwritten by conversion.
    #[arg(long)]
    pub overwrite: bool,
    /// Re-encode even if the input is already Opus.
    #[arg(long)]
    pub reencode: bool,
}
impl Default for ConversionOptions {
    fn default() -> Self {
        Self {
            output: None,
            metadata: None,
            bitrate: None,
            preset: Preset::Speech,
            channels: Channels::Keep,
            jobs: 0,
            padding_kib: 256,
            overwrite: false,
            reencode: false,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Plan {
    pub schema_version: u32,
    pub operation: String,
    pub output: PathBuf,
    pub book: Book,
    pub recommendation: Recommendation,
    pub target_bitrate_kbps: u32,
    pub channels: Channels,
    pub jobs: usize,
    pub padding_bytes: usize,
    pub overwrite: bool,
    pub metadata_patch: MetadataPatch,
}
pub fn read_patch(path: &Path) -> Result<MetadataPatch> {
    let mut bytes = Vec::new();
    if path == Path::new("-") {
        std::io::stdin()
            .take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
    } else {
        File::open(path)?
            .take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
    }
    ensure!(
        bytes.len() <= 8 * 1024 * 1024,
        "Metadata JSON exceeds 8 MiB"
    );
    let mut v: serde_json::Value =
        serde_json::from_slice(&bytes).context("Invalid metadata JSON")?;
    if let Some(version) = v.get("schema_version") {
        ensure!(version == 1, "Unsupported metadata schema version");
    }
    if v.get("metadata").is_some() {
        v = v["metadata"].take();
    }
    let mut patch: MetadataPatch = serde_json::from_value(v).context("Invalid metadata fields")?;
    if let Some(cover) = &mut patch.cover
        && cover.is_relative()
        && path != Path::new("-")
    {
        *cover = path.parent().unwrap_or(Path::new(".")).join(&cover);
    }
    Ok(patch)
}
pub fn make_plan(paths: &[PathBuf], options: &ConversionOptions) -> Result<Plan> {
    let book = probe::book(paths)?;
    let patch = options
        .metadata
        .as_ref()
        .map(|p| read_patch(p))
        .transpose()?
        .unwrap_or_default();
    let plan = plan_book(book, options, patch)?;
    validate_cover(&plan)?;
    Ok(plan)
}
pub fn plan_book(
    mut book: Book,
    options: &ConversionOptions,
    patch: MetadataPatch,
) -> Result<Plan> {
    patch.validate()?;
    ensure!(
        !book.sources.is_empty(),
        "At least one audio source is required"
    );
    patch.apply(&mut book.metadata);
    model::validate_metadata(&book.metadata, Some(book.duration_ms))?;
    let recommendation = model::recommend(&book, options.preset);
    let one_opus = book.sources.len() == 1 && book.sources[0].codec == "opus";
    let encode = !one_opus
        || options.reencode
        || options.bitrate.is_some()
        || options.channels != Channels::Keep
        || options.preset != Preset::Speech;
    let output = options.output.clone().unwrap_or_else(|| {
        if one_opus && !encode {
            book.sources[0].path.clone()
        } else if one_opus {
            book.sources[0].path.with_file_name(format!(
                "{}-converted.opus",
                book.sources[0]
                    .path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
            ))
        } else if let Some(dir) = &book.input_directory {
            dir.parent()
                .unwrap_or(Path::new("."))
                .join(format!("{}.opus", safe_filename(&book.metadata.title)))
        } else if book.sources.len() == 1 {
            book.sources[0].path.with_extension("opus")
        } else {
            book.sources[0]
                .path
                .parent()
                .unwrap_or(Path::new("."))
                .join(format!("{}.opus", safe_filename(&book.metadata.title)))
        }
    });
    ensure!(
        output
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("opus")),
        "Output filename must end with .opus"
    );
    let output = absolute_output(&output)?;
    let same_source = book.sources.iter().any(|s| s.path == output);
    ensure!(
        !(encode && same_source),
        "Conversion output must differ from every source file"
    );
    let operation = if encode {
        "encode"
    } else if same_source {
        "metadata"
    } else {
        "copy"
    };
    let jobs = if options.jobs == 0 {
        std::thread::available_parallelism()
            .map(|v| v.get())
            .unwrap_or(2)
            .min(8)
    } else {
        options.jobs
    };
    ensure!((1..=256).contains(&jobs), "Worker count must be 1..256");
    let target_bitrate = if encode {
        options.bitrate.unwrap_or(recommendation.bitrate_kbps)
    } else {
        book.sources[0].bitrate_kbps
    };
    Ok(Plan {
        schema_version: 1,
        operation: operation.into(),
        output,
        book,
        recommendation,
        target_bitrate_kbps: target_bitrate,
        channels: options.channels,
        jobs,
        padding_bytes: options.padding_kib as usize * 1024,
        overwrite: options.overwrite,
        metadata_patch: patch,
    })
}
pub fn absolute_output(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return Ok(path.canonicalize()?);
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    Ok(parent
        .canonicalize()
        .with_context(|| format!("Output directory does not exist: {}", parent.display()))?
        .join(path.file_name().context("Invalid output name")?))
}
fn safe_filename(s: &str) -> String {
    let s: String = s
        .chars()
        .map(|c| {
            if c.is_control() || "/\\:*?\"<>|".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    if s.trim().is_empty() {
        "audiobook".into()
    } else {
        s.trim().into()
    }
}

#[derive(Serialize, Deserialize)]
pub struct ConversionReport {
    pub schema_version: u32,
    pub operation: String,
    pub output: PathBuf,
    pub duration_ms: u64,
    pub size_bytes: u64,
    pub elapsed_seconds: f64,
    pub metadata: ogg::EditReport,
}
pub fn execute(plan: &Plan, progress: &mut dyn FnMut(Progress)) -> Result<ConversionReport> {
    let start = Instant::now();
    validate_cover(plan)?;
    if plan.operation == "metadata" {
        let edit = ogg::edit(&plan.output, &plan.metadata_patch)?;
        return Ok(ConversionReport {
            schema_version: 1,
            operation: "metadata".into(),
            output: plan.output.clone(),
            duration_ms: plan.book.duration_ms,
            size_bytes: fs::metadata(&plan.output)?.len(),
            elapsed_seconds: start.elapsed().as_secs_f64(),
            metadata: edit,
        });
    }
    if plan.output.exists() && !plan.overwrite {
        return Err(fail(
            "output_exists",
            format!(
                "Output exists: {}. Choose another path or use --overwrite.",
                plan.output.display()
            ),
        ));
    }
    let parent = plan.output.parent().context("Missing output directory")?;
    let temp = tempfile::Builder::new()
        .prefix(".opusab-")
        .tempdir_in(parent)?;
    let raw = temp.path().join("encoded.opus");
    let mut final_metadata = plan.book.metadata.clone();
    let encoded = if plan.operation == "copy" {
        plan.book.sources[0].path.clone()
    } else {
        // Resolve dependencies before doing work.
        backend::find("freaccmd")?;
        let channels = match plan.channels {
            Channels::Keep => None,
            Channels::Mono => Some(1),
            Channels::Stereo => Some(2),
        };
        let mut inputs = Vec::new();
        for (i, source) in plan.book.sources.iter().enumerate() {
            if let Some(n) = channels.filter(|n| *n != source.channels) {
                let flac = temp.path().join(format!("{i:05}.flac"));
                progress(backend::event(
                    "preparing_channels",
                    format!("Preparing {}", source.path.display()),
                    start.elapsed().as_secs_f64(),
                ));
                backend::prepare_channels(&source.path, &flac, n, progress)?;
                inputs.push(flac);
            } else {
                inputs.push(source.path.clone());
            }
        }
        progress(backend::event(
            "encoding",
            format!(
                "Encoding with {} workers at {} kbit/s VBR",
                plan.jobs, plan.target_bitrate_kbps
            ),
            start.elapsed().as_secs_f64(),
        ));
        backend::encode(&inputs, &raw, plan.target_bitrate_kbps, plan.jobs, progress)?;
        // Keep our millisecond chapter timings: fre:ac 1.1.7 rounds each track
        // duration to whole seconds when constructing its chapter tags.
        let mut f = File::open(&raw)?;
        let h = ogg::read_header(&mut f)?;
        if channels.is_some()
            && final_metadata.cover.is_none()
            && !h.has_cover()
            && plan.book.sources[0].has_embedded_cover
            && !plan.metadata_patch.remove_cover
        {
            let cover = temp.path().join("cover.jpg");
            let mut command = std::process::Command::new(backend::find("ffmpeg")?);
            command
                .args(["-v", "error", "-nostdin", "-i"])
                .arg(&plan.book.sources[0].path)
                .args(["-map", "0:v:0", "-frames:v", "1"])
                .arg(&cover);
            backend::run(&mut command, "cover", progress)?;
            final_metadata.cover = Some(cover);
        }
        raw
    };
    let mut patch = if plan.operation == "copy" {
        plan.metadata_patch.clone()
    } else {
        MetadataPatch::from(&final_metadata)
    };
    patch.remove_cover = plan.metadata_patch.remove_cover;
    patch.remove_tags = plan.metadata_patch.remove_tags.clone();
    if patch.remove_cover {
        patch.cover = None;
    }
    progress(backend::event(
        "finalizing",
        "Writing metadata and reserving space for future edits",
        start.elapsed().as_secs_f64(),
    ));
    let report = ogg::rewrite(
        &encoded,
        &plan.output,
        &patch,
        plan.padding_bytes,
        plan.overwrite,
    )?;
    let mut output = File::open(&plan.output)?;
    let h = ogg::read_header(&mut output)?;
    let duration = ogg::duration_ms(&mut output, &h)?;
    Ok(ConversionReport {
        schema_version: 1,
        operation: plan.operation.clone(),
        output: plan.output.clone(),
        duration_ms: duration,
        size_bytes: output.metadata()?.len(),
        elapsed_seconds: start.elapsed().as_secs_f64(),
        metadata: report,
    })
}
fn validate_cover(plan: &Plan) -> Result<()> {
    if let Some(cover) = &plan.book.metadata.cover {
        // Catch missing, oversized, or invalid artwork before starting a long encode.
        ogg::encode_picture(cover)?;
    }
    Ok(())
}

use crate::{
    backend,
    error::fail,
    model::{Book, Chapter, Source},
    ogg,
};
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs::{self, File},
    path::{Path, PathBuf},
    process::Command,
};

pub fn supported(path: &Path) -> bool {
    path.extension().is_some_and(|e| {
        matches!(
            e.to_string_lossy().to_ascii_lowercase().as_str(),
            "mp3"
                | "m4b"
                | "m4a"
                | "mp4"
                | "opus"
                | "ogg"
                | "flac"
                | "wav"
                | "aif"
                | "aiff"
                | "aac"
        )
    })
}
pub fn book(paths: &[PathBuf]) -> Result<Book> {
    ensure!(!paths.is_empty(), "Provide a file or folder");
    let mut sources = Vec::new();
    let mut warnings = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut cover = None;
    for path in paths {
        let path = path
            .canonicalize()
            .with_context(|| format!("Cannot open {}", path.display()))?;
        let directory = path.is_dir();
        let mut files = Vec::new();
        if directory {
            for entry in walkdir::WalkDir::new(&path).follow_links(false) {
                let entry = entry?;
                if entry.file_type().is_file() && supported(entry.path()) {
                    files.push(entry.path().to_path_buf());
                }
            }
            files.sort_by(|a, b| natord::compare(&a.to_string_lossy(), &b.to_string_lossy()));
            if cover.is_none() {
                cover = folder_cover(&path)?;
            }
        } else {
            files.push(path.clone());
        }
        ensure!(
            !files.is_empty(),
            "No supported audio files found in {}",
            path.display()
        );
        let mut group = Vec::new();
        for file in files {
            if backend::is_cancelled() {
                return Err(crate::error::cancelled());
            }
            ensure!(
                seen.insert(file.clone()),
                "Duplicate source: {}",
                file.display()
            );
            group.push(inspect(&file)?);
        }
        if directory && group.iter().all(|s| s.track.is_some()) {
            group.sort_by(|a, b| {
                (a.disc.unwrap_or(1), a.track)
                    .cmp(&(b.disc.unwrap_or(1), b.track))
                    .then_with(|| {
                        natord::compare(&a.path.to_string_lossy(), &b.path.to_string_lossy())
                    })
            });
            if group
                .windows(2)
                .any(|w| (w[0].disc, w[0].track) == (w[1].disc, w[1].track))
            {
                warnings.push(
                    "Duplicate disc/track numbers; filenames break ties. Review file order.".into(),
                );
            }
        } else if directory && group.len() > 1 {
            warnings.push("Some track numbers are missing; files use natural filename order. Review before converting.".into());
        }
        sources.extend(group);
    }
    let mut metadata = sources[0].metadata.clone();
    if sources.len() > 1 {
        // Probe stores ALBUM in a reserved per-source hint; it is removed before output.
        if let Some(album) = metadata
            .extra
            .remove("OPUSAB_SOURCE_ALBUM")
            .and_then(|v| v.into_iter().next())
        {
            metadata.title = album;
        }
        metadata.extra.remove("TRACKNUMBER");
        metadata.extra.remove("DISCNUMBER");
    }
    metadata.extra.remove("OPUSAB_SOURCE_ALBUM");
    metadata.cover = cover;
    if metadata.title.is_empty() {
        metadata.title = paths[0]
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .replace('_', " ");
    }
    let input_directory = if paths.len() == 1 && paths[0].is_dir() {
        Some(paths[0].canonicalize()?)
    } else {
        None
    };
    let mut result = Book {
        schema_version: 1,
        input_directory,
        sources,
        metadata,
        duration_ms: 0,
        warnings,
    };
    result.rebuild_chapters();
    if result.sources.iter().any(|s| s.channels > 2) {
        result.warnings.push(
            "Multichannel source: choose --channels mono or stereo for phone compatibility.".into(),
        );
    }
    Ok(result)
}
fn folder_cover(path: &Path) -> Result<Option<PathBuf>> {
    let mut files: Vec<_> = fs::read_dir(path)?
        .filter_map(|e| e.ok().map(|v| v.path()))
        .filter(|p| {
            p.extension().is_some_and(|e| {
                matches!(
                    e.to_string_lossy().to_ascii_lowercase().as_str(),
                    "jpg" | "jpeg" | "png"
                )
            })
        })
        .collect();
    files.sort_by_key(|p| {
        let n = p
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        (
            if n == "cover" {
                0
            } else if n == "folder" {
                1
            } else if n.contains("front") {
                2
            } else {
                3
            },
            n,
        )
    });
    Ok(files.into_iter().next())
}
fn numeric(v: &Value) -> f64 {
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0.0)
}
fn tag_number(tags: &BTreeMap<String, Vec<String>>, keys: &[&str]) -> Option<u32> {
    keys.iter().find_map(|k| {
        tags.get(*k)
            .and_then(|vs| vs.first())
            .and_then(|v| v.split('/').next()?.parse().ok())
    })
}
pub fn inspect(path: &Path) -> Result<Source> {
    let mut file = File::open(path)?;
    let mut signature = [0u8; 4];
    use std::io::Read;
    let _ = file.read(&mut signature)?;
    if &signature == b"OggS"
        && let Ok(h) = ogg::read_header(&mut file)
    {
        ogg::check_journal(path)?;
        let duration_ms = ogg::duration_ms(&mut file, &h)?;
        let size_bytes = file.metadata()?.len();
        return Ok(Source {
            path: path.to_path_buf(),
            codec: "opus".into(),
            duration_ms,
            sample_rate: 48000,
            channels: h.channels,
            bitrate_kbps: size_bytes
                .saturating_sub(h.end)
                .saturating_mul(8)
                .checked_div(duration_ms)
                .unwrap_or(0) as u32,
            size_bytes,
            disc: None,
            track: None,
            metadata: h.metadata(),
            has_embedded_cover: h.has_cover(),
        });
    }
    let output = Command::new(backend::find("ffprobe")?)
        .args([
            "-v",
            "error",
            "-show_format",
            "-show_streams",
            "-show_chapters",
            "-of",
            "json",
        ])
        .arg(path)
        .output()?;
    if !output.status.success() {
        return Err(fail(
            "probe_failed",
            format!(
                "Cannot inspect {}: {}",
                path.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        ));
    }
    let v: Value = serde_json::from_slice(&output.stdout)?;
    let streams = v["streams"].as_array().context("No streams")?;
    let audio = streams
        .iter()
        .find(|s| s["codec_type"] == "audio")
        .context("No audio stream in input")?;
    let duration = numeric(&audio["duration"]).max(numeric(&v["format"]["duration"]));
    ensure!(
        duration.is_finite() && duration > 0.0,
        "Cannot determine duration of {}",
        path.display()
    );
    let mut tags: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for t in [&audio["tags"], &v["format"]["tags"]] {
        if let Some(t) = t.as_object() {
            for (k, v) in t {
                if let Some(v) = v.as_str() {
                    tags.insert(k.to_ascii_uppercase(), vec![v.into()]);
                }
            }
        }
    }
    let track = tag_number(&tags, &["TRACK", "TRACKNUMBER"]);
    let disc = tag_number(&tags, &["DISC", "DISCNUMBER"]);
    let mut metadata = ogg::metadata_from_tags(&tags);
    if let Some(album) = tags.get("ALBUM") {
        metadata
            .extra
            .insert("OPUSAB_SOURCE_ALBUM".into(), album.clone());
    }
    for k in [
        "MAJOR_BRAND",
        "MINOR_VERSION",
        "COMPATIBLE_BRANDS",
        "ENCODER",
        "ENCODING_TOOL",
        "HANDLER_NAME",
        "VENDOR_ID",
        "TLEN",
        "DURATION",
        "TRACK",
        "DISC",
        "LANGUAGE",
    ] {
        metadata.extra.remove(k);
    }
    if let Some(chapters) = v["chapters"].as_array() {
        metadata.chapters = chapters
            .iter()
            .map(|c| Chapter {
                start_ms: (numeric(&c["start_time"]) * 1000.0).round() as u64,
                title: c["tags"]["title"].as_str().unwrap_or("").into(),
            })
            .collect();
    }
    Ok(Source {
        path: path.to_path_buf(),
        codec: audio["codec_name"].as_str().unwrap_or("unknown").into(),
        duration_ms: (duration * 1000.0).round() as u64,
        sample_rate: numeric(&audio["sample_rate"]) as u32,
        channels: numeric(&audio["channels"]) as u32,
        bitrate_kbps: (numeric(&audio["bit_rate"]) / 1000.0).round() as u32,
        size_bytes: fs::metadata(path)?.len(),
        disc,
        track,
        metadata,
        has_embedded_cover: streams
            .iter()
            .any(|s| s["disposition"]["attached_pic"] == 1),
    })
}

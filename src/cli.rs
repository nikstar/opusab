use crate::{
    app::{self, ConversionOptions},
    backend,
    error::fail,
    model::{self, MetadataPatch},
    ogg, probe,
};
use anyhow::{Result, ensure};
use clap::{Args, CommandFactory, Parser, Subcommand};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{self, IsTerminal},
    path::PathBuf,
};

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Prepare, convert, and edit audiobooks",
    long_about = "An audiobook CLI and terminal editor. Pass a file or folder to open the TUI; use explicit commands for scripts and agents."
)]
pub struct Cli {
    /// Emit versioned JSON results and structured errors.
    #[arg(long, global = true)]
    pub json: bool,
    /// Suppress progress (results and errors are still printed).
    #[arg(short, long, global = true)]
    pub quiet: bool,
    #[command(subcommand)]
    pub command: Option<Commands>,
    /// File or folder to open in the interactive editor.
    pub source: Option<PathBuf>,
}
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Inspect sources, tags, chapters, and inferred file order.
    Inspect {
        #[arg(required = true)]
        sources: Vec<PathBuf>,
    },
    /// Preview the operation, settings, metadata, and output without writing.
    Plan {
        #[arg(required = true)]
        sources: Vec<PathBuf>,
        #[command(flatten)]
        options: ConversionOptions,
    },
    /// Convert or update a book without prompts. Existing Opus audio is kept by default.
    Convert {
        #[arg(required = true)]
        sources: Vec<PathBuf>,
        #[command(flatten)]
        options: ConversionOptions,
    },
    /// Open the interactive editor.
    Edit { source: PathBuf },
    /// Read or edit Opus tags without re-encoding audio.
    Tags {
        #[command(subcommand)]
        command: TagCommands,
    },
    /// Locate the bundled or installed audio tools.
    Doctor,
    /// Generate shell completions.
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}
#[derive(Subcommand, Debug)]
pub enum TagCommands {
    /// Show editable metadata and available header space.
    Show { file: PathBuf },
    /// Update selected fields. All edits are in place; insufficient capacity is an error.
    Set {
        file: PathBuf,
        #[command(flatten)]
        fields: Box<TagFields>,
    },
    /// Apply a JSON patch. Missing fields are preserved; empty strings clear fields.
    Apply {
        file: PathBuf,
        #[arg(long)]
        from: PathBuf,
        #[arg(long)]
        in_place: bool,
    },
    /// Repack once to reserve extra metadata space. Audio packets are preserved.
    Reserve {
        file: PathBuf,
        #[arg(long,default_value_t=256,value_parser=clap::value_parser!(u32).range(0..=8192))]
        padding_kib: u32,
    },
    /// Roll back an interrupted metadata update using its recovery journal.
    Recover { file: PathBuf },
    /// Open the metadata TUI for an Opus file.
    Edit { file: PathBuf },
}
#[derive(Args, Debug, Default)]
pub struct TagFields {
    #[arg(long)]
    title: Option<String>,
    #[arg(long)]
    author: Option<String>,
    #[arg(long)]
    narrator: Option<String>,
    #[arg(long)]
    series: Option<String>,
    #[arg(long)]
    series_part: Option<String>,
    #[arg(long)]
    date: Option<String>,
    #[arg(long)]
    genre: Option<String>,
    #[arg(long)]
    description: Option<String>,
    #[arg(long, conflicts_with = "remove_cover")]
    cover: Option<PathBuf>,
    #[arg(long)]
    remove_cover: bool,
    /// Set an additional tag (repeat for multiple values of the same key).
    #[arg(long = "tag", value_name = "KEY=VALUE")]
    tags: Vec<String>,
    #[arg(long = "remove-tag", value_name = "KEY")]
    remove_tags: Vec<String>,
}
impl TagFields {
    fn patch(self) -> Result<MetadataPatch> {
        let mut extra: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for tag in self.tags {
            let (k, v) = tag
                .split_once('=')
                .ok_or_else(|| fail("invalid_tag", "Use --tag KEY=VALUE"))?;
            extra.entry(k.to_uppercase()).or_default().push(v.into());
        }
        Ok(MetadataPatch {
            title: self.title,
            author: self.author,
            narrator: self.narrator,
            series: self.series,
            series_part: self.series_part,
            date: self.date,
            genre: self.genre,
            description: self.description,
            cover: self.cover,
            remove_cover: self.remove_cover,
            chapters: None,
            extra: Some(extra),
            remove_tags: self.remove_tags,
        })
    }
}
pub fn run(cli: Cli) -> Result<()> {
    if let Some(source) = cli.source {
        ensure!(
            cli.command.is_none(),
            "Choose a source or an explicit command"
        );
        return open_tui(source);
    }
    let Some(command) = cli.command else {
        Cli::command().print_help()?;
        println!();
        return Ok(());
    };
    match command {
        Commands::Inspect { sources } => {
            let book = probe::book(&sources)?;
            if cli.json {
                print_json(&book)?;
            } else {
                print_book(&book);
            }
        }
        Commands::Plan { sources, options } => {
            let plan = app::make_plan(&sources, &options)?;
            if cli.json {
                print_json(&plan)?;
            } else {
                print_plan(&plan);
            }
        }
        Commands::Convert { sources, options } => {
            let plan = app::make_plan(&sources, &options)?;
            let mut progress = |p: backend::Progress| {
                if !cli.quiet {
                    if cli.json {
                        if let Ok(s) = serde_json::to_string(&p) {
                            eprintln!("{s}");
                        }
                    } else {
                        eprintln!("{} · {} ({:.0}s)", p.stage, p.message, p.elapsed_seconds);
                    }
                }
            };
            let report = app::execute(&plan, &mut progress)?;
            if cli.json {
                print_json(&report)?;
            } else {
                println!(
                    "{}\n{} · {:.1} MiB · {:.1}s",
                    report.output.display(),
                    model::time_string(report.duration_ms),
                    report.size_bytes as f64 / 1048576.0,
                    report.elapsed_seconds
                );
            }
        }
        Commands::Edit { source } => open_tui(source)?,
        Commands::Tags { command } => match command {
            TagCommands::Show { file } => {
                let file = file.canonicalize()?;
                ogg::check_journal(&file)?;
                let mut f = File::open(&file)?;
                let h = ogg::read_header(&mut f)?;
                let value = serde_json::json!({"schema_version":1,"metadata":h.metadata(),"has_embedded_cover":h.has_cover(),"capacity_bytes":h.capacity,"available_bytes":h.capacity-h.used});
                if cli.json {
                    print_json(&value)?;
                } else {
                    let m = h.metadata();
                    println!(
                        "{}\nAuthor: {}\nNarrator: {}\nChapters: {}\nCover: {}\nMetadata space: {} / {} bytes free",
                        m.title,
                        m.author,
                        m.narrator,
                        m.chapters.len(),
                        h.has_cover(),
                        h.capacity - h.used,
                        h.capacity
                    );
                    for c in m.chapters {
                        println!("  {}  {}", model::time_string(c.start_ms), c.title);
                    }
                }
            }
            TagCommands::Set { file, fields } => {
                edit_result(ogg::edit(&file, &fields.patch()?)?, cli.json)?
            }
            TagCommands::Apply { file, from, .. } => {
                edit_result(ogg::edit(&file, &app::read_patch(&from)?)?, cli.json)?
            }
            TagCommands::Reserve { file, padding_kib } => {
                let file = file.canonicalize()?;
                edit_result(
                    ogg::rewrite(
                        &file,
                        &file,
                        &MetadataPatch::default(),
                        padding_kib as usize * 1024,
                        true,
                    )?,
                    cli.json,
                )?;
            }
            TagCommands::Recover { file } => edit_result(ogg::recover(&file)?, cli.json)?,
            TagCommands::Edit { file } => open_tui(file)?,
        },
        Commands::Doctor => {
            let tools = backend::doctor();
            let ready = tools.iter().take(2).all(|t| t.available);
            if cli.json {
                print_json(&serde_json::json!({"schema_version":1,"ready":ready,"tools":tools}))?;
            } else {
                for t in tools {
                    println!(
                        "{}: {}",
                        t.name,
                        t.path
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "missing".into())
                    );
                }
                println!("Metadata editing of Opus files needs no external tools.");
            }
            if !ready {
                return Err(fail(
                    "backend_missing",
                    "Conversion requires freaccmd and ffprobe",
                ));
            }
        }
        Commands::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "opusab", &mut io::stdout())
        }
    }
    Ok(())
}
fn open_tui(source: PathBuf) -> Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(fail(
            "terminal_required",
            "The editor requires a terminal. Use inspect, plan, convert, or tags commands for automation.",
        ));
    }
    crate::tui::run(source)
}
pub fn print_json(value: &impl serde::Serialize) -> Result<()> {
    serde_json::to_writer_pretty(io::stdout().lock(), value)?;
    println!();
    Ok(())
}
fn edit_result(report: ogg::EditReport, json: bool) -> Result<()> {
    if json {
        print_json(&serde_json::json!({"schema_version":1,"result":report}))?;
    } else {
        println!(
            "{}: wrote {} metadata bytes across {} pages; {} audio bytes written. {} bytes available.",
            report.mode,
            report.bytes_written,
            report.pages_written,
            report.audio_bytes_written,
            report.available_bytes
        );
    }
    Ok(())
}
pub fn print_book(book: &model::Book) {
    println!(
        "{}\n{}{}\n{} · {} source file(s) · {} chapters",
        book.metadata.title,
        book.metadata.author,
        if book.metadata.narrator.is_empty() {
            String::new()
        } else {
            format!(" · read by {}", book.metadata.narrator)
        },
        model::time_string(book.duration_ms),
        book.sources.len(),
        book.metadata.chapters.len()
    );
    for (i, s) in book.sources.iter().enumerate() {
        println!(
            "  {:>2}. {} · {} kbit/s · {} ch · {}\n      {}",
            i + 1,
            s.codec,
            s.bitrate_kbps,
            s.channels,
            model::time_string(s.duration_ms),
            s.path.display()
        );
    }
    for warning in &book.warnings {
        println!("Note: {warning}");
    }
}
pub fn print_plan(plan: &app::Plan) {
    print_book(&plan.book);
    println!(
        "\nOperation: {}\nOutput: {}\nTarget: {} kbit/s VBR · {} workers\nSuggestion: {}\nEstimated audio size: {:.1} MiB",
        plan.operation,
        plan.output.display(),
        plan.target_bitrate_kbps,
        plan.jobs,
        plan.recommendation.reason,
        plan.recommendation.estimated_bytes as f64 / 1048576.0
    );
}

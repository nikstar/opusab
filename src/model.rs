use crate::error::fail;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Chapter {
    pub start_ms: u64,
    pub title: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Metadata {
    pub title: String,
    pub author: String,
    pub narrator: String,
    pub series: String,
    pub series_part: String,
    pub date: String,
    pub genre: String,
    pub description: String,
    /// External cover to embed. None preserves an existing embedded cover.
    pub cover: Option<PathBuf>,
    pub chapters: Vec<Chapter>,
    pub extra: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MetadataPatch {
    pub title: Option<String>,
    pub author: Option<String>,
    pub narrator: Option<String>,
    pub series: Option<String>,
    pub series_part: Option<String>,
    pub date: Option<String>,
    pub genre: Option<String>,
    pub description: Option<String>,
    pub cover: Option<PathBuf>,
    pub remove_cover: bool,
    pub chapters: Option<Vec<Chapter>>,
    pub extra: Option<BTreeMap<String, Vec<String>>>,
    pub remove_tags: Vec<String>,
}
impl MetadataPatch {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !(self.cover.is_some() && self.remove_cover),
            "Cannot add and remove cover together"
        );
        for key in &self.remove_tags {
            ensure!(!is_managed(key), "Use the dedicated field to clear {key}");
        }
        Ok(())
    }
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.author.is_none()
            && self.narrator.is_none()
            && self.series.is_none()
            && self.series_part.is_none()
            && self.date.is_none()
            && self.genre.is_none()
            && self.description.is_none()
            && self.cover.is_none()
            && !self.remove_cover
            && self.chapters.is_none()
            && self.extra.as_ref().is_none_or(|v| v.is_empty())
            && self.remove_tags.is_empty()
    }
    pub fn apply(&self, m: &mut Metadata) {
        macro_rules! field { ($($f:ident),*) => { $(if let Some(v) = &self.$f { m.$f = v.clone(); })* }; }
        field!(
            title,
            author,
            narrator,
            series,
            series_part,
            date,
            genre,
            description,
            chapters
        );
        if let Some(v) = &self.cover {
            m.cover = Some(v.clone());
        }
        if self.remove_cover {
            m.cover = None;
        }
        if let Some(extra) = &self.extra {
            for (k, v) in extra {
                m.extra.insert(k.to_uppercase(), v.clone());
            }
        }
        for key in &self.remove_tags {
            m.extra.remove(&key.to_uppercase());
        }
    }
}
impl From<&Metadata> for MetadataPatch {
    fn from(m: &Metadata) -> Self {
        Self {
            title: Some(m.title.clone()),
            author: Some(m.author.clone()),
            narrator: Some(m.narrator.clone()),
            series: Some(m.series.clone()),
            series_part: Some(m.series_part.clone()),
            date: Some(m.date.clone()),
            genre: Some(m.genre.clone()),
            description: Some(m.description.clone()),
            cover: m.cover.clone(),
            chapters: Some(m.chapters.clone()),
            extra: Some(m.extra.clone()),
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Source {
    pub path: PathBuf,
    pub codec: String,
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub channels: u32,
    pub bitrate_kbps: u32,
    pub size_bytes: u64,
    pub disc: Option<u32>,
    pub track: Option<u32>,
    pub metadata: Metadata,
    pub has_embedded_cover: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Book {
    pub schema_version: u32,
    pub input_directory: Option<PathBuf>,
    pub sources: Vec<Source>,
    pub metadata: Metadata,
    pub duration_ms: u64,
    pub warnings: Vec<String>,
}
impl Book {
    pub fn rebuild_chapters(&mut self) {
        let mut elapsed = 0;
        let mut chapters = Vec::new();
        let multiple = self.sources.len() > 1;
        let same_titles = self
            .sources
            .windows(2)
            .all(|w| w[0].metadata.title == w[1].metadata.title);
        for s in &self.sources {
            if s.metadata.chapters.is_empty() {
                if multiple {
                    let title = if s.metadata.title.is_empty() || same_titles {
                        s.path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned()
                    } else {
                        s.metadata.title.clone()
                    };
                    chapters.push(Chapter {
                        start_ms: elapsed,
                        title,
                    });
                }
            } else {
                chapters.extend(s.metadata.chapters.iter().map(|c| Chapter {
                    start_ms: elapsed + c.start_ms,
                    title: c.title.clone(),
                }));
            }
            elapsed += s.duration_ms;
        }
        self.metadata.chapters = chapters;
        self.duration_ms = elapsed;
    }
}

#[derive(Clone, Copy, Debug, Default, clap::ValueEnum, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    #[default]
    Speech,
    Compact,
    Mixed,
}
#[derive(Clone, Copy, Debug, Default, clap::ValueEnum, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Channels {
    #[default]
    Keep,
    Mono,
    Stereo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recommendation {
    pub bitrate_kbps: u32,
    pub reason: String,
    pub estimated_bytes: u64,
}
pub fn recommend(book: &Book, preset: Preset) -> Recommendation {
    let low_rate = book.sources.iter().all(|s| s.sample_rate <= 24000);
    let low_mp3 = book
        .sources
        .iter()
        .all(|s| s.codec == "mp3" && s.bitrate_kbps > 0 && s.bitrate_kbps <= 64);
    let (bitrate, reason) = match preset {
        Preset::Compact => (
            24,
            "Compact narration preset; prioritize size over fidelity.",
        ),
        Preset::Mixed => (
            64,
            "Mixed speech/music preset; retain more detail for music and stereo effects.",
        ),
        Preset::Speech if low_mp3 || low_rate => (
            24,
            "Lower-bandwidth source; suggest 24 kbit/s VBR for narration. Preview before choosing.",
        ),
        Preset::Speech => (
            32,
            "Narration preset: 32 kbit/s VBR. Source bitrate is a hint, not a quality equivalence.",
        ),
    };
    Recommendation {
        bitrate_kbps: bitrate,
        reason: reason.into(),
        estimated_bytes: book.duration_ms * bitrate as u64 / 8,
    }
}
pub fn validate_metadata(m: &Metadata, duration_ms: Option<u64>) -> Result<()> {
    ensure!(
        m.chapters.len() <= 1000,
        "At most 1000 chapters are supported"
    );
    let mut last = None;
    for c in &m.chapters {
        if last.is_some_and(|v| c.start_ms <= v) {
            return Err(fail(
                "invalid_chapters",
                "Chapter times must be strictly increasing",
            ));
        }
        if duration_ms.is_some_and(|v| c.start_ms >= v) {
            return Err(fail(
                "invalid_chapters",
                "Chapter starts beyond the end of the book",
            ));
        }
        last = Some(c.start_ms);
    }
    for k in m.extra.keys() {
        ensure!(
            !k.is_empty() && k.bytes().all(|v| (0x20..=0x7d).contains(&v) && v != b'='),
            "Invalid tag name: {k}"
        );
        ensure!(!is_managed(k), "Use the dedicated field for {k}");
    }
    Ok(())
}
pub fn is_managed(key: &str) -> bool {
    let k = key.to_ascii_uppercase();
    matches!(
        k.as_str(),
        "TITLE"
            | "ALBUM"
            | "ARTIST"
            | "AUTHOR"
            | "ALBUMARTIST"
            | "ALBUM_ARTIST"
            | "NARRATOR"
            | "COMPOSER"
            | "PERFORMER"
            | "SERIES"
            | "SERIES-PART"
            | "SERIES_PART"
            | "PART"
            | "DATE"
            | "YEAR"
            | "GENRE"
            | "DESCRIPTION"
            | "COMMENT"
            | "METADATA_BLOCK_PICTURE"
    ) || k.starts_with("CHAPTER")
}
pub fn time_string(ms: u64) -> String {
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}
pub fn parse_time(s: &str) -> Result<u64> {
    let parts: Vec<_> = s.trim().split(':').collect();
    ensure!(
        (1..=3).contains(&parts.len()),
        "Use HH:MM:SS.mmm or seconds"
    );
    let mut secs = 0.0;
    for p in parts {
        let v: f64 = p.parse()?;
        ensure!(v.is_finite() && v >= 0.0, "Invalid timestamp");
        secs = secs * 60.0 + v;
    }
    ensure!(secs < 1e12, "Timestamp too large");
    Ok((secs * 1000.0).round() as u64)
}

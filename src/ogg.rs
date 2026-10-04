//! Ogg Opus metadata I/O. The fast path never scans or copies audio pages.
use crate::{
    error::fail,
    model::{self, Chapter, Metadata, MetadataPatch},
};
use anyhow::{Context, Result, ensure};
use base64::{Engine, engine::general_purpose::STANDARD as B64};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

pub const DEFAULT_PADDING: usize = 256 * 1024;
pub const MAX_HEADER: usize = 16 * 1024 * 1024;
const POLY: u32 = 0x04c11db7;

#[derive(Debug, Clone)]
pub struct Page {
    pub offset: u64,
    pub bytes: Vec<u8>,
    pub header_len: usize,
}
impl Page {
    pub fn serial(&self) -> u32 {
        u32::from_le_bytes(self.bytes[14..18].try_into().unwrap())
    }
    pub fn sequence(&self) -> u32 {
        u32::from_le_bytes(self.bytes[18..22].try_into().unwrap())
    }
    pub fn body(&self) -> &[u8] {
        &self.bytes[self.header_len..]
    }
    pub fn checksum(&mut self) {
        self.bytes[22..26].fill(0);
        let crc = crc(&self.bytes);
        self.bytes[22..26].copy_from_slice(&crc.to_le_bytes());
    }
}
pub fn crc(data: &[u8]) -> u32 {
    let mut result = 0u32;
    for &b in data {
        result ^= (b as u32) << 24;
        for _ in 0..8 {
            result = if result & 0x8000_0000 != 0 {
                (result << 1) ^ POLY
            } else {
                result << 1
            };
        }
    }
    result
}
pub fn read_page<R: Read + Seek>(r: &mut R) -> Result<Page> {
    let offset = r.stream_position()?;
    let mut header = [0u8; 27];
    r.read_exact(&mut header).context("Truncated Ogg page")?;
    ensure!(
        &header[..4] == b"OggS" && header[4] == 0,
        "Invalid Ogg page at {offset}"
    );
    let mut lacing = vec![0; header[26] as usize];
    r.read_exact(&mut lacing)?;
    let body_len: usize = lacing.iter().map(|v| *v as usize).sum();
    let header_len = 27 + lacing.len();
    let mut bytes = Vec::with_capacity(header_len + body_len);
    bytes.extend(header);
    bytes.extend(lacing);
    bytes.resize(header_len + body_len, 0);
    r.read_exact(&mut bytes[header_len..])?;
    let expected = u32::from_le_bytes(bytes[22..26].try_into().unwrap());
    bytes[22..26].fill(0);
    ensure!(crc(&bytes) == expected, "Ogg checksum mismatch at {offset}");
    bytes[22..26].copy_from_slice(&expected.to_le_bytes());
    Ok(Page {
        offset,
        bytes,
        header_len,
    })
}

#[derive(Debug, Clone)]
pub struct Header {
    pub head: Page,
    pub pages: Vec<Page>,
    pub vendor: Vec<u8>,
    pub comments: Vec<String>,
    pub opaque: Vec<u8>,
    pub capacity: usize,
    pub used: usize,
    pub end: u64,
    pub channels: u32,
    pub pre_skip: u16,
}
fn get_u32(packet: &[u8], at: &mut usize) -> Result<usize> {
    ensure!(*at + 4 <= packet.len(), "Truncated OpusTags length");
    let n = u32::from_le_bytes(packet[*at..*at + 4].try_into().unwrap()) as usize;
    *at += 4;
    Ok(n)
}
fn get_bytes(packet: &[u8], at: &mut usize) -> Result<Vec<u8>> {
    let n = get_u32(packet, at)?;
    ensure!(
        n <= packet.len().saturating_sub(*at),
        "Invalid OpusTags field length"
    );
    let v = packet[*at..*at + n].to_vec();
    *at += n;
    Ok(v)
}
pub fn read_header<R: Read + Seek>(r: &mut R) -> Result<Header> {
    r.seek(SeekFrom::Start(0))?;
    let head = read_page(r)?;
    ensure!(
        head.bytes[5] & 2 != 0
            && head.sequence() == 0
            && head.body().starts_with(b"OpusHead")
            && head.body().len() >= 19,
        "Expected an Ogg Opus file"
    );
    ensure!(head.body()[8] >> 4 == 0, "Unsupported Opus header version");
    let channels = head.body()[9] as u32;
    let pre_skip = u16::from_le_bytes(head.body()[10..12].try_into().unwrap());
    let mut pages = Vec::new();
    let mut packet = Vec::new();
    loop {
        let page = read_page(r)?;
        ensure!(
            page.serial() == head.serial() && page.sequence() == pages.len() as u32 + 1,
            "Unexpected Opus metadata stream/sequence"
        );
        ensure!(
            (page.bytes[5] & 1 != 0) == !pages.is_empty(),
            "Invalid metadata continuation flag"
        );
        ensure!(page.bytes[5] & 6 == 0, "Invalid metadata page flags");
        let laces = &page.bytes[27..page.header_len];
        ensure!(!laces.is_empty(), "Empty metadata page");
        ensure!(
            laces[..laces.len() - 1].iter().all(|v| *v == 255),
            "Metadata must occupy complete pages"
        );
        let done = *laces.last().unwrap() < 255;
        ensure!(
            packet.len() + page.body().len() <= MAX_HEADER,
            "Opus metadata exceeds 16 MiB limit"
        );
        packet.extend_from_slice(page.body());
        pages.push(page);
        if done {
            break;
        }
    }
    ensure!(packet.starts_with(b"OpusTags"), "Missing OpusTags packet");
    let capacity = packet.len();
    let mut at = 8;
    let vendor = get_bytes(&packet, &mut at)?;
    let count = get_u32(&packet, &mut at)?;
    ensure!(
        count <= packet.len().saturating_sub(at) / 4,
        "Invalid OpusTags comment count"
    );
    let mut comments = Vec::new();
    for _ in 0..count {
        comments.push(String::from_utf8(get_bytes(&packet, &mut at)?).context("Tag is not UTF-8")?);
    }
    let opaque = if packet.get(at).is_some_and(|b| b & 1 != 0) {
        packet[at..].to_vec()
    } else {
        Vec::new()
    };
    Ok(Header {
        head,
        pages,
        vendor,
        comments,
        used: at + opaque.len(),
        opaque,
        capacity,
        end: r.stream_position()?,
        channels,
        pre_skip,
    })
}
impl Header {
    pub fn tags(&self) -> BTreeMap<String, Vec<String>> {
        let mut tags: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for c in &self.comments {
            if let Some((k, v)) = c.split_once('=') {
                tags.entry(k.to_ascii_uppercase())
                    .or_default()
                    .push(v.into());
            }
        }
        tags
    }
    pub fn metadata(&self) -> Metadata {
        metadata_from_tags(&self.tags())
    }
    pub fn has_cover(&self) -> bool {
        self.comments.iter().any(|s| {
            s.to_ascii_uppercase()
                .starts_with("METADATA_BLOCK_PICTURE=")
        })
    }
    pub fn packet(&self, comments: &[String], capacity: usize) -> Result<Vec<u8>> {
        let mut b = b"OpusTags".to_vec();
        put_string(&mut b, &self.vendor);
        b.extend_from_slice(&(comments.len() as u32).to_le_bytes());
        for c in comments {
            put_string(&mut b, c.as_bytes());
        }
        b.extend_from_slice(&self.opaque);
        if b.len() > capacity {
            return Err(fail(
                "metadata_capacity_exceeded",
                format!(
                    "Metadata needs {} bytes; {} available. Run `opusab tags reserve FILE --padding-kib N` to grow the header without re-encoding.",
                    b.len(),
                    capacity
                ),
            ));
        }
        ensure!(capacity <= MAX_HEADER, "Metadata capacity exceeds 16 MiB");
        b.resize(capacity, 0);
        Ok(b)
    }
    pub fn patched_comments(
        &self,
        patch: &MetadataPatch,
        duration: Option<u64>,
    ) -> Result<Vec<String>> {
        if patch.is_empty() {
            return Ok(self.comments.clone());
        }
        let mut m = self.metadata();
        patch.validate()?;
        patch.apply(&mut m);
        model::validate_metadata(&m, duration)?;
        ensure!(
            !(patch.cover.is_some() && patch.remove_cover),
            "Cannot add and remove cover together"
        );
        let mut tags = self.tags();
        // Only fields explicitly edited are replaced; preserve aliases and unknown fields otherwise.
        macro_rules! set { ($field:ident, $key:expr, [$($alias:expr),*]) => {
            if let Some(value) = &patch.$field { $(tags.remove($alias);)* tags.remove($key); if !value.is_empty() { tags.insert($key.into(), vec![value.clone()]); } }
        }; }
        set!(title, "TITLE", []);
        set!(author, "ARTIST", ["AUTHOR", "ALBUMARTIST", "ALBUM_ARTIST"]);
        set!(narrator, "NARRATOR", ["COMPOSER", "PERFORMER"]);
        set!(series, "SERIES", []);
        set!(series_part, "SERIES-PART", ["SERIES_PART", "PART"]);
        set!(date, "DATE", ["YEAR"]);
        set!(genre, "GENRE", []);
        set!(description, "DESCRIPTION", ["COMMENT"]);
        if let Some(v) = &patch.title {
            tags.remove("ALBUM");
            if !v.is_empty() {
                tags.insert("ALBUM".into(), vec![v.clone()]);
            }
        }
        if let Some(chapters) = &patch.chapters {
            tags.retain(|k, _| !k.starts_with("CHAPTER"));
            for (i, c) in chapters.iter().enumerate() {
                tags.insert(
                    format!("CHAPTER{i:03}"),
                    vec![model::time_string(c.start_ms)],
                );
                tags.insert(format!("CHAPTER{i:03}NAME"), vec![c.title.clone()]);
            }
        }
        if let Some(extra) = &patch.extra {
            for (key, values) in extra {
                let key = key.to_uppercase();
                tags.remove(&key);
                if !values.is_empty() {
                    tags.insert(key, values.clone());
                }
            }
        }
        for key in &patch.remove_tags {
            ensure!(
                !model::is_managed(key),
                "Use the dedicated field to clear {key}"
            );
            tags.remove(&key.to_uppercase());
        }
        if patch.remove_cover {
            tags.remove("METADATA_BLOCK_PICTURE");
        }
        if let Some(path) = &patch.cover {
            tags.insert("METADATA_BLOCK_PICTURE".into(), vec![encode_picture(path)?]);
        }
        let pictures = tags.remove("METADATA_BLOCK_PICTURE");
        let mut comments: Vec<_> = tags
            .into_iter()
            .flat_map(|(k, vs)| vs.into_iter().map(move |v| format!("{k}={v}")))
            .collect();
        if let Some(vs) = pictures {
            comments.extend(
                vs.into_iter()
                    .map(|v| format!("METADATA_BLOCK_PICTURE={v}")),
            );
        }
        // Comments without '=' are legal to retain, although not editable through the key/value API.
        comments.extend(self.comments.iter().filter(|v| !v.contains('=')).cloned());
        Ok(comments)
    }
}
fn put_string(b: &mut Vec<u8>, s: &[u8]) {
    b.extend_from_slice(&(s.len() as u32).to_le_bytes());
    b.extend_from_slice(s);
}
pub fn metadata_from_tags(tags: &BTreeMap<String, Vec<String>>) -> Metadata {
    let value = |names: &[&str]| -> String {
        names
            .iter()
            .find_map(|k| tags.get(*k).and_then(|v| v.first()).cloned())
            .unwrap_or_default()
    };
    let mut chapters = Vec::new();
    for (k, vs) in tags {
        if let Some(n) = k.strip_prefix("CHAPTER")
            && n.len() == 3
            && n.bytes().all(|c| c.is_ascii_digit())
            && let Some(t) = vs.first().and_then(|s| model::parse_time(s).ok())
        {
            chapters.push(Chapter {
                start_ms: t,
                title: value(&[&format!("{k}NAME")]),
            });
        }
    }
    chapters.sort_by_key(|c| c.start_ms);
    Metadata {
        title: value(&["TITLE", "ALBUM"]),
        author: value(&["ARTIST", "AUTHOR", "ALBUMARTIST", "ALBUM_ARTIST"]),
        narrator: value(&["NARRATOR", "COMPOSER", "PERFORMER"]),
        series: value(&["SERIES"]),
        series_part: value(&["SERIES-PART", "SERIES_PART", "PART"]),
        date: value(&["DATE", "YEAR"]),
        genre: value(&["GENRE"]),
        description: value(&["DESCRIPTION", "COMMENT"]),
        cover: None,
        chapters,
        extra: tags
            .iter()
            .filter(|(k, _)| !model::is_managed(k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    }
}
pub(crate) fn encode_picture(path: &Path) -> Result<String> {
    ensure!(
        fs::metadata(path)?.len() <= 8 * 1024 * 1024,
        "Cover must be at most 8 MiB"
    );
    let data = fs::read(path)?;
    let format = image::guess_format(&data)?;
    let mime = match format {
        image::ImageFormat::Png => "image/png",
        image::ImageFormat::Jpeg => "image/jpeg",
        _ => anyhow::bail!("Cover must be JPEG or PNG"),
    };
    let (w, h) = image::ImageReader::new(std::io::Cursor::new(&data))
        .with_guessed_format()?
        .into_dimensions()?;
    let mut b = Vec::new();
    b.extend_from_slice(&3u32.to_be_bytes());
    b.extend_from_slice(&(mime.len() as u32).to_be_bytes());
    b.extend_from_slice(mime.as_bytes());
    b.extend_from_slice(&0u32.to_be_bytes());
    for n in [w, h, 24, 0, data.len() as u32] {
        b.extend_from_slice(&n.to_be_bytes());
    }
    b.extend(data);
    Ok(B64.encode(b))
}
pub fn duration_ms(file: &mut File, header: &Header) -> Result<u64> {
    let len = file.metadata()?.len();
    let start = len.saturating_sub(131072);
    file.seek(SeekFrom::Start(start))?;
    let mut tail = Vec::new();
    file.read_to_end(&mut tail)?;
    for pos in (0..tail.len().saturating_sub(26)).rev() {
        if &tail[pos..pos + 4] != b"OggS" {
            continue;
        }
        let mut r = std::io::Cursor::new(&tail);
        r.set_position(pos as u64);
        if let Ok(page) = read_page(&mut r) {
            if page.bytes[5] & 4 == 0 {
                continue;
            }
            ensure!(
                page.serial() == header.head.serial(),
                "Chained Opus files are not supported; convert to one logical stream first"
            );
            let granule = u64::from_le_bytes(page.bytes[6..14].try_into().unwrap());
            ensure!(
                granule != u64::MAX && granule >= header.pre_skip as u64,
                "Invalid end granule"
            );
            return Ok((granule - header.pre_skip as u64) * 1000 / 48000);
        }
    }
    Err(fail("invalid_opus", "No valid final Ogg page found"))
}

pub fn journal_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".opusab-journal");
    path.with_file_name(name)
}
pub fn check_journal(path: &Path) -> Result<()> {
    if journal_path(path).exists() {
        return Err(fail(
            "recovery_required",
            format!(
                "An interrupted metadata edit exists. Run `opusab tags recover {:?}` first.",
                path
            ),
        ));
    }
    Ok(())
}
#[derive(Debug, Serialize, Deserialize)]
struct JournalPage {
    offset: u64,
    before: String,
    after: String,
}
#[derive(Debug, Serialize, Deserialize)]
struct Journal {
    version: u32,
    file_len: u64,
    identity: String,
    head_hash: String,
    header_end: u64,
    pages: Vec<JournalPage>,
}
fn identity(file: &File) -> Result<String> {
    let m = file.metadata()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(format!("{}:{}", m.dev(), m.ino()))
    }
    #[cfg(not(unix))]
    {
        Ok(format!("{:?}", m.created()?))
    }
}
fn check_identity(path: &Path, file: &File) -> Result<()> {
    ensure!(
        identity(&File::open(path)?)? == identity(file)?,
        "The file was replaced by another process; retry the operation"
    );
    Ok(())
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn sync_parent(_path: &Path) -> Result<()> {
    #[cfg(unix)]
    File::open(_path.parent().unwrap_or(Path::new(".")))?.sync_all()?;
    Ok(())
}
#[derive(Debug, Serialize, Deserialize)]
pub struct EditReport {
    pub mode: String,
    pub pages_written: usize,
    pub bytes_written: u64,
    pub audio_bytes_written: u64,
    pub capacity_bytes: usize,
    pub available_bytes: usize,
}

pub fn edit(path: &Path, patch: &MetadataPatch) -> Result<EditReport> {
    let path = path.canonicalize()?;
    let mut file = OpenOptions::new().read(true).write(true).open(&path)?;
    file.try_lock_exclusive().map_err(|_| {
        fail(
            "file_busy",
            "The audiobook is being edited by another process",
        )
    })?;
    check_journal(&path)?;
    check_identity(&path, &file)?;
    let header = read_header(&mut file)?;
    let duration = duration_ms(&mut file, &header)?;
    let comments = header.patched_comments(patch, Some(duration))?;
    let packet = header.packet(&comments, header.capacity)?;
    let used = 8
        + 4
        + header.vendor.len()
        + 4
        + comments.iter().map(|s| 4 + s.len()).sum::<usize>()
        + header.opaque.len();
    let mut at = 0;
    let mut changes = Vec::new();
    for page in &header.pages {
        let mut new = page.clone();
        let n = page.body().len();
        new.bytes[page.header_len..].copy_from_slice(&packet[at..at + n]);
        at += n;
        new.checksum();
        if new.bytes != page.bytes {
            changes.push((page.clone(), new));
        }
    }
    let mut report = EditReport {
        mode: if changes.is_empty() {
            "unchanged"
        } else {
            "in_place"
        }
        .into(),
        pages_written: changes.len(),
        bytes_written: 0,
        audio_bytes_written: 0,
        capacity_bytes: header.capacity,
        available_bytes: header.capacity - used,
    };
    if changes.is_empty() {
        return Ok(report);
    }
    let journal = Journal {
        version: 1,
        file_len: file.metadata()?.len(),
        identity: identity(&file)?,
        head_hash: hash(&header.head.bytes),
        header_end: header.end,
        pages: changes
            .iter()
            .map(|(a, b)| JournalPage {
                offset: a.offset,
                before: B64.encode(&a.bytes),
                after: B64.encode(&b.bytes),
            })
            .collect(),
    };
    let jp = journal_path(&path);
    let mut temporary_journal = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
    serde_json::to_writer(&mut temporary_journal, &journal)?;
    temporary_journal.as_file().sync_all()?;
    let jf = temporary_journal
        .persist_noclobber(&jp)
        .map_err(|e| e.error)?;
    sync_parent(&jp)?;
    for (_, page) in changes {
        file.seek(SeekFrom::Start(page.offset))?;
        file.write_all(&page.bytes)?;
        report.bytes_written += page.bytes.len() as u64;
    }
    file.sync_all()?;
    drop(jf);
    fs::remove_file(&jp)?;
    sync_parent(&jp)?;
    Ok(report)
}
pub fn recover(path: &Path) -> Result<EditReport> {
    let path = path.canonicalize()?;
    let mut file = OpenOptions::new().read(true).write(true).open(&path)?;
    file.try_lock_exclusive().map_err(|_| {
        fail(
            "file_busy",
            "The audiobook is being edited by another process",
        )
    })?;
    let jp = journal_path(&path);
    check_identity(&path, &file)?;
    ensure!(
        fs::metadata(&jp)?.len() < (MAX_HEADER * 4) as u64,
        "Recovery journal is too large"
    );
    let j: Journal = serde_json::from_reader(File::open(&jp)?)?;
    ensure!(
        j.version == 1 && j.file_len == file.metadata()?.len() && j.identity == identity(&file)?,
        "Recovery journal belongs to a different file"
    );
    let head = read_page(&mut file)?;
    ensure!(
        hash(&head.bytes) == j.head_hash && j.header_end <= MAX_HEADER as u64 + 65536,
        "Invalid recovery header"
    );
    let mut validated = Vec::new();
    let mut end = head.bytes.len() as u64;
    for p in &j.pages {
        let before = B64.decode(&p.before)?;
        let after = B64.decode(&p.after)?;
        ensure!(
            before.len() == after.len()
                && p.offset >= end
                && p.offset + before.len() as u64 <= j.header_end,
            "Invalid journal page range"
        );
        let old = read_page(&mut std::io::Cursor::new(&before))?;
        ensure!(old.serial() == head.serial(), "Recovery stream mismatch");
        end = p.offset + before.len() as u64;
        validated.push((p.offset, before));
    }
    let mut bytes = 0;
    for (offset, before) in &validated {
        file.seek(SeekFrom::Start(*offset))?;
        file.write_all(before)?;
        bytes += before.len() as u64;
    }
    file.sync_all()?;
    fs::remove_file(jp)?;
    sync_parent(&path)?;
    let h = read_header(&mut file)?;
    Ok(EditReport {
        mode: "recovered".into(),
        pages_written: validated.len(),
        bytes_written: bytes,
        audio_bytes_written: 0,
        capacity_bytes: h.capacity,
        available_bytes: h.capacity - h.used,
    })
}

pub fn comment_pages(packet: &[u8], serial: u32) -> Vec<Page> {
    let mut pages = Vec::new();
    let mut at = 0;
    loop {
        let remaining = packet.len() - at;
        let n = remaining.min(65025);
        let mut lacing = vec![255u8; n / 255];
        let done = remaining < 65025;
        if done {
            lacing.push((n % 255) as u8);
        }
        let mut bytes = vec![0; 27];
        bytes[..4].copy_from_slice(b"OggS");
        bytes[5] = if pages.is_empty() { 0 } else { 1 };
        bytes[6..14].copy_from_slice(&if done { 0u64 } else { u64::MAX }.to_le_bytes());
        bytes[14..18].copy_from_slice(&serial.to_le_bytes());
        bytes[18..22].copy_from_slice(&(pages.len() as u32 + 1).to_le_bytes());
        bytes[26] = lacing.len() as u8;
        bytes.extend(lacing);
        let header_len = bytes.len();
        bytes.extend_from_slice(&packet[at..at + n]);
        let mut p = Page {
            offset: 0,
            bytes,
            header_len,
        };
        p.checksum();
        pages.push(p);
        at += n;
        if done {
            break;
        }
    }
    pages
}
/// Create a new padded file, copying compressed audio pages without decoding.
pub fn rewrite(
    source: &Path,
    destination: &Path,
    patch: &MetadataPatch,
    padding: usize,
    overwrite: bool,
) -> Result<EditReport> {
    ensure!(padding <= MAX_HEADER / 2, "Padding must be at most 8 MiB");
    let source = source.canonicalize()?;
    let mut input = File::open(&source)?;
    if destination.canonicalize().is_ok_and(|p| p == source) {
        input.try_lock_exclusive().map_err(|_| {
            fail(
                "file_busy",
                "The audiobook is being edited by another process",
            )
        })?;
    } else {
        FileExt::try_lock_shared(&input).map_err(|_| {
            fail(
                "file_busy",
                "The audiobook is being edited by another process",
            )
        })?;
    }
    check_identity(&source, &input)?;
    check_journal(&source)?;
    let h = read_header(&mut input)?;
    let duration = duration_ms(&mut input, &h)?;
    let comments = h.patched_comments(patch, Some(duration))?;
    let used = 8
        + 4
        + h.vendor.len()
        + 4
        + comments.iter().map(|s| 4 + s.len()).sum::<usize>()
        + h.opaque.len();
    let packet = h.packet(&comments, used + padding)?;
    let pages = comment_pages(&packet, h.head.serial());
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut out = tempfile::NamedTempFile::new_in(parent)?;
    fs::set_permissions(out.path(), input.metadata()?.permissions())?;
    out.write_all(&h.head.bytes)?;
    for p in &pages {
        out.write_all(&p.bytes)?;
    }
    input.seek(SeekFrom::Start(h.end))?;
    let delta = pages.len() as i64 - h.pages.len() as i64;
    let mut audio_bytes = 0;
    let mut count = 0;
    let len = input.metadata()?.len();
    while input.stream_position()? < len {
        if crate::backend::is_cancelled() {
            return Err(crate::error::cancelled());
        }
        let mut p = read_page(&mut input)?;
        ensure!(
            p.serial() == h.head.serial() && p.bytes[5] & 2 == 0,
            "Chained/multiplexed Ogg files are not supported"
        );
        let seq = p.sequence() as i64 + delta;
        ensure!(
            (0..=u32::MAX as i64).contains(&seq),
            "Ogg page sequence overflow"
        );
        p.bytes[18..22].copy_from_slice(&(seq as u32).to_le_bytes());
        p.checksum();
        out.write_all(&p.bytes)?;
        audio_bytes += p.bytes.len() as u64;
        count += 1;
    }
    out.as_file().sync_all()?;
    if overwrite {
        out.persist(destination).map_err(|e| e.error)?;
    } else {
        out.persist_noclobber(destination).map_err(|e| e.error)?;
    }
    sync_parent(destination)?;
    Ok(EditReport {
        mode: "repacked".into(),
        pages_written: pages.len() + count + 1,
        bytes_written: fs::metadata(destination)?.len(),
        audio_bytes_written: audio_bytes,
        capacity_bytes: packet.len(),
        available_bytes: padding,
    })
}

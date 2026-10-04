use fs2::FileExt;
use opusab::{
    model::{Chapter, MetadataPatch},
    ogg,
};
use std::{
    fs::{self, File},
    io::{Seek, SeekFrom, Write},
    path::Path,
};

fn fixture(dir: &Path) -> std::path::PathBuf {
    let p = dir.join("book.opus");
    fs::write(&p, include_bytes!("fixtures/tone.opus")).unwrap();
    p
}
fn header(p: &Path) -> ogg::Header {
    ogg::read_header(&mut File::open(p).unwrap()).unwrap()
}
fn audio_packets(p: &Path) -> Vec<Vec<u8>> {
    let mut file = File::open(p).unwrap();
    let h = ogg::read_header(&mut file).unwrap();
    file.seek(SeekFrom::Start(h.end)).unwrap();
    let mut result = Vec::new();
    let mut packet = Vec::new();
    while file.stream_position().unwrap() < file.metadata().unwrap().len() {
        let page = ogg::read_page(&mut file).unwrap();
        let mut at = 0;
        for n in &page.bytes[27..page.header_len] {
            packet.extend_from_slice(&page.body()[at..at + *n as usize]);
            at += *n as usize;
            if *n < 255 {
                result.push(std::mem::take(&mut packet));
            }
        }
    }
    assert!(packet.is_empty());
    result
}
#[test]
fn in_place_keeps_inode_length_offsets_and_audio_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let p = fixture(dir.path());
    ogg::rewrite(
        &p,
        &p,
        &MetadataPatch::default(),
        ogg::DEFAULT_PADDING,
        true,
    )
    .unwrap();
    let before = fs::read(&p).unwrap();
    let h = header(&p);
    #[cfg(unix)]
    let inode = {
        use std::os::unix::fs::MetadataExt;
        fs::metadata(&p).unwrap().ino()
    };
    let patch = MetadataPatch {
        title: Some("Прощай, оружие — café 🎧".into()),
        author: Some("Ernest Hemingway".into()),
        chapters: Some(vec![
            Chapter {
                start_ms: 0,
                title: "Start".into(),
            },
            Chapter {
                start_ms: 100,
                title: "Second".into(),
            },
        ]),
        ..Default::default()
    };
    let report = ogg::edit(&p, &patch).unwrap();
    let after = fs::read(&p).unwrap();
    assert_eq!(before.len(), after.len());
    assert_eq!(h.end, header(&p).end);
    assert_eq!(before[h.end as usize..], after[h.end as usize..]);
    assert_eq!(report.audio_bytes_written, 0);
    assert!(report.bytes_written <= h.end);
    assert_eq!(header(&p).metadata().title, patch.title.unwrap());
    assert_eq!(header(&p).metadata().chapters.len(), 2);
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(inode, fs::metadata(&p).unwrap().ino());
    }
    assert!(!ogg::journal_path(&p).exists());
}
#[test]
fn no_room_is_an_error_without_any_write() {
    let dir = tempfile::tempdir().unwrap();
    let p = fixture(dir.path());
    let before = fs::read(&p).unwrap();
    let e = ogg::edit(
        &p,
        &MetadataPatch {
            description: Some("x".repeat(100000)),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(e.to_string().contains("Metadata needs"));
    assert_eq!(before, fs::read(&p).unwrap());
    assert!(!ogg::journal_path(&p).exists());
}
#[test]
fn reserve_preserves_compressed_packets_and_duration() {
    let dir = tempfile::tempdir().unwrap();
    let p = fixture(dir.path());
    let packets = audio_packets(&p);
    let duration = ogg::duration_ms(&mut File::open(&p).unwrap(), &header(&p)).unwrap();
    ogg::rewrite(&p, &p, &MetadataPatch::default(), 300000, true).unwrap();
    assert_eq!(packets, audio_packets(&p));
    assert_eq!(
        duration,
        ogg::duration_ms(&mut File::open(&p).unwrap(), &header(&p)).unwrap()
    );
    assert_eq!(header(&p).capacity - header(&p).used, 300000);
}
#[test]
fn page_lacing_handles_exact_multiples_and_empty_last_segment() {
    for n in [0, 1, 254, 255, 256, 65024, 65025, 65026, 130050] {
        let packet = vec![42; n];
        let pages = ogg::comment_pages(&packet, 123);
        let mut roundtrip = Vec::new();
        for (i, p) in pages.iter().enumerate() {
            assert_eq!(p.sequence(), i as u32 + 1);
            let parsed = ogg::read_page(&mut std::io::Cursor::new(&p.bytes)).unwrap();
            roundtrip.extend_from_slice(parsed.body());
            if i == pages.len() - 1 {
                assert!(*p.bytes[27..p.header_len].last().unwrap() < 255);
            } else {
                assert!(p.bytes[27..p.header_len].iter().all(|b| *b == 255));
            }
        }
        assert_eq!(packet, roundtrip);
    }
}
#[test]
fn custom_repeated_tags_survive_other_edits() {
    let dir = tempfile::tempdir().unwrap();
    let p = fixture(dir.path());
    ogg::rewrite(&p, &p, &MetadataPatch::default(), 4096, true).unwrap();
    let mut extra = std::collections::BTreeMap::new();
    extra.insert(
        "MY_CUSTOM_TAG".into(),
        vec!["first".into(), "second".into()],
    );
    ogg::edit(
        &p,
        &MetadataPatch {
            extra: Some(extra.clone()),
            ..Default::default()
        },
    )
    .unwrap();
    ogg::edit(
        &p,
        &MetadataPatch {
            title: Some("New title".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(header(&p).tags()["MY_CUSTOM_TAG"], extra["MY_CUSTOM_TAG"]);
}
#[test]
fn empty_patch_is_no_op() {
    let dir = tempfile::tempdir().unwrap();
    let p = fixture(dir.path());
    let before = fs::read(&p).unwrap();
    let result = ogg::edit(&p, &MetadataPatch::default()).unwrap();
    assert_eq!(result.bytes_written, 0);
    assert_eq!(fs::read(p).unwrap(), before);
}
#[test]
fn simultaneous_edits_fail_instead_of_racing() {
    let dir = tempfile::tempdir().unwrap();
    let p = fixture(dir.path());
    let file = File::open(&p).unwrap();
    file.lock_exclusive().unwrap();
    let e = ogg::edit(&p, &MetadataPatch::default()).unwrap_err();
    assert!(e.to_string().contains("another process"));
    let e = ogg::rewrite(&p, &p, &MetadataPatch::default(), 4096, true).unwrap_err();
    assert!(e.to_string().contains("another process"));
}
#[test]
fn corrupt_page_and_malformed_tags_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let p = fixture(dir.path());
    let h = header(&p);
    let mut bytes = fs::read(&p).unwrap();
    bytes[h.pages[0].offset as usize + h.pages[0].header_len + 9] ^= 1;
    fs::write(&p, bytes).unwrap();
    assert!(ogg::read_header(&mut File::open(&p).unwrap()).is_err());
}
#[test]
fn interrupted_edit_can_restore_header_without_touching_audio() {
    use base64::{Engine, engine::general_purpose::STANDARD as B64};
    use sha2::{Digest, Sha256};
    let dir = tempfile::tempdir().unwrap();
    let p = fixture(dir.path());
    ogg::rewrite(&p, &p, &MetadataPatch::default(), 4096, true).unwrap();
    let before = fs::read(&p).unwrap();
    let h = header(&p);
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&p)
        .unwrap();
    #[cfg(unix)]
    let identity = {
        use std::os::unix::fs::MetadataExt;
        let m = file.metadata().unwrap();
        format!("{}:{}", m.dev(), m.ino())
    };
    #[cfg(not(unix))]
    let identity = format!("{:?}", file.metadata().unwrap().created().unwrap());
    let page = &h.pages[0];
    let journal = serde_json::json!({"version":1,"file_len":before.len(),"identity":identity,"head_hash":format!("{:x}",Sha256::digest(&h.head.bytes)),"header_end":h.end,"pages":[{"offset":page.offset,"before":B64.encode(&page.bytes),"after":B64.encode(&page.bytes)}]});
    fs::write(ogg::journal_path(&p), serde_json::to_vec(&journal).unwrap()).unwrap();
    file.seek(SeekFrom::Start(page.offset + page.header_len as u64 + 20))
        .unwrap();
    file.write_all(b"torn write").unwrap();
    file.sync_all().unwrap();
    drop(file);
    assert!(
        ogg::edit(&p, &MetadataPatch::default())
            .unwrap_err()
            .to_string()
            .contains("interrupted")
    );
    let report = ogg::recover(&p).unwrap();
    assert_eq!(report.audio_bytes_written, 0);
    assert_eq!(fs::read(&p).unwrap(), before);
    assert!(!ogg::journal_path(&p).exists());
}
#[test]
fn truncated_audio_does_not_replace_destination() {
    let dir = tempfile::tempdir().unwrap();
    let p = fixture(dir.path());
    let h = header(&p);
    let f = fs::OpenOptions::new().write(true).open(&p).unwrap();
    f.set_len(h.end + 20).unwrap();
    let dest = dir.path().join("existing.opus");
    fs::write(&dest, b"keep me").unwrap();
    assert!(ogg::rewrite(&p, &dest, &MetadataPatch::default(), 10, true).is_err());
    assert_eq!(fs::read(dest).unwrap(), b"keep me");
}

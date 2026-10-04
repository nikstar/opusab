use opusab::{
    app::{self, ConversionOptions},
    model::{Chapter, MetadataPatch},
    probe,
};
use std::{fs, process::Command};
fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_opusab"))
}
#[test]
fn metadata_commands_need_no_backend() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("book.opus");
    fs::write(&p, include_bytes!("fixtures/tone.opus")).unwrap();
    let out = bin()
        .env("PATH", "")
        .args(["tags", "show"])
        .arg(&p)
        .arg("--json")
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["metadata"]["title"], "Tone");
}

#[cfg(unix)]
#[test]
fn installed_symlink_finds_its_adjacent_runtime() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("application");
    fs::create_dir_all(bundle.join("runtime/bin")).unwrap();
    let exe = bundle.join("opusab");
    fs::copy(env!("CARGO_BIN_EXE_opusab"), &exe).unwrap();
    for name in ["freaccmd", "ffprobe", "ffmpeg"] {
        fs::write(bundle.join("runtime/bin").join(name), b"fixture").unwrap();
    }
    let link = dir.path().join("installed-opusab");
    std::os::unix::fs::symlink(exe, &link).unwrap();
    let output = Command::new(link)
        .env_remove("OPUSAB_RUNTIME")
        .env_remove("OPUSAB_FREACCMD")
        .env_remove("OPUSAB_FFPROBE")
        .env_remove("OPUSAB_FFMPEG")
        .env("PATH", "")
        .args(["doctor", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    for tool in value["tools"].as_array().unwrap() {
        assert!(
            std::path::Path::new(tool["path"].as_str().unwrap())
                .starts_with(bundle.canonicalize().unwrap())
        );
    }
}
#[test]
fn agent_errors_are_json_and_do_not_open_tui() {
    let out = bin()
        .args(["convert", "--jobs", "invalid", "--json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let e: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(e["error"]["code"], "invalid_arguments");
    let out = bin().args(["some-book.m4b", "--json"]).output().unwrap();
    let e: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(e["error"]["code"], "terminal_required");
}
#[test]
fn existing_opus_keeps_audio_unless_explicitly_reencoded() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("book.opus");
    fs::write(&p, include_bytes!("fixtures/tone.opus")).unwrap();
    let plan = app::make_plan(std::slice::from_ref(&p), &ConversionOptions::default()).unwrap();
    assert_eq!(plan.operation, "metadata");
    assert_eq!(plan.output, p.canonicalize().unwrap());
    let options = ConversionOptions {
        bitrate: Some(32),
        ..Default::default()
    };
    let plan = app::make_plan(std::slice::from_ref(&p), &options).unwrap();
    assert_eq!(plan.operation, "encode");
    assert_ne!(plan.output, p);
    let options = ConversionOptions {
        bitrate: Some(32),
        output: Some(p.clone()),
        overwrite: true,
        ..Default::default()
    };
    assert!(app::make_plan(&[p], &options).is_err());
}
#[test]
fn json_patch_rejects_typos_and_resolves_cover_relative_to_json() {
    let dir = tempfile::tempdir().unwrap();
    let patch = dir.path().join("metadata.json");
    fs::write(&patch, r#"{"autor":"typo"}"#).unwrap();
    assert!(app::read_patch(&patch).is_err());
    fs::write(
        &patch,
        r#"{"schema_version":1,"metadata":{"title":"New","cover":"cover.png"}}"#,
    )
    .unwrap();
    let p = app::read_patch(&patch).unwrap();
    assert_eq!(p.cover.unwrap(), dir.path().join("cover.png"));
}
#[test]
fn generated_and_edited_chapter_times_keep_millisecond_precision() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.opus");
    let b = dir.path().join("b.opus");
    fs::write(&a, include_bytes!("fixtures/tone.opus")).unwrap();
    fs::write(&b, include_bytes!("fixtures/tone.opus")).unwrap();
    let mut book = probe::book(&[a, b]).unwrap();
    assert_eq!(book.metadata.chapters[1].start_ms, 250);
    book.metadata.chapters = vec![
        Chapter {
            start_ms: 0,
            title: "One".into(),
        },
        Chapter {
            start_ms: 110,
            title: "Manual timing".into(),
        },
    ];
    let plan = app::plan_book(
        book,
        &ConversionOptions::default(),
        MetadataPatch::default(),
    )
    .unwrap();
    assert_eq!(plan.book.metadata.chapters[1].start_ms, 110);
}

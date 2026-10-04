//! Optional integration test against the real audio backend.
use opusab::{app, model::Chapter, probe};
use std::fs;

#[test]
#[ignore = "requires freaccmd and ffprobe; run cargo test --test backend -- --ignored"]
fn parallel_encoder_retains_fractional_chapter_times_and_manual_names() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("01 chapter's café.opus");
    let b = dir.path().join("02 chapter.opus");
    fs::write(&a, include_bytes!("fixtures/tone.opus")).unwrap();
    fs::write(&b, include_bytes!("fixtures/tone.opus")).unwrap();
    let output = dir.path().join("merged.opus");
    let options = app::ConversionOptions {
        output: Some(output.clone()),
        jobs: 2,
        ..Default::default()
    };
    let mut plan = app::make_plan(&[a, b], &options).unwrap();
    plan.book.metadata.chapters[1].title = "A custom name".into();
    app::execute(&plan, &mut |_| {}).unwrap();
    let book = probe::book(&[output]).unwrap();
    assert!(book.duration_ms.abs_diff(500) <= 20);
    assert_eq!(
        book.metadata.chapters[1],
        Chapter {
            start_ms: 250,
            title: "A custom name".into(),
        }
    );
}

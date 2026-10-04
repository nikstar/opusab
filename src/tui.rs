use crate::{
    app::{self, ConversionOptions, ConversionReport},
    backend::{self, Progress},
    model::{self, Book, Channels, Chapter, MetadataPatch, Preset},
    probe,
};
use anyhow::{Result, ensure};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph, Row, Table, TableState, Tabs, Wrap},
};
use std::{io, path::PathBuf, sync::mpsc, thread, time::Duration};
use unicode_width::UnicodeWidthStr;

const ACCENT: Color = Color::Rgb(84, 211, 181);
const MUTED: Color = Color::Rgb(137, 150, 165);
const BG: Color = Color::Rgb(19, 24, 31);
const SELECT: Color = Color::Rgb(35, 57, 61);
const GOLD: Color = Color::Rgb(238, 183, 93);
const BOOK_FIELDS: [&str; 11] = [
    "Title",
    "Author",
    "Narrator",
    "Series",
    "Series number",
    "Year / date",
    "Genre",
    "Description",
    "Cover image",
    "Output file",
    "Remove cover",
];
const SETTING_FIELDS: [&str; 6] = [
    "Bitrate (kbit/s)",
    "Preset",
    "Channels",
    "Workers",
    "Metadata reserve (KiB)",
    "Replace existing output",
];

#[derive(Clone, Copy)]
enum EditTarget {
    Book(usize),
    ChapterName(usize),
    ChapterTime(usize),
    Setting(usize),
}
struct Editor {
    title: String,
    text: String,
    cursor: usize,
    target: EditTarget,
}
enum WorkerEvent {
    Progress(Progress),
    Done(Result<ConversionReport, String>),
}
struct State {
    book: Book,
    options: ConversionOptions,
    tab: usize,
    selected: [usize; 4],
    editor: Option<Editor>,
    message: String,
    error: bool,
    dirty: bool,
    quit_confirm: bool,
    remove_cover: bool,
    worker: Option<mpsc::Receiver<WorkerEvent>>,
    last_output: Option<PathBuf>,
}
impl State {
    fn new(book: Book) -> Self {
        Self {
            book,
            options: ConversionOptions::default(),
            tab: 0,
            selected: [0; 4],
            editor: None,
            message: "Review metadata and chapters, then press F5 to save or convert.".into(),
            error: false,
            dirty: false,
            quit_confirm: false,
            remove_cover: false,
            worker: None,
            last_output: None,
        }
    }
    fn count(&self) -> usize {
        match self.tab {
            0 => BOOK_FIELDS.len(),
            1 => self.book.metadata.chapters.len(),
            2 => self.book.sources.len(),
            _ => SETTING_FIELDS.len(),
        }
    }
    fn book_values(&self) -> Vec<String> {
        let m = &self.book.metadata;
        vec![
            m.title.clone(),
            m.author.clone(),
            m.narrator.clone(),
            m.series.clone(),
            m.series_part.clone(),
            m.date.clone(),
            m.genre.clone(),
            m.description.clone(),
            m.cover
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| {
                    if self.book.sources.iter().any(|s| s.has_embedded_cover) && !self.remove_cover
                    {
                        "Embedded cover · preserved".into()
                    } else {
                        "None".into()
                    }
                }),
            self.output_text(),
            if self.remove_cover { "yes" } else { "no" }.into(),
        ]
    }
    fn output_text(&self) -> String {
        self.options
            .output
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| {
                app::plan_book(self.book.clone(), &self.options, MetadataPatch::default())
                    .map(|p| p.output.display().to_string())
                    .unwrap_or_else(|_| "Choose an output file".into())
            })
    }
    fn setting_values(&self) -> Vec<String> {
        let r = model::recommend(&self.book, self.options.preset);
        vec![
            self.options
                .bitrate
                .map(|v| v.to_string())
                .unwrap_or_else(|| format!("auto · suggested {} VBR", r.bitrate_kbps)),
            format!("{:?}", self.options.preset).to_lowercase(),
            format!("{:?}", self.options.channels).to_lowercase(),
            if self.options.jobs == 0 {
                "auto".into()
            } else {
                self.options.jobs.to_string()
            },
            self.options.padding_kib.to_string(),
            if self.options.overwrite { "yes" } else { "no" }.into(),
        ]
    }
    fn open(&mut self, target: EditTarget) {
        let (title, text) = match target {
            EditTarget::Book(i) => {
                let mut s = self.book_values()[i].clone();
                if i == 8 {
                    s = self
                        .book
                        .metadata
                        .cover
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default();
                }
                (BOOK_FIELDS[i].into(), s)
            }
            EditTarget::ChapterName(i) => (
                "Chapter title".into(),
                self.book.metadata.chapters[i].title.clone(),
            ),
            EditTarget::ChapterTime(i) => (
                "Chapter start · HH:MM:SS.mmm".into(),
                model::time_string(self.book.metadata.chapters[i].start_ms),
            ),
            EditTarget::Setting(i) => {
                let mut s = self.setting_values()[i].clone();
                if i == 0 && self.options.bitrate.is_none() {
                    s = "auto".into();
                }
                let label = match i {
                    1 => "Preset · speech / compact / mixed",
                    2 => "Channels · keep / mono / stereo",
                    _ => SETTING_FIELDS[i],
                };
                (label.into(), s)
            }
        };
        self.editor = Some(Editor {
            cursor: text.len(),
            title,
            text,
            target,
        });
    }
    fn commit_editor(&mut self) -> Result<()> {
        let Some(e) = self.editor.as_ref() else {
            return Ok(());
        };
        let text = e.text.clone();
        match e.target {
            EditTarget::Book(i) => {
                let m = &mut self.book.metadata;
                match i {
                    0 => m.title = text,
                    1 => m.author = text,
                    2 => m.narrator = text,
                    3 => m.series = text,
                    4 => m.series_part = text,
                    5 => m.date = text,
                    6 => m.genre = text,
                    7 => m.description = text,
                    8 => {
                        if text.trim().is_empty() {
                            m.cover = None;
                        } else {
                            let p = PathBuf::from(text);
                            ensure!(p.is_file(), "Cover file does not exist");
                            m.cover = Some(p.canonicalize()?);
                            self.remove_cover = false;
                        }
                    }
                    9 => {
                        self.options.output = if text.trim().is_empty() {
                            None
                        } else {
                            Some(PathBuf::from(text))
                        }
                    }
                    10 => {
                        self.remove_cover = parse_bool(&text)?;
                        if self.remove_cover {
                            m.cover = None;
                        }
                    }
                    _ => {}
                }
            }
            EditTarget::ChapterName(i) => self.book.metadata.chapters[i].title = text,
            EditTarget::ChapterTime(i) => {
                let ms = model::parse_time(&text)?;
                ensure!(
                    ms < self.book.duration_ms,
                    "Chapter starts beyond end of book"
                );
                self.book.metadata.chapters[i].start_ms = ms;
                self.book.metadata.chapters.sort_by_key(|c| c.start_ms);
            }
            EditTarget::Setting(i) => match i {
                0 => {
                    self.options.bitrate = if text == "auto" || text.is_empty() {
                        None
                    } else {
                        let n: u32 = text.parse()?;
                        ensure!((6..=510).contains(&n), "Bitrate must be 6..510");
                        Some(n)
                    };
                }
                1 => {
                    self.options.preset = match text.to_lowercase().as_str() {
                        "speech" => Preset::Speech,
                        "compact" => Preset::Compact,
                        "mixed" => Preset::Mixed,
                        _ => anyhow::bail!("Choose speech, compact, or mixed"),
                    }
                }
                2 => {
                    self.options.channels = match text.to_lowercase().as_str() {
                        "keep" => Channels::Keep,
                        "mono" => Channels::Mono,
                        "stereo" => Channels::Stereo,
                        _ => anyhow::bail!("Choose keep, mono, or stereo"),
                    }
                }
                3 => {
                    let n = if text == "auto" { 0 } else { text.parse()? };
                    ensure!(n <= 256, "Workers must be 1..256, or auto");
                    self.options.jobs = n;
                }
                4 => {
                    let n = text.parse()?;
                    ensure!(n <= 8192, "Reserve must be 0..8192 KiB");
                    self.options.padding_kib = n;
                }
                5 => self.options.overwrite = parse_bool(&text)?,
                _ => {}
            },
        }
        self.editor = None;
        self.dirty = true;
        self.error = false;
        self.message = "Edited. F5 applies your changes; q leaves the source untouched.".into();
        Ok(())
    }
    fn save(&mut self) -> Result<()> {
        model::validate_metadata(&self.book.metadata, Some(self.book.duration_ms))?;
        let mut patch = MetadataPatch::from(&self.book.metadata);
        patch.remove_cover = self.remove_cover;
        let plan = app::plan_book(self.book.clone(), &self.options, patch)?;
        backend::reset_cancel();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut progress = |p| {
                let _ = tx.send(WorkerEvent::Progress(p));
            };
            let result = app::execute(&plan, &mut progress).map_err(|e| format!("{e:#}"));
            let _ = tx.send(WorkerEvent::Done(result));
        });
        self.worker = Some(rx);
        self.error = false;
        self.message = "Starting… Ctrl+C cancels the operation.".into();
        Ok(())
    }
    fn poll(&mut self) {
        let messages: Vec<_> = self
            .worker
            .as_ref()
            .map(|rx| rx.try_iter().collect())
            .unwrap_or_default();
        for message in messages {
            match message {
                WorkerEvent::Progress(p) => {
                    self.message = format!(
                        "{} · {:.0}s · Ctrl+C to cancel",
                        p.message, p.elapsed_seconds
                    )
                }
                WorkerEvent::Done(result) => {
                    self.worker = None;
                    backend::reset_cancel();
                    match result {
                        Ok(report) => {
                            self.last_output = Some(report.output.clone());
                            match probe::book(std::slice::from_ref(&report.output)) {
                                Ok(book) => {
                                    self.book = book;
                                    self.options = ConversionOptions::default();
                                    self.selected = [0; 4];
                                    self.remove_cover = false;
                                    self.dirty = false;
                                }
                                Err(e) => {
                                    self.message = e.to_string();
                                    self.error = true;
                                    continue;
                                }
                            }
                            self.error = false;
                            self.message = format!(
                                "Saved {} · {:.1}s · {} audio bytes rewritten",
                                report
                                    .output
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy(),
                                report.elapsed_seconds,
                                report.metadata.audio_bytes_written
                            );
                        }
                        Err(error) => {
                            self.error = true;
                            self.message = error;
                        }
                    }
                }
            }
        }
    }
    fn key(&mut self, key: KeyEvent) -> Result<bool> {
        if key.kind == KeyEventKind::Release {
            return Ok(false);
        }
        if self.worker.is_some() {
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                backend::cancel();
                self.message = "Cancelling…".into();
            }
            return Ok(false);
        }
        if self.editor.is_some() {
            if key.code == KeyCode::Esc {
                self.editor = None;
                return Ok(false);
            }
            if key.code == KeyCode::Enter {
                self.commit_editor()?;
                return Ok(false);
            }
            let e = self.editor.as_mut().unwrap();
            match key.code {
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    e.text.clear();
                    e.cursor = 0;
                }
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    e.text.insert(e.cursor, c);
                    e.cursor += c.len_utf8();
                }
                KeyCode::Backspace if e.cursor > 0 => {
                    let previous = e.text[..e.cursor].char_indices().last().unwrap().0;
                    e.text.drain(previous..e.cursor);
                    e.cursor = previous;
                }
                KeyCode::Delete if e.cursor < e.text.len() => {
                    let next = e.cursor + e.text[e.cursor..].chars().next().unwrap().len_utf8();
                    e.text.drain(e.cursor..next);
                }
                KeyCode::Left if e.cursor > 0 => {
                    e.cursor = e.text[..e.cursor].char_indices().last().unwrap().0
                }
                KeyCode::Right if e.cursor < e.text.len() => {
                    e.cursor += e.text[e.cursor..].chars().next().unwrap().len_utf8()
                }
                KeyCode::Home => e.cursor = 0,
                KeyCode::End => e.cursor = e.text.len(),
                _ => {}
            }
            return Ok(false);
        }
        if self.quit_confirm {
            self.quit_confirm = false;
            return Ok(matches!(key.code, KeyCode::Char('y') | KeyCode::Char('Y')));
        }
        match key.code {
            KeyCode::Char('q')|KeyCode::Esc=>{if self.dirty{self.quit_confirm=true;}else{return Ok(true);}},
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL)=>{if self.dirty{self.quit_confirm=true;}else{return Ok(true);}},
            KeyCode::Tab=>self.tab=(self.tab+1)%4,KeyCode::BackTab=>self.tab=(self.tab+3)%4,
            KeyCode::Char(c @ '1'..='4')=>self.tab=c as usize-'1' as usize,
            KeyCode::Up|KeyCode::Char('k')=>self.selected[self.tab]=self.selected[self.tab].saturating_sub(1),
            KeyCode::Down|KeyCode::Char('j')=>self.selected[self.tab]=(self.selected[self.tab]+1).min(self.count().saturating_sub(1)),
            KeyCode::Home=>self.selected[self.tab]=0,KeyCode::End=>self.selected[self.tab]=self.count().saturating_sub(1),
            KeyCode::Enter=>{let i=self.selected[self.tab];match self.tab{0=>self.open(EditTarget::Book(i)),1 if self.count()>0=>self.open(EditTarget::ChapterName(i)),3=>self.open(EditTarget::Setting(i)),_=>{}}},
            KeyCode::Char('t') if self.tab==1&&self.count()>0=>self.open(EditTarget::ChapterTime(self.selected[1])),
            KeyCode::Char('a') if self.tab==1=>{let start=self.book.metadata.chapters.last().map(|c|c.start_ms+1000).unwrap_or(0);ensure!(start<self.book.duration_ms,"No room for another chapter");self.book.metadata.chapters.push(Chapter{start_ms:start,title:format!("Chapter {}",self.count()+1)});self.selected[1]=self.count()-1;self.dirty=true;self.open(EditTarget::ChapterName(self.selected[1]));},
            KeyCode::Char('d')|KeyCode::Delete if self.tab==1&&self.count()>0=>{self.book.metadata.chapters.remove(self.selected[1]);self.selected[1]=self.selected[1].min(self.count().saturating_sub(1));self.dirty=true;},
            KeyCode::Char('r') if self.tab==1=>{self.book.rebuild_chapters();self.selected[1]=0;self.dirty=true;self.message="Chapters regenerated from the current source order.".into();},
            KeyCode::Char('J')|KeyCode::Char('K') if self.tab==2=>{let i=self.selected[2];let to=if key.code==KeyCode::Char('J'){(i+1).min(self.count()-1)}else{i.saturating_sub(1)};self.book.sources.swap(i,to);self.selected[2]=to;self.book.rebuild_chapters();self.dirty=true;self.message="Files reordered; chapter timings regenerated.".into();},
            KeyCode::F(5)=>self.save()?,
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL)=>self.save()?,
            KeyCode::Char('?')=>self.message="Tab switches views · Enter edits · F5 saves/converts · Chapters: t time, a add, d delete, r regenerate · Files: Shift+J/K reorder".into(),
            _=>{}
        }
        Ok(false)
    }
}
fn parse_bool(s: &str) -> Result<bool> {
    match s.to_lowercase().as_str() {
        "yes" | "true" | "1" => Ok(true),
        "no" | "false" | "0" => Ok(false),
        _ => anyhow::bail!("Use yes or no"),
    }
}
struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, crossterm::cursor::Show);
    }
}
pub fn run(source: PathBuf) -> Result<()> {
    let book = probe::book(&[source])?;
    let mut state = State::new(book);
    enable_raw_mode()?;
    let guard = TerminalGuard;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.clear()?;
    loop {
        state.poll();
        terminal.draw(|f| draw(f, &state))?;
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            match state.key(key) {
                Ok(true) => break,
                Ok(false) => {}
                Err(e) => {
                    state.error = true;
                    state.message = format!("{e:#}");
                }
            }
        }
    }
    drop(terminal);
    drop(guard);
    if let Some(path) = state.last_output {
        println!("Saved {}", path.display());
    }
    Ok(())
}
fn panel(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(57, 69, 82)))
        .title(Line::from(format!(" {title} ")).fg(MUTED))
}
fn draw(f: &mut Frame, state: &State) {
    let area = f.area();
    f.render_widget(
        Block::default().style(Style::default().bg(BG).fg(Color::Rgb(229, 235, 240))),
        area,
    );
    if area.width < 54 || area.height < 16 {
        f.render_widget(
            Paragraph::new("Please enlarge the terminal to at least 54 × 16.\nPress q to close.")
                .wrap(Wrap { trim: false }),
            area,
        );
        return;
    }
    let areas = Layout::vertical([
        Constraint::Length(4),
        Constraint::Length(3),
        Constraint::Min(4),
        Constraint::Length(4),
        Constraint::Length(1),
    ])
    .margin(1)
    .split(area);
    let title = Line::from(vec![
        Span::styled(" OPUSAB ", Style::default().fg(BG).bg(ACCENT).bold()),
        Span::raw("  "),
        Span::styled(&state.book.metadata.title, Style::default().bold()),
        Span::styled(
            if state.dirty { "  • edited" } else { "" },
            Style::default().fg(GOLD),
        ),
    ]);
    let summary = format!(
        "{}  ·  {}  ·  {} files  ·  {} chapters",
        state.book.metadata.author,
        model::time_string(state.book.duration_ms),
        state.book.sources.len(),
        state.book.metadata.chapters.len()
    );
    let mode = if state.worker.is_some() {
        "Working…"
    } else if state.book.sources.len() == 1
        && state.book.sources[0].codec == "opus"
        && state.options.bitrate.is_none()
        && state.options.channels == Channels::Keep
        && state.options.preset == Preset::Speech
    {
        "Existing Opus · audio kept · save edits in place"
    } else {
        "Prepare an audiobook · encode to one Opus file"
    };
    f.render_widget(
        Paragraph::new(vec![
            title,
            Line::from(summary).fg(MUTED),
            Line::from(mode).fg(ACCENT),
        ]),
        areas[0],
    );
    f.render_widget(
        Tabs::new(["1  Book", "2  Chapters", "3  Files", "4  Encoding"])
            .select(state.tab)
            .style(Style::default().fg(MUTED))
            .highlight_style(Style::default().fg(ACCENT).bold())
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::default().fg(MUTED)),
            ),
        areas[1],
    );
    let mut selection = TableState::default().with_selected(if state.count() > 0 {
        Some(state.selected[state.tab])
    } else {
        None
    });
    match state.tab {
        0 | 3 => {
            let (labels, values) = if state.tab == 0 {
                (BOOK_FIELDS.to_vec(), state.book_values())
            } else {
                (SETTING_FIELDS.to_vec(), state.setting_values())
            };
            let rows = labels.into_iter().zip(values).map(|(k, v)| {
                Row::new(vec![
                    k.to_string(),
                    if v.is_empty() { "—".into() } else { v },
                ])
                .style(Style::default().fg(Color::Rgb(220, 229, 235)))
            });
            let table = Table::new(
                rows,
                [
                    Constraint::Length(if state.tab == 0 { 16 } else { 26 }),
                    Constraint::Min(20),
                ],
            )
            .row_highlight_style(Style::default().bg(SELECT).fg(ACCENT))
            .highlight_symbol("› ")
            .block(panel("Enter to edit"));
            f.render_stateful_widget(table, areas[2], &mut selection);
        }
        1 => {
            let rows = state
                .book
                .metadata
                .chapters
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    Row::new(vec![
                        (i + 1).to_string(),
                        model::time_string(c.start_ms),
                        c.title.clone(),
                    ])
                });
            let table = Table::new(
                rows,
                [
                    Constraint::Length(5),
                    Constraint::Length(15),
                    Constraint::Min(12),
                ],
            )
            .header(Row::new(["#", "Start", "Title"]).style(Style::default().fg(MUTED)))
            .row_highlight_style(Style::default().bg(SELECT).fg(ACCENT))
            .highlight_symbol("› ")
            .block(panel(
                "Enter rename · t time · a add · d delete · r regenerate",
            ));
            f.render_stateful_widget(table, areas[2], &mut selection);
        }
        2 => {
            let rows = state.book.sources.iter().enumerate().map(|(i, s)| {
                Row::new(vec![
                    (i + 1).to_string(),
                    s.path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    model::time_string(s.duration_ms),
                    format!("{} / {}k", s.codec, s.bitrate_kbps),
                ])
            });
            let table = Table::new(
                rows,
                [
                    Constraint::Length(4),
                    Constraint::Min(15),
                    Constraint::Length(14),
                    Constraint::Length(13),
                ],
            )
            .header(Row::new(["#", "File", "Duration", "Codec"]).style(Style::default().fg(MUTED)))
            .row_highlight_style(Style::default().bg(SELECT).fg(ACCENT))
            .highlight_symbol("› ")
            .block(panel(
                "Shift+J / Shift+K reorder · chapters follow file order",
            ));
            f.render_stateful_widget(table, areas[2], &mut selection);
        }
        _ => {}
    }
    let msg = if state.quit_confirm {
        "Discard unsaved edits? y = discard and quit · any other key = keep editing"
    } else {
        &state.message
    };
    f.render_widget(
        Paragraph::new(msg)
            .style(Style::default().fg(if state.error {
                Color::LightRed
            } else if state.quit_confirm {
                GOLD
            } else {
                MUTED
            }))
            .wrap(Wrap { trim: false })
            .block(panel(if state.worker.is_some() {
                "Converting"
            } else {
                "Status"
            })),
        areas[3],
    );
    f.render_widget(
        Paragraph::new("↑↓ move   Tab view   Enter edit   F5 / Ctrl+S save   q quit   ? help")
            .style(Style::default().fg(ACCENT)),
        areas[4],
    );
    if let Some(e) = &state.editor {
        let rect = Rect::new(area.x + 3, area.y + area.height / 2 - 2, area.width - 6, 5);
        f.render_widget(Clear, rect);
        let max = rect.width.saturating_sub(4) as usize;
        let mut start = 0;
        while e.text[start..e.cursor].width() >= max && start < e.cursor {
            start += e.text[start..].chars().next().unwrap().len_utf8();
        }
        let visible = &e.text[start..];
        f.render_widget(
            Paragraph::new(vec![
                Line::from(visible),
                Line::from("Enter save · Esc cancel · Ctrl+U clear").fg(MUTED),
            ])
            .style(Style::default().bg(BG).fg(Color::White))
            .block(panel(&e.title)),
            rect,
        );
        f.set_cursor_position((
            rect.x + 1 + e.text[start..e.cursor].width() as u16,
            rect.y + 1,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn state() -> State {
        let book = Book {
            schema_version: 1,
            input_directory: None,
            sources: vec![],
            metadata: crate::model::Metadata {
                title: "Example".into(),
                chapters: vec![Chapter {
                    start_ms: 0,
                    title: "Opening".into(),
                }],
                ..Default::default()
            },
            duration_ms: 60000,
            warnings: vec![],
        };
        let mut s = State::new(book);
        s.options.output = Some(PathBuf::from("/tmp/book.opus"));
        s
    }
    #[test]
    fn unicode_edit_and_cancel() {
        let mut s = state();
        s.open(EditTarget::ChapterName(0));
        s.key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL))
            .unwrap();
        s.key(KeyEvent::new(KeyCode::Char('Ж'), KeyModifiers::NONE))
            .unwrap();
        s.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .unwrap();
        assert_eq!(s.book.metadata.chapters[0].title, "Ж");
        s.open(EditTarget::ChapterName(0));
        s.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .unwrap();
        assert!(s.dirty);
    }
    #[test]
    fn render_small_and_large() {
        let s = state();
        for (w, h) in [(54, 16), (80, 24), (120, 40)] {
            let b = ratatui::backend::TestBackend::new(w, h);
            let mut t = Terminal::new(b).unwrap();
            t.draw(|f| draw(f, &s)).unwrap();
            let text = t
                .backend()
                .buffer()
                .content
                .iter()
                .map(|c| c.symbol())
                .collect::<String>();
            assert!(text.contains("OPUSAB"));
            assert!(text.contains("Example"));
        }
    }
}

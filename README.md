# opusab

Prepare an audiobook, convert it to one Opus file, and edit its metadata from a
terminal. Written in Rust, with a Ratatui editor and a scriptable JSON CLI.

The encoder uses fre:ac's **SuperFast** Opus driver to parallelize a single input,
including a long M4B. The application supplies the audiobook workflow: source
ordering, chapters, metadata, bitrate suggestions, and fast updates afterward.

## Start here

From the portable macOS package:

```sh
./opusab '/path/to/book.m4b'
./opusab '/path/to/folder of MP3s'
./opusab '/path/to/finished-book.opus'
```

A source opens the terminal editor. Review the **Book**, **Chapters**, **Files**,
and **Encoding** tabs, then press **F5** or **Ctrl+S** to save/convert. An existing
Opus file keeps its audio by default. Changing its bitrate, channels, or preset
explicitly requests a new encode into a separate `-converted.opus` file.

| Key | Action |
| --- | --- |
| Tab / Shift+Tab, or 1–4 | Switch tabs |
| ↑ / ↓, or j / k | Select a row |
| Enter | Edit the selected value |
| Ctrl+U / Esc | Clear an input / cancel editing |
| F5 / Ctrl+S | Save metadata or convert |
| t / a / d / r in Chapters | Edit start time / add / delete / regenerate |
| Shift+J / Shift+K in Files | Reorder inputs and regenerate chapters |
| Ctrl+C while working | Cancel and clean up the unfinished output |
| q | Exit; unsaved edits prompt before being discarded |

A terminal of at least 54 × 16 is required; 100 × 30 is more comfortable. Use
explicit commands below in scripts; they never open an editor or prompt.

## CLI and agents

```sh
opusab inspect '/path/to/book.m4b' --json
opusab plan '/path/to/MP3 folder' --json
opusab convert '/path/to/book.m4b' -o book.opus --jobs 6 --json
opusab convert 01.mp3 02.mp3 03.mp3 -o book.opus --metadata metadata.json
opusab tags show book.opus --json
opusab tags set book.opus --title 'A Farewell to Arms' --author 'Ernest Hemingway'
opusab tags apply book.opus --from metadata.json --in-place --json
opusab doctor --json
```

Explicit file arguments retain their order. A directory is scanned recursively;
when all files have track tags, disc/track order wins, otherwise filenames are
naturally sorted (`2` before `10`). Inspect the plan when tags are inconsistent.
Source chapters are preserved; multiple files without chapters become chapters.
Folder artwork prefers `cover`, `folder`, then names containing `front`.

A folder's default output is a **sibling** `Book Title.opus`, so scanning the same
folder again does not pick up the previous output. A single M4B/MP3 defaults to
its filename with `.opus`. Existing outputs require `--overwrite`; conversion
cannot overwrite its source. Temporary output is written beside the destination
and committed only after completion.

`--json` emits one versioned result on stdout, with progress events and errors as
JSON Lines on stderr. `--quiet` suppresses progress. Exit codes are 0 for success,
1 for operational errors, 2 for invalid arguments, and 130 for cancellation.
Errors contain `schema_version`, `error.code`, and `error.message`. Every command
has `--help`; `opusab completions zsh` (or bash/fish/etc.) generates completions.

### Metadata JSON

Use a raw patch or a `{ "schema_version": 1, "metadata": { ... } }` envelope:

```json
{
  "title": "A Farewell to Arms",
  "author": "Ernest Hemingway",
  "narrator": "Narrator name",
  "series": "",
  "series_part": "",
  "date": "1929",
  "genre": "Audiobook",
  "description": "An optional description.",
  "cover": "cover.jpg",
  "chapters": [
    { "start_ms": 0, "title": "Chapter 1" },
    { "start_ms": 125340, "title": "Chapter 2" }
  ],
  "extra": { "LANGUAGE": ["eng"] }
}
```

Missing or `null` fields stay unchanged; an empty string clears a text field.
`chapters` replaces the chapter list, and `[]` removes it. Starts are integer
milliseconds, strictly increasing and within the recording. `cover` accepts JPEG
or PNG up to 8 MiB and resolves relative to the JSON file. `remove_cover: true`
removes embedded artwork. `extra` sets arbitrary tags (arrays allow repeated
values); `remove_tags` removes named extra tags. Dedicated fields must be used
for managed tags such as title or chapters. Unknown fields are rejected.

`opusab tags show book.opus --json > metadata.json` produces a document that can
be edited and applied again. Embedded artwork is preserved without exporting it.
`--from -` and `--metadata -` read JSON from stdin. The full CLI contract and
implementation details are in [docs/DESIGN.md](docs/DESIGN.md).

## Fast metadata updates

New outputs reserve **256 KiB** of extra space in the OpusTags header. Updating
metadata rewrites only changed header pages and their Ogg checksums. It keeps the
file's size, identity, audio bytes, and audio offsets unchanged. Reading the small
header and tail does not require scanning the recording.

If a larger cover or description does not fit, the edit fails with
`metadata_capacity` **before changing anything**. Expand the reserve explicitly:

```sh
opusab tags reserve book.opus --padding-kib 1024
opusab tags apply book.opus --from metadata.json
```

`reserve` is a one-time full file repack, preserving compressed audio packets.
Normal tag editing never silently falls back to a full rewrite. The same commands
work on Opus files created by other tools, subject to available header capacity.

Updates use an exclusive file lock and a small adjacent recovery journal. If a
process or machine dies during a write, run `opusab tags recover book.opus` to
restore the previous metadata. Keep the `.opusab-journal` file with the audiobook
until recovery finishes. Readers that ignore the lock can briefly observe an
in-progress header; fully atomic whole-file replacement would violate the fast
edit requirement. Single-stream Ogg Opus is supported; chained or multiplexed
streams must first be converted to one logical stream.

## Encoding defaults

| Preset | Suggested bitrate | Intended use |
| --- | --- | --- |
| `speech` (default) | 32 kbit/s VBR | Narration |
| `speech`, low-bandwidth source | 24 kbit/s VBR | Sources at ≤24 kHz, or MP3 at ≤64 kbit/s |
| `compact` | 24 kbit/s VBR | Smaller narration files |
| `mixed` | 64 kbit/s VBR | Speech with music or stereo effects |

These are editable starting points, not equivalent-quality guarantees. Lossy
transcoding cannot recover source detail. Use `--bitrate 40` to override. Original
channels are kept by default; `--channels mono` or `stereo` performs an additional
lossless preparation pass. This can require substantial temporary disk space.
Worker count defaults to up to eight available cores; `--jobs 1` disables the
SuperFast parallel path. Encoding uses 20 ms frames and complexity 10.

Chapter offsets retain millisecond precision. Source duration estimates can
include codec priming or padding, so generated file boundaries can differ slightly
from the decoded audio. The tool deliberately does not use fre:ac 1.1.7's generated
whole-second chapter offsets, which accumulate rounding error.

The full-book results and verification details are in [docs/VALIDATION.md](docs/VALIDATION.md).

## Build and distribution

```sh
cargo build --release --locked
cargo test --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
```

Use a current stable Rust toolchain. Metadata inspection and editing need only the
`opusab` executable. Conversion needs `freaccmd` and `ffprobe`; explicit channel
conversion and artwork extraction also use `ffmpeg`.

Discovery checks `OPUSAB_FREACCMD`, `OPUSAB_FFPROBE`, and `OPUSAB_FFMPEG`, then
`OPUSAB_RUNTIME`, a `runtime` directory beside the executable, and PATH. On macOS
it also checks `/Applications/freac.app` and common Homebrew paths. `doctor` shows
the exact tools selected.

For a portable macOS package, install the official fre:ac app, then run:

```sh
scripts/build-ffmpeg.sh
scripts/package-macos.sh
```

This builds small FFmpeg/ffprobe executables from pinned, checksummed source and
packages them with the intact fre:ac runtime. The result is a relocatable directory
and `.tar.gz` under `dist/`, with no Homebrew dependency at runtime. Keep its
`runtime` directory beside `opusab`. Run the included `install.sh` to install into
`~/.local`, or pass a different prefix. It refuses to replace existing installs.

This first distribution targets the host macOS architecture and is a local
unsigned development bundle. It is not a single static binary. Public releases
still need signing/notarization and complete corresponding-source/license
packaging; see [third-party notes](docs/THIRD_PARTY.md). Linux and Windows builds
have CI configuration but their runtime packages have not been validated here.

Phone support for Opus, Vorbis-style chapter tags, and cover art depends on the
player. Playback and chapter navigation on a physical phone have not been tested.

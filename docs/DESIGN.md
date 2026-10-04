# Design and CLI contract

## Boundaries

`model` describes a book, ordered sources, millisecond chapters, and editable tags.
`probe` discovers files and reads stream/container metadata. `app` builds a plan and
executes it. `cli` and `tui` share those operations. `backend` supervises child
processes; `ogg` owns the file-format code for fast metadata edits.

Rust provides the distributable command, strict argument handling, structured
errors, a native TUI, and predictable file I/O. Encoding remains in the existing
fre:ac/BoCA implementation. No codec or parallel packet stitching was implemented
here. A macOS runtime bundle is deliberately preferred to embedding all of fre:ac
as a library or promising a fully static cross-platform binary.

## Conversion

1. Canonicalize inputs. A directory is recursively scanned, naturally sorted, then
   ordered by disc/track if every file has track metadata. Explicit file arguments
   keep their order. Duplicates fail.
2. Probe metadata, channels, bitrate, duration, chapters, and artwork. Existing
   Ogg Opus files use the native header/tail reader; other formats use ffprobe.
3. Merge audiobook metadata and build file chapters. Apply the caller's patch,
   validate the result, and select a suggested bitrate. `plan` exposes this state.
4. A single existing Opus file uses metadata-only editing by default. An alternate
   output copies its compressed audio. Explicit bitrate/channel/preset changes or
   `--reencode` request an encode, always to a different path from the input.
5. Run freaccmd with `-e opus --superfast --threads=N`, VBR bitrate, complexity 10,
   and 20 ms frames. For `N=1`, omit `--superfast`: this backend treats ≤1 in that
   mode as automatic worker selection. The single M4B uses the same parallel driver
   as multi-file input. We don't load a user's fre:ac settings profile.
6. An explicit channel change first decodes/resamples to lossless FLAC temporary
   files. If needed, ffmpeg extracts artwork that would otherwise be lost in that
   preparation. The default preserves channels and avoids this pass.
7. Preserve our source/manual chapter offsets. fre:ac 1.1.7's generated chapter
   tags lose fractional seconds cumulatively; they are not a reliable replacement.
   Container duration estimates can still include priming/padding. In the real MP3
   fixture the aggregate discrepancy was 78 ms over 8.5 hours.
8. Write final tags and reserved space into a temporary padded Opus file. Copy
   compressed packets, adjust Ogg page sequence numbers and CRCs, and check every
   copied page. Sync and atomically install the final file. Keep sources intact.

Child stdout/stderr are drained concurrently. `--json` owns stdout exclusively;
backend chatter never leaks into the protocol. Ctrl+C terminates the whole child
process group on Unix, waits briefly, and escalates if needed. RAII removes
unfinished outputs. Source/output alias checks and no-clobber persistence guard
against accidental replacement.

## In-place OpusTags editing

The output is one ordinary Ogg Opus logical stream, not a concatenation of separate
Opus streams. The identification header and comment header occupy their own pages.
The OpusTags packet has 256 KiB of trailing reserve by default, permitted by
[RFC 7845 §5.2](https://www.rfc-editor.org/rfc/rfc7845#section-5.2).

An edit reads the header and at most 128 KiB at the tail for the duration. It
serializes the changed comments into exactly the old packet capacity, pads the
unused portion, reuses the original page boundaries, and recalculates CRCs. Only
pages with different bytes are written. Consequently the file length, identity,
page sequence numbers, audio offsets, and all audio bytes remain unchanged. The
cost depends on metadata size, not audiobook duration. Changing a text length can
shift embedded artwork within the header, so more than one page may change.

Repeated and unknown tags survive. Known aliases are consolidated only when that
field is edited. Opaque non-padding suffix bytes required by RFC 7845 are retained.
Limits: 16 MiB comment packet, 8 MiB supplied cover, up to 1,000 chapter tags.
`CHAPTER000` through `CHAPTER999` use `HH:MM:SS.mmm`; names use the matching `NAME`
tag. Artwork uses the standard base64 FLAC `METADATA_BLOCK_PICTURE` structure.

A capacity shortfall returns `metadata_capacity` before any write. `tags reserve`
is an explicit full file repack and can replace an existing file atomically; it
preserves the compressed packet payloads but may alter audio page headers/offsets.
It is never a hidden fallback for an ordinary metadata edit.

## Interrupted writes

In-place updates cannot give unaware readers an atomic view of all changed pages.
We use an exclusive advisory lock plus recovery:

1. Verify the opened file still matches the path; reject a pending journal.
2. Build and validate every changed page in memory.
3. Write original/replacement pages, offsets, file identity/size, and OpusHead hash
   to a temporary journal. Sync and atomically publish it beside the audiobook.
4. Write changed metadata pages, then sync the audiobook.
5. Remove the journal and sync the parent directory.

If a journal remains, normal edits stop. `tags recover` locks and checks the file
identity, header hash, lengths, ranges, and original CRCs, then restores the old
pages even if the current header contains a torn write. An interruption after all
pages were written but before journal removal still rolls back on recovery. No
whole audiobook backup is required. Locks coordinate opusab processes; other
programs must also cooperate if they edit concurrently.

## JSON interface, version 1

Pass `--json` globally or after any command. Stdout contains one JSON document on
success. Stderr contains newline-delimited progress events and, on failure, one
final error. `--quiet` removes progress events. All documents include
`schema_version: 1`. Agents should select fields by name and accept added fields.

| Command | Result |
| --- | --- |
| `inspect SOURCES...` | Book: ordered `sources`, `metadata`, `duration_ms`, warnings |
| `plan SOURCES...` | Operation, output path, book, recommendation, target settings |
| `convert SOURCES...` | Operation, final path, duration, size, elapsed time, metadata I/O report |
| `tags show FILE` | Editable `metadata`, cover presence, capacity and available bytes |
| `tags set/apply/reserve/recover` | `result` containing metadata I/O report |
| `doctor` | `ready` and exact tool paths |

Example progress event:

```json
{"schema_version":1,"event":"progress","stage":"encoding","message":"Encoding audiobook","elapsed_seconds":3.0}
```

Example failure:

```json
{"schema_version":1,"error":{"code":"metadata_capacity","message":"Metadata needs more header space…"}}
```

Specific error codes include `invalid_arguments`, `terminal_required`,
`backend_missing`, `backend_failed`, `probe_failed`, `file_busy`,
`metadata_capacity`, `invalid_chapters`, `invalid_opus`, `output_exists`, and
`cancelled`. Other validation or I/O failures use `operation_failed`; callers
must not assume every failure has a specialized code. Exit codes: 0 success,
1 operational failure, 2 argument syntax, 130 cancellation.

A metadata patch is strict within `metadata`; unknown field names are errors.
Export envelopes may carry informational fields (capacity, cover presence, etc.).
Omitted/null fields preserve data. Empty strings remove text, empty chapter arrays
remove chapters, and `remove_cover` is explicit. `extra` patches named keys, rather
than replacing all unknown tags. Dedicated fields take care of managed aliases.
The CLI does not require an agent SDK, MCP server, or interactive confirmation.

## Validation and distribution

`cargo test --all-targets` exercises metadata invariants, Ogg lacing boundaries,
recovery after a torn write, lock conflicts, malformed inputs, JSON behavior,
chapter precision, runtime discovery via symlinks, Unicode edits, and terminal
sizes. `cargo test --test backend -- --ignored` additionally runs a two-worker
fre:ac conversion, including filenames with spaces/apostrophes/Unicode and two
subsecond tracks. `cargo clippy --all-targets -- -D warnings` is clean.

The checked-in tone fixture is a generated 250 ms 440 Hz sine, encoded as Opus; it
contains no user audio. Real audiobook fixtures and generated books remain ignored
under `.local/`; they are never included in a source or binary package.

`build-ffmpeg.sh` downloads a pinned SHA-256 verified source archive and builds a
small static-library FFmpeg/ffprobe pair. `package-macos.sh` preserves the official
fre:ac bundle layout and includes the Rust executable, runtime, installer, docs,
and FFmpeg source/license materials. Its dependency audit rejects references to
Homebrew or other developer-only library paths. Runtime lookup canonicalizes the
executable, including installations through a symlink.

The macOS builds target macOS 11 but have only been exercised on the development
host. The local package is unsigned/unnotarized; public release preparation is
separate, including corresponding source for all bundled native components.
Physical phone players and Windows/Linux runtime packages remain unverified.

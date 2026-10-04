# Local validation — 2026-10-04

Tested on the development Mac (Apple Silicon), using fre:ac 1.1.7. These are local
results, not claims of cross-platform or phone-player certification.

## Complete recordings

| Input | Input audio | Output | Duration | Chapters | Conversion time |
| --- | ---: | ---: | --- | ---: | ---: |
| Neuromancer, one M4B | 256.0 MiB | 91.8 MiB | 9:23:44.718 | 24 | 77.3 s |
| A Farewell to Arms, seven MP3s | 469.9 MiB | 110.2 MiB | 8:33:15.282 | 7 | 47.8 s |

The jobs overlapped and each requested six workers; these timings are not isolated
benchmarks. Defaults selected 24 kbit/s for the 22.05 kHz M4B and 32 kbit/s for the
128 kbit/s MP3s. Opus VBR output size differs from a constant-bitrate estimate.

An independent ffprobe recognized all 24/7 chapters and embedded covers.
Both complete outputs were decoded through the bundled FFmpeg with exit 0 and no
diagnostics. Covers match their source image bytes exactly. M4B chapter starts
match the original 24 entries. MP3 chapters follow the cumulative source durations
at millisecond resolution. The generated recordings are 69 ms and 78 ms shorter
than the respective source container duration estimates; codec priming/padding and
decoder duration accounting can differ. No audible boundary/quality assessment or
physical phone playback was performed.

SHA-256 verification confirms the original M4B, seven MP3s, and JPEG were unchanged.
The source audio is not checked into the repository or included in packages.

## Metadata and interface checks

A title edit on each full output took about 20–24 ms including process startup.
Each edited two header pages (130,614 bytes) and zero audio bytes. File size,
inode, audio offset, and the SHA-256 of all bytes after the metadata header remained
unchanged. Original titles were restored afterward. Actual changed-page count
varies with the edit and artwork position.

Automated tests cover insufficient capacity without any writes, padding/page
boundary cases, byte-identical audio preservation, unknown and repeated tags,
interrupted-write recovery, concurrent edit rejection, and truncated/corrupt data.
CLI tests cover strict JSON, noninteractive errors, avoiding accidental re-encode,
source overwrite rejection, chapter precision, and installed runtime discovery.
A real fre:ac test merges two 250 ms tracks with two workers and retains the 250 ms
chapter boundary and a custom name.

The TUI was exercised through an actual pseudoterminal: open, edit a Unicode title,
save in place, and exit. Render tests cover three terminal sizes. A full-book encode
was interrupted with SIGINT: exit 130, structured cancellation error, no completed
output, and no residual temporary conversion directory.

## Runtime package

The portable archive includes the application, intact fre:ac app runtime, minimal
FFmpeg/ffprobe, installation script, and documentation. Native dependency checks
reject Homebrew paths. Bundled tools and installation through a symlink are checked
with a restricted PATH. A conversion through the installed command also exercised
mono channel preparation and embedded-cover extraction. The macOS deployment target is 11.0; execution on older
macOS versions has not been tested. Signing, notarization, and full public-release
source/license assembly remain release work.

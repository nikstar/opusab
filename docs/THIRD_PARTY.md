# Runtime components

The Rust application is MIT licensed. Its audio tools run as separate processes
and retain their respective licenses.

* **fre:ac 1.1.7**, BoCA, smooth, and the codecs in the official macOS bundle:
  https://www.freac.org/ and https://github.com/enzo1982/freac/tree/v1.1.7.
  fre:ac is GPL-2.0-or-later. The complete upstream application bundle is preserved,
  including its manual, component information, and notices. Source and development
  kit downloads: https://www.freac.org/downloads-mainmenu-33.
* **FFmpeg 9.0.2**, built with the checked-in `build-ffmpeg.sh`: LGPL-2.1-or-later.
  This build enables no GPL or nonfree components. License texts, the complete
  unmodified source archive, configure log, and build script accompany the local
  package. https://ffmpeg.org/download.html and https://ffmpeg.org/legal.html.
* Rust dependency versions are pinned in `Cargo.lock`; their individual licenses
  remain in their upstream crates.

`scripts/package-macos.sh` creates a local development bundle. It does not publish,
sign with a Developer ID, or notarize a release. A public distributor must also
provide the corresponding source and notices for the exact fre:ac/BoCA/smooth/
codec binaries being distributed, and the notices for Rust dependencies. Do not
assume that linking an upstream download page alone satisfies those obligations.

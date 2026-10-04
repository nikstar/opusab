CARGO ?= cargo
prefix ?= $(HOME)/.local

build:
	$(CARGO) build --release --locked

check:
	$(CARGO) fmt --check
	$(CARGO) clippy --all-targets --locked -- -D warnings
	$(CARGO) test --all-targets --locked

runtime:
	scripts/build-ffmpeg.sh

package:
	scripts/package-macos.sh

# Installs the CLI only; use install.sh in the portable package for a bundled install.
install: build
	install -d "$(prefix)/bin"
	install target/release/opusab "$(prefix)/bin/opusab"

clean:
	$(CARGO) clean

.PHONY: build check runtime package install clean

prefix ?= /usr/local
bindir = $(prefix)/bin

build:
	swift build -c release --disable-sandbox

install: build
	install ".build/release/opusab" "$(bindir)"

uninstall:
	rm -rf "$(bindir)/opusab"

clean:
	rm -rf .build

.PHONY: build install uninstall clean

# Opusab

Create Opus audiobooks from a folder of mp3 files. Opus offers ~4x smaller file sizes compared to mp3 without losing in quality. 

```
OVERVIEW: Create Opus audiobooks from a list of mp3 files

USAGE: opusab <options> files...

OPTIONS:
  --bitrate, -b   bitrate in kbits (default 32)
  --cover         path to the cover file
  --dry-run, -n   print command but do not execute it
  --output, -o    name of the output file
  --verbose, -v   verbose output
  --help          Display available options

POSITIONAL ARGUMENTS:
  files           audio files
```

**This is work in progress. Suggestions and PRs are very welcome!**

## Installation 
```
brew install nikstar/tap/opusab --HEAD
```
*Please use the up-to-date `--HEAD` version until **opusab** reaches 1.0*

## Bitrates
+ 48 kbit/s or higher should only be used for high quality sources with occasional background music
+ 32 kbits/s is a **good default** that should be used to convert, for example, 120 kbits/s mp3s
+ 24 kbit/s offers good quality and I personally cannot tell difference from 32 kbit/s, but you might :)
+ 16 kbit/s is perfectly intelligible and gives you tiny files at just **7.2 MB per hour**


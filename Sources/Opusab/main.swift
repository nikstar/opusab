import os
import OpusabCore

let tool = try Opusab(arguments: [".build/debug/Opusab", "-o", "test.opus", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/01 Chapter 01 - Dudley Demented.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/02 Chapter 02 - A Peck of Owls.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/03 Chapter 03 - The Advance Guard.mp3"])

do {
    try tool.run()
} catch {
    print("program returned error: \(error)")
    exit(1)
}

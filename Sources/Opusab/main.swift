import os
import OpusabCore


let short =  [".build/debug/Opusab", "-o", "test.opus", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/01 Chapter 01 - Dudley Demented.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/02 Chapter 02 - A Peck of Owls.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/03 Chapter 03 - The Advance Guard.mp3"]
let tool = try Opusab(arguments: [".build/debug/Opusab", "-o", "test.opus", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/01 Chapter 01 - Dudley Demented.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/02 Chapter 02 - A Peck of Owls.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/03 Chapter 03 - The Advance Guard.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/04 Chapter 04 - Number Twelve Grimmauld Place.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/05 Chapter 05 - The Order of the Phoenix.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/06 Chapter 06 - The Noble and Most Ancient House of Black.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/07 Chapter 07 - The Ministry of Magic.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/08 Chapter 08 - The Hearing.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/09 Chapter 09 - The Woes of Mrs. Weasley.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/10 Chapter 10 - Luna Lovegood.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/11 Chapter 11 - The Sorting Hat\'s New Song.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/12 Chapter 12 - Professor Umbridge.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/13 Chapter 13 - Detention With Dolores.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/14 Chapter 14 - Percy and Padfoot.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/15 Chapter 15 - The Hogwarts High Inquisitor.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/16 Chapter 16 -In The Hog\'s Head.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/17 Chapter 17 - Educational Decree Number Twenty-Four.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/18 Chapter 18 - Dumbledore\'s Army.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/19 Chapter 19 - The Lion and the Serpent.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/20 Chapter 20 - Hagrid\'s Tale.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/21 Chapter 21 - The Eye of the Snake.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/22 Chapter 22 - St. Mungo\'s Hospital for Magical Maladies and Injuries.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/23 Chapter 23 - Christmas on the Closed Ward.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/24 Chapter 24 - Occlumency.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/25 Chapter 25 - The Beetle at Bay.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/26 Chapter 26 - Seen and Unforeseen.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/27 Chapter 27 - The Centaur and the Sneak.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/28 Chapter 28 - Snape\'s Worst Memory.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/29 Chapter 29 - Career Advice.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/30 Chapter 30 - Grawp.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/31 Chapter 31 - O.W.L.S.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/32 Chapter 32 - Out of the Fire.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/33 Chapter 33 - Fight and Flight.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/34 Chapter 34 - The Department of Mysteries.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/35 Chapter 35 - Beyond the Veil.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/36 Chapter 36 - The Only One He Ever Feared.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/37 Chapter 37 - The Lost Prophecy.mp3", "/Volumes/Green/Audiobooks/Harry Potter (Stephen Fry)/5/38 Chapter 38 - The Second War Begins.mp3"])

do {
    try tool.run()
} catch {
    print("program returned error: \(error)")
    exit(1)
}

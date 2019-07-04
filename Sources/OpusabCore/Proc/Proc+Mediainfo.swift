import Files
import Foundation
import Proc

struct MediainfoMetadata {
    var trackName: String
    var album: String
    var performer: String
}

extension Proc {
    static func mediainfo(_ filename: String) throws -> MediainfoMetadata {
        let string = try Proc("/usr/local/bin/mediainfo", filename)
            .runForStdout()
        
        //         output example
        //         ...
        //             Album                                    : LotR Part II: The Two Towers
        //             Track name                               : 4.10 The Choices of Master Samwise
        //             Performer                                : J.R.R. Tolkien
        //         ...
        
        let trackName = extract(tag: "Track name", from: string) ?? (try! File(path: filename).nameExcludingExtension)
        let album = extract(tag: "Album", from: string)
        let author = extract(tag: "Performer", from: string)
        
        return MediainfoMetadata(trackName: trackName, album: album!, performer: author!)
    }

    fileprivate static func extract(tag: String, from string: String) -> String? {
        if let result = string
            .split(separator: "\n")
            .first(where: { $0.hasPrefix(tag) })?
            .split(separator: ":", maxSplits: 1)[1]
            .dropFirst() {
            return String(result)
        }
        return nil
    }
}

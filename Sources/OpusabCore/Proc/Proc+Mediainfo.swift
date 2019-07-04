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
        let data = try Proc("/usr/local/bin/mediainfo", "--Output=JSON", filename)
            .runForStdout()
            .data(using: .utf8)!
        let metadata = try! JSONDecoder().decode(Mediainfo.self, from: data)
        
        let trackName = metadata.media.track[0].Track ?? (try! File(path: filename).nameExcludingExtension)
        let album = metadata.media.track[0].Album!
        let author = metadata.media.track[0].Performer!
        
        return MediainfoMetadata(trackName: trackName, album: album, performer: author)
    }
}

fileprivate struct Mediainfo: Decodable {
    var media: Media
    
    struct Media: Decodable {
        var track: [Track]
        
        struct Track: Decodable {
            var Track: String?
            var Album: String?
            var Performer: String?
        }
    }
}

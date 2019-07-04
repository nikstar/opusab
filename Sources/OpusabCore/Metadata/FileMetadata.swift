import Files
import Foundation
import Proc

struct FileMetadata {
    let filename: String
    var name: String
    let album: String
    let author: String
    let duration: Double
}

extension FileMetadata {
    init(file: String) throws {
        let m = try Proc.mediainfo(file)
        let duration = Proc.ffmpeg_accurateDuration(for: file)
        self.init(filename: file, name: m.trackName, album: m.album, author: m.performer, duration: duration)
    }
}

extension Array where Element == FileMetadata {
    mutating func stripCommonPrefix() {
        guard count > 1 else { return }
        let prefix = self.map { $0.name }.commonPrefix
        if prefix.count > 0 {
            for i in 0..<count {
                self[i].name = String(self[i].name.dropFirst(prefix.count))
                if self[i].name == "" {
                    self[i].name = String(format: "%03d", i+1)
                }
            }
        }
    }
}

fileprivate extension Array where Element == String {
    var commonPrefix: String {
        guard count > 0 else { return "" }
        var prefix = self[0]
        loop: while prefix.count > 0 {
            for s in self.dropFirst() {
                if !s.hasPrefix(prefix) { prefix = String(prefix.dropLast()); continue loop }
            }
            break
        }
        return prefix
    }
}


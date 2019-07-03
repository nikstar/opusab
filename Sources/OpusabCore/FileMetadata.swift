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
        let m = try mediainfo(with: file)
        let duration = ffmpeg(with: file)
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

// MARK: - mediainfo
struct MediainfoMetadata {
    var trackName: String
    var album: String
    var performer: String
}

fileprivate func mediainfo(with filename: String) throws -> MediainfoMetadata {
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

fileprivate func extract(tag: String, from string: String) -> String? {
    if let result = string
        .split(separator: "\n")
        .first(where: { $0.hasPrefix(tag) })?
        .split(separator: ":", maxSplits: 1)[1]
        .dropFirst() {
//        if verbose { print(result) }
        return String(result)
    }
    return nil
}

// MARK: - ffmpeg
fileprivate func ffmpeg(with filename: String) -> Double {
    let string = try! Proc("/bin/bash", "-c", "/usr/local/bin/ffmpeg -nostats -hide_banner -nostdin -i \"\(filename)\" -f null /dev/null 2>&1")
        .runForStdout()
        .split(separator: " ")
        .first(where: { $0.hasPrefix("time=")})!
        .dropFirst(5)
    let duration = time(from: String(string))
    return duration
}

private func time(from string: String) -> Double {
    let t = string.split(separator: ":")
    let h = Double(t[0])!
    let m = Double(t[1])!
    let s = Double(t[2])!
    return h * 3600 + m * 60 + s
}



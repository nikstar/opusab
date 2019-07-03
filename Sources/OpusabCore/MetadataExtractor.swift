import Files
import Foundation
import Proc

public final class MetadataExtractor {
    let filenames: [String]
    let verbose: Bool
    
    init(filenames: [String], verbose: Bool) {
        self.filenames = filenames
        self.verbose = verbose
        print(self, verbose)
    }
    
    func gather() throws -> [FileMetadata] {
        let p = ProgressReporter(description: "Getting metadata", steps: filenames.count, verbose: verbose)
        defer { p.terminate() }
        var m = try filenames.map { (filename) -> FileMetadata in
            p.nextStep(description: try! File(path: filename).name)
            let metadata = try self.getMetadata(for: filename)
            if verbose { print(metadata); print() }
            return metadata
        }
        if m.count > 1 { // remove prefixes
            let prefix = commonPrefix(m.map { $0.name })
            print("common prefix: ", prefix)
            if prefix.count > 0 {
                for i in 0..<m.count {
                    if verbose {
                        print(m[i])
                    }
                    m[i].name = String(m[i].name.dropFirst(prefix.count))
                    if m[i].name == "" {
                        m[i].name = String(format: "%03d", i+1)
                    }
                }
            }
        }
        if verbose {
            print(m)
        }
        return m
    }
    
    private func getMetadata(for filename: String) throws -> FileMetadata {
        let m = try mediainfo(with: filename)
        let duration = ffmpeg(with: filename)
        return FileMetadata(filename: filename,
            name: m.trackName, album: m.album, author: m.performer,
            duration: duration)
    }
}

struct MediainfoMetadata {
    var trackName: String
    var album: String
    var performer: String
}

extension MetadataExtractor { // mediainfo
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
    
    private func extract(tag: String, from string: String) -> String? {
        if verbose { print(tag, terminator: "=>") }
        if let result = string
            .split(separator: "\n")
            .first(where: { $0.hasPrefix(tag) })?
            .split(separator: ":", maxSplits: 1)[1]
        .dropFirst() {
            if verbose { print(result) }
            return String(result)
        }
        if verbose { print("nil") }
        return nil
    }
}


extension MetadataExtractor { // ffmpeg
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
}


struct FileMetadata {
    let filename: String
    var name: String
    let album: String
    let author: String
    let duration: Double
}


func commonPrefix(_ strs: [String]) -> String {
    guard strs.count > 0 else { return "" }
    var prefix = strs[0]
    loop: while prefix.count > 0 {
        for s in strs.dropFirst() {
            print(prefix, s)
            if !s.hasPrefix(prefix) { prefix = String(prefix.dropLast()); continue loop }
        }
        break
    }
    return prefix
}

import Files
import Foundation

public final class MetadataExtractor {
    let filenames: [String]
    
    init(filenames: [String]) {
        self.filenames = filenames
    }
    
    func gather() throws -> [FileMetadata] {
        return try filenames.map { (filename) -> FileMetadata in
            print("Getting metadata for \(filename)")
            return try self.getMetadata(for: filename)
        }
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
        let string = Process.standardOutput(forProcessWithArguments:
            ["/usr/local/bin/mediainfo", filename])
        
        /* output example
             General
             Complete name                            : /Volumes/Green/Audiobooks/LOTR/4.10 The Choices of Master Samwise.mp3
             Format                                   : MPEG Audio
             File size                                : 7.93 MiB
             Duration                                 : 46 min 9 s
             Overall bit rate mode                    : Constant
             Overall bit rate                         : 24.0 kb/s
             Album                                    : LotR Part II: The Two Towers
             Track name                               : 4.10 The Choices of Master Samwise
             Performer                                : J.R.R. Tolkien
             Genre                                    : Books & Spoken
         
             Audio
             Format                                   : MPEG Audio
             Format version                           : Version 2
             Format profile                           : Layer 3
             Duration                                 : 46 min 9 s
             Bit rate mode                            : Constant
             Bit rate                                 : 24.0 kb/s
             Channel(s)                               : 1 channel
             Sampling rate                            : 22.05 kHz
             Frame rate                               : 38.281 FPS (576 SPF)
             Compression mode                         : Lossy
             Stream size                              : 7.92 MiB (100%)
         */
        
        let trackName = extract(tag: "Track name", from: string)
        let album = extract(tag: "Album", from: string)
        let author = extract(tag: "Performer", from: string)
        
        return MediainfoMetadata(trackName: trackName, album: album, performer: author)
    }
    
    private func extract(tag: String, from string: String) -> String {
        let result = string
            .split(separator: "\n")
            .first { $0.hasPrefix(tag) }!
            .split(separator: ":", maxSplits: 2)[1]
            .dropFirst()
        return String(result)
    }
}


extension MetadataExtractor { // ffmpeg
    fileprivate func ffmpeg(with filename: String) -> Double {
        let string = Process.standardOutput(forProcessWithArguments: [
            "/bin/bash", "-c",
            "/usr/local/bin/ffmpeg -nostats -hide_banner -nostdin -i \"\(filename)\" -f null /dev/null 2>&1"])
            .lazy
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
    let name: String
    let album: String
    let author: String
    let duration: Double
}


public extension Process {
    public static func standardOutput(forProcessWithArguments args: [String]) -> String {
        let process = Process()
        process.launchPath = args[0]
        process.arguments = Array(args.dropFirst())
        print(args.map { "'\($0)'" }.joined(separator: " "))
        let output = Pipe()
        process.standardOutput = output
        process.launch()
        let data = output.fileHandleForReading.readDataToEndOfFile()
        
        return String(data: data, encoding: .utf8)!
    }
}

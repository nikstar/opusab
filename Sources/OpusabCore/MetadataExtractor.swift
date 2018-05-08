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
        let (trackName, album, author, rawDuration, rawBitrate) = try mediainfo(with: filename)
        let duration = ffmpeg(with: filename)
        return FileMetadata(filename: filename, name: trackName, album: album, author: author, duration: duration, bitrate: 0)
    }
}


extension MetadataExtractor { // mediainfo
    fileprivate func mediainfo(with filename: String) throws -> (String, String, String, String, String) {
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
        let bitrate = extract(tag: "Bit rate", from: string)
        let duration = extract(tag: "Duration", from: string)
        print(trackName, album, author, duration, bitrate)
        return (trackName, album, author, duration, bitrate)
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
    fileprivate func ffmpeg(with filename: String) -> Int {
        let string = Process.standardOutput(forProcessWithArguments: [
            "/bin/bash", "-c",
            "ffmpeg -nostats -hide_banner -nostdin -i \"\(filename)\" -f null /dev/null 2>&1 | grep 'time=' | egrep -o '\\d\\d:\\d\\d:\\d\\d.\\d\\d'"])
        let duration = time(from: string)
        return duration
    }
    
    private func time(from string: String) -> Int {
        let t = string.split(separator: ":")
        
        let h = Int(t[0])!
        let m = Int(t[1])!
        
        let t2 = t[2]
            .replacingOccurrences(of: "\n", with: "")
            .split(separator: ".")
        let s = Int(t2[0])!
        let ms = Int(t2[1])! * 10
        
        print(h, m, s, ms)
        return ((((h * 60) + m) * 60) + s) * 1000 + ms
    }
}


struct FileMetadata {
    let filename: String
    let name: String
    let album: String
    let author: String
    let duration: Int
    let bitrate: Int
}


public extension Process {
    public static func standardOutput(forProcessWithArguments args: [String]) -> String {
        let process = Process()
        process.launchPath = args[0]
        process.arguments = Array(args.dropFirst())
        let output = Pipe()
        process.standardOutput = output
        process.launch()
        let data = output.fileHandleForReading.readDataToEndOfFile()
        return String(data: data, encoding: .utf8)!
    }
}



extension String {
    subscript(from start: Int, to end: Int) -> Substring {
        let startIdx, endIdx: String.Index
        if start >= 0 {
            startIdx = index(startIndex, offsetBy: start)
        } else {
            startIdx = index(endIndex, offsetBy: start)
        }
        if end > 0 {
            endIdx = index(startIndex, offsetBy: end)
        } else {
            endIdx =  index(endIndex, offsetBy: end)
        }
        return self[startIdx..<endIdx]
    }
}




















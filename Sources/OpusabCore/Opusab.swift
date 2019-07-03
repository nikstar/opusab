import Files
import Foundation
import CLInterface
import Proc

public final class Opusab: CLInterface {
    public var description = "Create Opus audiobooks from a list of mp3 files"
    public var optionsString = "<options> files..."
    
    @Argument("--output", "-o", usage: "name of the output file")
    var outputPath: String!
    
    @Argument("--cover", usage: "path to the cover file")
    var coverPath: String?
    
    @PositionalArguments(name: "files", usage: "audio files")
    var audioFiles: [String]
    
    // TODO: add default value to description
    @Argument("--bitrate", "-b", usage: "bitrate in kbits (default 32)", default: 32)
    var bitrate: Int!
    
    @Argument("--verbose", "-v", usage: "verbose output", default: false)
    var verbose: Bool!
    
    @Argument("--dry-run", "-n", usage: "print command but do not execute it", default: false)
    var dryRun: Bool!
    
    public init(arguments: [String]? = nil) throws {
        try parseArgs(arguments)
    }
    
    private func parseArgs(_ arguments: [String]?) throws {
        let arguments = arguments ?? Array(CommandLine.arguments.dropFirst())
        print(arguments)
        
        try parseArguments(arguments)
        print(outputPath as Any, coverPath as Any, audioFiles, bitrate!, verbose!, dryRun!)

        // verify files exist
        try self.audioFiles.forEach { _ = try File(path: $0) }
        if let coverPath = coverPath {
            _ = try File(path: coverPath)
        }
    }
    
    public func run() throws {
        let metadata = MetadataExtractor(filenames: audioFiles, verbose: verbose!)
        let filesMetadata = try metadata.gather()
        
        let ffmpegArgs = "-hide_banner -nostdin -loglevel fatal -nostats -f mp3 -i pipe:0 -f wav -"
            .split(separator: " ")
            .map(String.init)
        
        let opusencArgs = Converter().generateCommand(filesMetadata: filesMetadata, output: outputPath!, bitrate: bitrate!, cover: coverPath)
        
        let cat = Proc("/bin/cat", audioFiles)
        let ffmpeg = Proc("/usr/local/bin/ffmpeg", ffmpegArgs)
        let opusenc = Proc("/usr/local/bin/opusenc", opusencArgs)
        
        let p = cat.pipe(to: ffmpeg).pipe(to: opusenc)
        print(p)
        
        if dryRun! {
            return
        }
        
        try p.run()
        p.waitUntilExit()
    }
}

public extension Opusab {
    enum Error: Swift.Error {
        case missingFileName
        case failedToCreateFile
    }
}

extension Process {
    convenience init(_ arguments: [String]) {
        self.init()
        self.launchPath = arguments.first!
        self.arguments = Array(arguments.dropFirst())
    }
    
    class func bash(_ command: String) -> Process {
        return Process(["/usr/bin/env", "bash", "-c", command])
    }
    
    func getOutput() -> String {
        let output = Pipe()
        self.standardOutput = output
        self.launch()
        let data = output.fileHandleForReading.readDataToEndOfFile()
        return String(data: data, encoding: .utf8)!
    }
}


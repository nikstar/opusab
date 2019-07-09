import Files
import Foundation
import CLInterface
import Proc

public final class Opusab: CLInterface {
    public var description = "Create Opus audiobooks from a list of mp3 files"
    public var optionsString = "<options> files..."
    
    @Argument("--output", "-o", usage: "name of the output file")
    var outputPath: String
    
    @Argument("--cover", usage: "path to the cover file")
    var coverPath: String?
    
    @PositionalArguments(name: "files", usage: "audio files")
    var audioFiles: [String]
    
    @Argument("--bitrate", "-b", usage: "bitrate in kbits (default 32)", default: 32)
    var bitrate: Int
    
    @Argument("--verbose", "-v", usage: "verbose output", default: false)
    var verbose: Bool
    
    @Argument("--dry-run", "-n", usage: "print command but do not execute it", default: false)
    var dryRun: Bool
    
    public init(arguments: [String]? = nil) throws {
        let arguments = arguments ?? Array(CommandLine.arguments.dropFirst())
        try parseArguments(arguments)

        // verify files exist
        try audioFiles.forEach { _ = try File(path: $0) }
        try coverPath.flatMap { _ = try File(path: $0) }
    }
    
    public func run() throws {
        let metadata = try Metadata(files: audioFiles, verbose: verbose)
        print("""
            \(metadata.title) by \(metadata.author)
            \(metadata.chapters.count) chapters; total duration: \(String(time: metadata.totalDuration))
            """)
        
        let cat = Proc("/bin/cat", audioFiles)
        let ffmpeg = Proc.ffmpeg_mp3ToWav()
        let opusenc = Proc.opusenc(metadata: metadata, output: outputPath, bitrate: bitrate, cover: coverPath)
        
        let p = cat.pipe(to: ffmpeg).pipe(to: opusenc)
        if verbose || dryRun { print("final command: \(p)") }
        
        guard !dryRun else { return }
        try p.run()
        p.waitUntilExit()
    }
}

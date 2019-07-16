import Files
import Foundation
import CLInterface
import Proc

public final class Opusab: CLInterface {
    public var description = "Create Opus audiobooks from a list of mp3 files"
    public var optionsString = "<options> files..."
    
    @Argument("--output", "-o", usage: "name of the output file")
    var outputPath: String
    
    @Argument("--cover", "-c", usage: "path to the cover file")
    var coverPath: String?
    
    @PositionalArguments(name: "files", usage: "audio files")
    var audioFiles: [String]
    
    @Argument("--bitrate", "-b", usage: "bitrate in kbits (default 32)", default: 32)
    var bitrate: Int
    
    @Argument("--verbose", "-v", usage: "verbose output", default: false)
    var verbose: Bool
    
    @Argument("--print-metadata", "-p", usage: "print metadata to standard output", default: false)
    var printMetadata: Bool
    
    @Argument("--dry-run", "-n", usage: "do not execute final command", default: false)
    var dryRun: Bool
    
    @Argument("--metadata", "-m", usage: "json file, containing metadata (optional)")
    var metadataPath: String?
    
    public init(arguments: [String]? = nil) throws {
        let arguments = arguments ?? Array(CommandLine.arguments.dropFirst())
        try parseArguments(arguments)

        // verify files exist
        try audioFiles.forEach { _ = try File(path: $0) }
        try coverPath.map { _ = try File(path: $0) }
        try metadataPath.map { _ = try File(path: $0) }
    }
    
    public func run() throws {
        let metadata = try loadMetadataFromDisk() ?? Metadata(files: audioFiles, verbose: verbose)
        print(metadata.overview, to: &stderr)
        if printMetadata || verbose {
            let e = JSONEncoder()
            e.outputFormatting = .prettyPrinted
            let data = try e.encode(metadata)
            print(String(data: data, encoding: .utf8)!)
        }
        
        let cat = Proc("/bin/cat", audioFiles)
        let ffmpeg = Proc.ffmpeg_mp3ToWav()
        let opusenc = Proc.opusenc(metadata: metadata, output: outputPath, bitrate: bitrate, cover: coverPath)
        let p = cat.pipe(to: ffmpeg).pipe(to: opusenc)
        if dryRun || verbose {
            print("final command: \(p)", to: &stderr)
        }
        
        if !dryRun {
            try p.run()
            p.waitUntilExit()
        }
    }
    
    private func loadMetadataFromDisk() throws -> Metadata? {
        return try metadataPath.map { path in
                let url = URL(fileURLWithPath: path)
                let data = try Data(contentsOf: url)
                let d = JSONDecoder()
                return try d.decode(Metadata.self, from: data)
            }
    }
}

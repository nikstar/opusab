import Files
import Foundation
import Utility

public final class Opusab {
    var outputPath: String!
    var coverPath: String?
    var audioFiles: [String]!
    var bitrate: Int = 32
    var verbose: Bool = false
    var dryRun: Bool = false
    
    public init(arguments: [String] = CommandLine.arguments) throws {
        try parseArguments(arguments)
    }
    
    private func parseArguments(_ arguments: [String]) throws {
        let arguments = Array(arguments.dropFirst())
        
        let parser = ArgumentParser(
            usage: "<options> files...",
            overview: "Create Opus audobooks from a list of mp3 files")
        let audioFiles = parser.add(positional: "files", kind: [String].self,
            usage: "audio files",
            completion: .filename)
        let bitrate = parser.add(option: "--bitrate", shortName: "-b", kind: Int.self,
             usage: "bitrate in kbits (default 32)")
        let output = parser.add(option: "--output", shortName: "-o", kind: String.self,
             usage: "name of the output file")
        let cover = parser.add(option: "--cover", kind: String.self,
             usage: "path to the cover file (unimplemented)",
             completion: .filename)
        let verbose = parser.add(option: "--verbose", shortName: "-v", kind: Bool.self,
             usage: "verbose output")
        let dryRun = parser.add(option: "--dry-run", shortName: "-n", kind: Bool.self,
            usage: "print command but do not execute it")
        
        let parsedArguments = try parser.parse(arguments)
        print(parsedArguments)
        
        guard let outputPath = parsedArguments.get(output) else {
            throw ArgumentParserError.expectedArguments(parser, ["output"])
        }
        self.outputPath = outputPath
        self.coverPath = parsedArguments.get(cover)
        self.audioFiles = parsedArguments.get(audioFiles)!
        if let bitrate = parsedArguments.get(bitrate) {
            self.bitrate = bitrate
        }
        if let verbose = parsedArguments.get(verbose) {
            self.verbose = verbose
        }
        if let dryRun = parsedArguments.get(dryRun) {
            self.dryRun = dryRun
        }
        // verify files exist
        try self.audioFiles.forEach { _ = try File(path: $0) }
        if let coverPath = coverPath {
            _ = try File(path: coverPath)
        }
    }
    
    public func run() throws {
        let metadata = MetadataExtractor(filenames: audioFiles, verbose: verbose)
        let filesMetadata = try metadata.gather()
        
        let opusCommand = Converter().generateCommand(filesMetadata: filesMetadata, output: outputPath!, bitrate: bitrate, cover: coverPath)
        
        var comp = [
           "cat"
        ]
        comp.append(contentsOf: audioFiles.map { $0.shellEscaped() } )
        comp.append("|")
        comp.append("ffmpeg -hide_banner -nostdin -loglevel fatal -nostats -f mp3 -i pipe:0 -f wav -")
        comp.append("|")
        comp.append(opusCommand)
        
        let command  = comp.joined(separator: " ")
        print(command)
        
        if dryRun {
            return
        }
        
        let finalCommand = Process.bash(command)
        finalCommand.launch()
        finalCommand.waitUntilExit()
    }
}

extension FileHandle: TextOutputStream {
    public func write(_ string: String) {
        guard let data = string.data(using: .utf8) else { return }
        self.write(data)
    }
}

public extension Opusab {
    public enum Error: Swift.Error {
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


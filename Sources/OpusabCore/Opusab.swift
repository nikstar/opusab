import Files
import Foundation
import Utility

public final class Opusab {
    var outputPath: String? = nil
    var coverPath: String? = nil
    var audioFiles: [String]! = nil
    
    public init(arguments: [String] = CommandLine.arguments) throws {
        try parseArguments(arguments)
    }
    
    private func parseArguments(_ arguments: [String]) throws {
        print(CommandLine.arguments)
        
        let arguments = Array(arguments.dropFirst())
        
        let parser = ArgumentParser(
            usage: "<options> files...",
            overview: "Create Opus audobooks from a list of mp3 files")
        let audioFiles = parser.add(positional: "files", kind: [String].self,
            usage: "audio files",
            completion: .filename)
        
        let output = parser.add(option: "--output", shortName: "-o", kind: String.self,
            usage: "name of the output file",
            completion: .filename)
        let cover = parser.add(option: "--cover", kind: String.self,
           usage: "path to the cover file",
           completion: .filename)
        
        let parsedArguments = try parser.parse(arguments)
        print(parsedArguments)
        
        self.outputPath = parsedArguments.get(output)
        self.coverPath = parsedArguments.get(cover)
        self.audioFiles = parsedArguments.get(audioFiles)!
    }
    
    public func run() throws {
        let metadata = MetadataExtractor(filenames: audioFiles)
        let filesMetadata = try metadata.gather()
        print(filesMetadata)
        
        let opusArgs = Converter().generateCommand(filesMetadata: filesMetadata)
        print(opusArgs)
        
//        let p1 = Process()
//        p1.launchPath = "
        
    }
}

public extension Opusab {
    public enum Error: Swift.Error {
        case missingFileName
        case failedToCreateFile
    }
}

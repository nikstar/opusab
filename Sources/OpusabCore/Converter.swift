import Foundation

final class Converter {
    func generateCommand(filesMetadata: [FileMetadata]) -> [String] {
        var command: [String] = []
        command.append("opusenc")
        command.append(contentsOf: ["--bitrate", "32"])
        command.append("--downmix-mono")
        for (idx, m) in filesMetadata.enumerated() {
            command.append(contentsOf: ["--comment",
                "CHAPTER\(idx+1)=\(m.filename)"])
        }
        
        
        return command
    }
}

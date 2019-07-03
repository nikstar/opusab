import Foundation

final class Converter {
    func generateCommand(filesMetadata: [FileMetadata], output: String, bitrate: Int, cover: String?) -> [String] {
        precondition(filesMetadata.count > 0)
        
        var command: [String] = [
//            "opusenc",
            "--bitrate", "\(bitrate)",
            "--downmix-mono",
            "--comment", "title=\(filesMetadata[0].album)",
            "--comment", "artist=\(filesMetadata[0].author)",
            "--comment", "album=\(filesMetadata[0].album)",
        ]
        if let cover = cover {
            command.append(contentsOf: [
                "--picture", cover
            ])
        }
        var acc = 0.0
        for (idx, m) in filesMetadata.enumerated() {
            let time = timeString(acc: acc)
            acc += m.duration
            
            command.append(contentsOf: [
                "--comment", String(format: "CHAPTER%03d=\(time)", idx+1),
                "--comment", String(format: "CHAPTER%03dNAME=\(m.name)", idx+1)
            ])
            
            
        }
        command.append(contentsOf: ["-", output])
        return command
//            .map { $0.spm_shellEscaped() }
//            .joined(separator: " ")
    }
    
    private func timeString(acc: Double) -> String {
        let sec = acc.truncatingRemainder(dividingBy: 60.0)
        let min = Int(acc) / 60 % 60
        let h = Int(acc) / 3_600
        return String.init(format: "%02d:%02d:%06.3lf", h, min, sec)
    }
}

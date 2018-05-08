import Foundation

final class Converter {
    func generateCommand(filesMetadata: [FileMetadata]) -> [String] {
        var command: [String] = [
            "opusenc",
            "--bitrate", "32",
            "--downmix-mono",
            "--comment", "title=\(filesMetadata[0].album)",
            "--comment", "artist=\(filesMetadata[0].author)",
            "--comment", "album=\(filesMetadata[0].album)",
        ]
        
        var acc = 0.0
        for (idx, m) in filesMetadata.enumerated() {
            let time = timeString(acc: acc)
            acc += m.duration
            
            command.append(contentsOf: [
                "--comment", "CHAPTER\(idx+1)=\(time)",
                "--comment", "CHAPTER\(idx+1)NAME=\(m.name)"
            ])
        }
        print(command.map { "\"\($0)\"" }.joined(separator: " "))
        return command
    }
    
    private func timeString(acc: Double) -> String {
        let sec = acc.truncatingRemainder(dividingBy: 60.0)
        let min = Int(acc) / 60 % 60
        let h = Int(acc) / 3_600
        return String.init(format: "%02d:%02d:%02.3lf", h, min, sec)
    }
}

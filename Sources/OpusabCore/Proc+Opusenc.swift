import Foundation
import Proc

extension Proc {
    fileprivate static var opusencPath = "/usr/local/bin/opusenc"
    
    static func opusenc(inputs: [FileMetadata], output: String, bitrate: Int, cover: String?) -> Proc {
        let args = opusencArgs(inputs: inputs, output: output, bitrate: bitrate, cover: cover)
        return Proc(opusencPath, args)
    }
}

fileprivate func opusencArgs(inputs: [FileMetadata], output: String, bitrate: Int, cover: String?) -> [String] {
    precondition(inputs.count > 0, "Expected at least one input file")
    
    var args = [
        "--bitrate", "\(bitrate)",
        "--downmix-mono",
        "--comment", "title=\(inputs[0].album)",
        "--comment", "artist=\(inputs[0].author)",
        "--comment", "album=\(inputs[0].album)",
    ]
    if let cover = cover {
        args += [ "--picture", cover ]
    }
    var acc = 0.0
    for (idx, m) in inputs.enumerated() {
        defer { acc += m.duration }
        let time = timeString(acc: acc)
        args += [
            "--comment", String(format: "CHAPTER%03d=\(time)", idx+1),
            "--comment", String(format: "CHAPTER%03dNAME=\(m.name)", idx+1)
        ]
    }
    args += ["-", output]
    return args
}

fileprivate func timeString(acc: Double) -> String {
    let sec = acc.truncatingRemainder(dividingBy: 60.0)
    let min = Int(acc) / 60 % 60
    let h = Int(acc) / 3_600
    return String(format: "%02d:%02d:%06.3lf", h, min, sec)
}



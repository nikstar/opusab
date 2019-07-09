import Foundation
import Proc

extension Proc {
    static func opusenc(metadata: Metadata, output: String, bitrate: Int, cover: String?) -> Proc {
        let args = opusencArgs(metadata: metadata, output: output, bitrate: bitrate, cover: cover)
        return Proc(name: "opusenc", args)
    }
}

fileprivate func opusencArgs(metadata: Metadata, output: String, bitrate: Int, cover: String?) -> [String] {
    var args = [
        "--bitrate", "\(bitrate)",
        "--downmix-mono", // TODO: Explore importance, should probably remove
        "--title", metadata.title,
        "--artist", metadata.author,
        "--album", metadata.title, // TODO: See if used anywhere
    ]
    if let cover = cover {
        args += [ "--picture", cover ]
    }
    for (idx, chapter) in metadata.chapters.enumerated() {
        let time = timeString(time: chapter.start)
        args += [
            "--comment", String(format: "CHAPTER%03d=\(time)", idx+1),
            "--comment", String(format: "CHAPTER%03dNAME=\(chapter.name)", idx+1)
        ]
    }
    args += ["-", output]
    return args
}

func timeString(time: Double) -> String {
    let sec = time.truncatingRemainder(dividingBy: 60.0)
    let min = Int(time) / 60 % 60
    let h = Int(time) / 3_600
    return String(format: "%02d:%02d:%06.3lf", h, min, sec)
}



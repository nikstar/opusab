import Foundation
import Proc

extension Proc {
    fileprivate static var opusencPath = "/usr/local/bin/ffmpeg"
    fileprivate static var commonArgs = [ "-hide_banner", "-nostdin", "-loglevel", "fatal", "-nostats" ]
    
    static func ffmpeg_mp3ToWav() -> Proc {
        let args = commonArgs + [ "-f", "mp3", "-i", "pipe:0", "-f", "wav", "-" ]
        return Proc(opusencPath, args)
    }
    
    static func ffmpeg_accurateDuration(for file: String) -> Double {
        let string = try! Proc("/bin/bash", "-c", "/usr/local/bin/ffmpeg -nostats -hide_banner -nostdin -i \"\(file)\" -f null /dev/null 2>&1")
            .runForStdout()
            .split(separator: " ")
            .first(where: { $0.hasPrefix("time=")})!
            .dropFirst(5)
        return Double(ffmpegTime: String(string))
    }
}

fileprivate extension Double {
    init(ffmpegTime string: String) {
        let t = string.split(separator: ":")
        let h = Double(t[0])!
        let m = Double(t[1])!
        let s = Double(t[2])!
        self = h * 3600 + m * 60 + s
    }
}

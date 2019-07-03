import Foundation
import Proc

extension Proc {
    fileprivate static var opusencPath = "/usr/local/bin/ffmpeg"
    fileprivate static var commonArgs = [ "-hide_banner", "-nostdin", "-loglevel", "fatal", "-nostats" ]
    
    static func ffmpeg_mp3ToWav() -> Proc {
        let args = commonArgs + [ "-f", "mp3", "-i", "pipe:0", "-f", "wav", "-" ]
        return Proc(opusencPath, args)
    }
    
    static func ffmpeg_accurateDuration(for file: String) -> Proc {
        Proc("/bin/bash", "-c", "/usr/local/bin/ffmpeg -nostats -hide_banner -nostdin -i \"\(file)\" -f null /dev/null 2>&1")
    }
}

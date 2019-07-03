import Foundation
import Proc

extension Proc {
    fileprivate static var opusencPath = "/usr/local/bin/ffmpeg"
    fileprivate static var commonArgs = [ "-hide_banner", "-nostdin", "-loglevel", "fatal", "-nostats" ]
    
    static func ffmpeg_mp3ToWav() -> Proc {
        let args = commonArgs + [ "-f", "mp3", "-i", "pipe:0", "-f", "wav", "-" ]
        return Proc(opusencPath, args)
    }
}

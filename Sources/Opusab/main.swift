import Foundation
import OpusabCore

do {
    let opusab = try Opusab()
    try opusab.run()
} catch {
    print("error: \(error)", &stderr)
    exit(1)
}

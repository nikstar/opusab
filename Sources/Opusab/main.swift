import os
import OpusabCore

let tool = try Opusab()

do {
    try tool.run()
} catch {
    print("program returned error: \(error)")
    exit(1)
}

import os
import OpusabCore

do {
    let tool = try Opusab()
    try tool.run()
} catch {
    print("\(error)")
    exit(1)
}

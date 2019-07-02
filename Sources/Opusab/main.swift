import os
import OpusabCore
import SPMUtility

do {
    let tool = try Opusab()
    try tool.run()
} catch {
    print("\(error)")
    exit(1)
}

import Foundation

public struct Stderr : TextOutputStream {
    private var _stderr = FileHandle.standardError

    fileprivate init() {}

    public mutating func write(_ string: String) {
        let data = string.data(using: .utf8)!
        _stderr.write(data)
    }
    
    public func flush() {
        fflush(__stderrp)
    }
}

public var stderr = Stderr()

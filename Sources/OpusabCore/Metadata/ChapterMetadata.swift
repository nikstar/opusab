import Foundation

struct ChapterMetadata {
    var name: String
    var start: Double
}

extension ChapterMetadata : Codable {}

extension Array where Element == ChapterMetadata {
    init(files: [FileMetadata]) {
        var acc = 0.0
        self = files.map { file in
            defer { acc += file.duration }
            return ChapterMetadata(name: file.name, start: acc)
        }
    }
}

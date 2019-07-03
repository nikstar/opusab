import Foundation

struct Metadata {
    var title: String
    var author: String
    var chapters: [ChapterMetadata]
    var totalDuration: Double
}

extension Metadata {
    init(files: [String], verbose: Bool = false) throws {
        precondition(files.count > 0, "At least one input file required")
        var filesMetadata = try files.map(FileMetadata.init)
        let title = filesMetadata.first!.album
        let author = filesMetadata.first!.author
        filesMetadata.stripCommonPrefix()
        let chaptersMetadata = [ChapterMetadata](files: filesMetadata)
        let totalDuration = chaptersMetadata.last!.start + filesMetadata.last!.duration
        self.init(title: title, author: author, chapters: chaptersMetadata, totalDuration: totalDuration)
    }
}

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
        let progressReporter = ProgressReporter(description: "Parsing metadata", steps: files.count, verbose: verbose)
        var filesMetadata = try files.map(reporter: progressReporter) { try FileMetadata(file: $0) }
        let title = filesMetadata.first!.album
        let author = filesMetadata.first!.author
        filesMetadata.stripCommonPrefix()
        let chaptersMetadata = [ChapterMetadata](files: filesMetadata)
        let totalDuration = chaptersMetadata.last!.start + filesMetadata.last!.duration
        self.init(title: title, author: author, chapters: chaptersMetadata, totalDuration: totalDuration)
    }
}

fileprivate extension Array where Element == String {
    func map<T>(reporter: ProgressReporter, _ f: (Element) throws -> T ) rethrows -> [T] {
        return try map {
            reporter.nextStep(description: $0)
            return try f($0)
        }
    }
}

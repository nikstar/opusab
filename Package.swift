// swift-tools-version:4.0
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "Opusab",
    dependencies: [
        .package(url: "https://github.com/apple/swift-package-manager.git", from: "0.1.0"),
        .package(url: "https://github.com/johnsundell/files.git", from: "1.0.0"),
        .package(url: "https://github.com/SwiftyBeaver/SwiftyBeaver.git", from: "1.0.0"),
    ],
    targets: [
        .target(
            name: "Opusab",
            dependencies: ["OpusabCore"]),
        .target(
            name: "OpusabCore",
            dependencies: ["Files", "Utility", "SwiftyBeaver"]),
        .target(
            name: "OpusabTest",
            dependencies: ["OpusabCore"]),
        .testTarget(
            name: "OpusabTests",
            dependencies: ["OpusabCore", "Files"]),
    ]
)

// swift-tools-version:4.0
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "Opusab",
    dependencies: [
        .package(url: "https://github.com/apple/swift-package-manager.git", from: "0.3.0"),
        .package(url: "https://github.com/johnsundell/files.git", from: "1.0.0"),
    ],
    targets: [
        .target(
            name: "opusab",
            dependencies: ["OpusabCore"]),
        .target(
            name: "OpusabCore",
            dependencies: ["Files", "Utility"]),
        .target(
            name: "OpusabTest",
            dependencies: ["OpusabCore"]),
        .testTarget(
            name: "OpusabTests",
            dependencies: ["OpusabCore", "Files"]),
    ]
)

// swift-tools-version:5.1
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "Opusab",
    products: [
        .executable(name: "opusab", targets: ["Opusab"])
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-package-manager.git", .branch("master")),
        .package(url: "https://github.com/johnsundell/files.git", from: "2.0.0"),
    ],
    targets: [
        .target(
            name: "Opusab",
            dependencies: ["OpusabCore"]),
        .target(
            name: "OpusabCore",
            dependencies: ["Files", "SPMUtility"]),
        .target(
            name: "OpusabTest",
            dependencies: ["OpusabCore"]),
    ]
)

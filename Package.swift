// swift-tools-version:5.1
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "Opusab",
    platforms: [
        .macOS(.v10_13)
    ],
    products: [
        .executable(name: "opusab", targets: ["Opusab"])
    ],
    dependencies: [
        .package(url: "https://github.com/johnsundell/files.git", from: "2.0.0"),
        .package(url: "https://github.com/nikstar/CLInterface.git", .branch("master")),
        .package(url: "https://github.com/nikstar/Proc.git", .branch("master")),
    ],
    targets: [
        .target(
            name: "Opusab",
            dependencies: ["OpusabCore"]),
        .target(
            name: "OpusabCore",
            dependencies: ["Files", "CLInterface", "Proc"]),
        .target(
            name: "OpusabTest",
            dependencies: ["OpusabCore"]),
    ]
)


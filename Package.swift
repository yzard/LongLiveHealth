// swift-tools-version: 6.2
import PackageDescription

let package = Package(
    name: "LongLiveHealthCore",
    platforms: [.iOS(.v17), .macOS(.v14)],
    products: [.library(name: "LongLiveHealthCore", targets: ["LongLiveHealthCore"])],
    targets: [
        .target(name: "LongLiveHealthCore", path: "src/ios/Core"),
        .testTarget(name: "LongLiveHealthCoreTests", dependencies: ["LongLiveHealthCore"], path: "tests/ios/Core")
    ]
)

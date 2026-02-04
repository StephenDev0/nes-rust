// swift-tools-version:5.3
import PackageDescription

let package = Package(
    name: "NesRust",
    platforms: [
        .iOS(.v13)
    ],
    products: [
        .library(
            name: "NesRust",
            targets: ["NesRust"]
        ),
    ],
    targets: [
        .binaryTarget(
            name: "NesRust",
            path: "ios/NesRust.xcframework"
        )
    ]
)

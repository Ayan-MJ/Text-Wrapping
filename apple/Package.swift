// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "LastDraftFlowApple",
    platforms: [
        .iOS(.v17),
        .macOS(.v14),
    ],
    products: [
        .library(name: "LastDraftFlowApple", targets: ["LastDraftFlowApple"]),
    ],
    targets: [
        .binaryTarget(
            name: "CLastDraftFlow",
            path: "Artifacts/CLastDraftFlow.xcframework"
        ),
        .target(
            name: "LastDraftFlowApple",
            dependencies: ["CLastDraftFlow"],
            path: "Sources/LastDraftFlowApple"
        ),
        .testTarget(
            name: "LastDraftFlowAppleTests",
            dependencies: ["LastDraftFlowApple"],
            path: "Tests/LastDraftFlowAppleTests"
        ),
    ]
)

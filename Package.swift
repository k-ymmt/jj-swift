// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "jj-swift",
    platforms: [
        .macOS(.v12),
    ],
    products: [
        .library(
            name: "JJSwift",
            targets: ["JJSwift"]
        ),
    ],
    targets: [
        // Build Tool Plugin
        .plugin(
            name: "BuildFFI",
            capability: .buildTool(),
            path: "Plugins/BuildFFI"
        ),

        // C ヘッダーモジュール
        .target(
            name: "CJJFfi",
            path: "Sources/CJJFfi",
            publicHeadersPath: "include"
        ),

        // メイン Swift ターゲット
        .target(
            name: "JJSwift",
            dependencies: ["CJJFfi"],
            swiftSettings: [
                .swiftLanguageMode(.v5),
            ],
            linkerSettings: [
                .unsafeFlags([
                    "-L", "jj-ffi/target/release",
                    "-ljj_ffi",
                ]),
                .linkedLibrary("c++"),
            ],
            plugins: ["BuildFFI"]
        ),

        .testTarget(
            name: "JJSwiftTests",
            dependencies: ["JJSwift"]
        ),
    ]
)

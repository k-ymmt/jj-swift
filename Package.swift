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
        .plugin(
            name: "BuildFFI",
            capability: .command(
                intent: .custom(
                    verb: "cargo_build",
                    description: "build ffi library with cargo."
                ),
                permissions: [
                    .allowNetworkConnections(scope: .all(ports: [80, 443]), reason: "fetch cargo dependencies."),
                    .writeToPackageDirectory(reason: "write build artifacts"),
                ],
            ),
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
        ),

        .testTarget(
            name: "JJSwiftTests",
            dependencies: ["JJSwift"]
        ),
    ]
)

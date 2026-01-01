# jj-swift: SwiftPM ライブラリ実装計画

## 概要

jj-ffi (UniFFI で生成された Rust バインディング) を SwiftPM パッケージとして配布可能にする。
Rust ライブラリのビルドは SwiftPM Build Tool Plugin で自動化する。

**対象プラットフォーム:** macOS (Apple Silicon のみ)

## 現状分析

### jj-ffi で生成されるファイル

```
jj-ffi/generated/
├── jj_ffi.swift          # Swift バインディングコード
├── jj_ffiFFI.h           # C ヘッダーファイル
└── jj_ffiFFI.modulemap   # Clang モジュールマップ

jj-ffi/target/release/
└── libjj_ffi.a           # 静的ライブラリ (arm64)
```

---

## アーキテクチャ

### SwiftPM Build Tool Plugin アプローチ

```
jj-swift/
├── Package.swift
├── Plugins/
│   └── BuildFFI/
│       └── plugin.swift       # Build Tool Plugin
├── Sources/
│   ├── CJJFfi/                # C モジュール (ヘッダー参照用)
│   │   ├── include/
│   │   │   └── module.modulemap
│   │   └── shim.c             # 空ファイル (SwiftPM 要件)
│   └── JJSwift/
│       └── jj_ffi.swift       # UniFFI 生成 Swift コード
├── jj-ffi/                    # Rust FFI (既存)
└── Tests/
```

### ビルドフロー

```
1. SwiftPM ビルド開始
2. BuildFFI Plugin 実行 (prebuild)
   ├── cargo build --release (Rust ビルド)
   ├── uniffi-bindgen generate (Swift/C バインディング生成)
   └── ファイルを所定位置にコピー
3. CJJFfi ターゲットビルド (C ヘッダー)
4. JJSwift ターゲットビルド (Swift + リンク)
```

---

## 実装フェーズ

### Phase 1: Build Tool Plugin

`Plugins/BuildFFI/plugin.swift`:

```swift
import PackagePlugin
import Foundation

@main
struct BuildFFIPlugin: BuildToolPlugin {
    func createBuildCommands(context: PluginContext, target: Target) async throws -> [Command] {
        let ffiDir = context.package.directory.appending("jj-ffi")
        let outputDir = context.pluginWorkDirectory

        return [
            .prebuildCommand(
                displayName: "Build Rust FFI Library",
                executable: try context.tool(named: "bash").path,
                arguments: [
                    "-c",
                    """
                    cd "\(ffiDir)" && \
                    cargo build --release && \
                    cargo run --bin uniffi-bindgen generate \
                      --library target/release/libjj_ffi.a \
                      --language swift \
                      --out-dir "\(outputDir)"
                    """
                ],
                outputFilesDirectory: outputDir
            )
        ]
    }
}
```

### Phase 2: C モジュール (ヘッダー参照)

`Sources/CJJFfi/include/module.modulemap`:

```
module CJJFfi {
    header "jj_ffiFFI.h"
    export *
}
```

`Sources/CJJFfi/shim.c`:

```c
// Empty file required by SwiftPM for C targets
```

**注意:** `jj_ffiFFI.h` は Plugin 実行後に配置されるか、事前に `jj-ffi/generated/` から参照する。

### Phase 3: Package.swift

```swift
// swift-tools-version: 5.9
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
            plugins: ["BuildFFI"],
            linkerSettings: [
                .unsafeFlags([
                    "-L", "jj-ffi/target/release",
                    "-ljj_ffi",
                ]),
                .linkedLibrary("c++"),
            ]
        ),

        .testTarget(
            name: "JJSwiftTests",
            dependencies: ["JJSwift"]
        ),
    ]
)
```

### Phase 4: Swift コード配置

`Sources/JJSwift/jj_ffi.swift`:
- `jj-ffi/generated/jj_ffi.swift` をコピー
- または Plugin で動的生成されたファイルを参照

---

## 代替案: Command Plugin + 手動実行

Build Tool Plugin の制限 (サンドボックス等) が問題になる場合、Command Plugin を使用:

```swift
@main
struct BuildFFICommand: CommandPlugin {
    func performCommand(context: PluginContext, arguments: [String]) async throws {
        // swift package plugin build-ffi で手動実行
    }
}
```

---

## 考慮事項

### 1. Plugin サンドボックス

SwiftPM Plugin はデフォルトでサンドボックス内で実行される。
`cargo` 実行には `--allow-writing-to-package-directory` が必要な場合あり:

```bash
swift build --allow-writing-to-package-directory
```

### 2. 生成ファイルの扱い

**Option A: 事前生成 (推奨)**
- `jj_ffi.swift` と `jj_ffiFFI.h` をリポジトリにコミット
- Plugin は Rust ライブラリのビルドのみ担当

**Option B: 動的生成**
- Plugin で毎回生成
- `.build/` 配下に出力、Swift ターゲットから参照

### 3. unsafeFlags の制限

`unsafeFlags` を使用すると、他パッケージからの依存が制限される。
配布用途では XCFramework アプローチを併用することを推奨。

---

## 実装チェックリスト

- [ ] `Plugins/BuildFFI/plugin.swift` 作成
- [ ] `Sources/CJJFfi/` ディレクトリ構成
  - [ ] `include/module.modulemap`
  - [ ] `include/jj_ffiFFI.h` (コピー or シンボリックリンク)
  - [ ] `shim.c`
- [ ] `Sources/JJSwift/` ディレクトリ構成
  - [ ] `jj_ffi.swift` (コピー)
- [ ] `Package.swift` 更新
- [ ] ビルド確認 (`swift build`)
- [ ] テスト作成・実行

---

## ディレクトリ構造 (最終形)

```
jj-swift/
├── Package.swift
├── Plugins/
│   └── BuildFFI/
│       └── plugin.swift
├── Sources/
│   ├── CJJFfi/
│   │   ├── include/
│   │   │   ├── module.modulemap
│   │   │   └── jj_ffiFFI.h
│   │   └── shim.c
│   └── JJSwift/
│       └── jj_ffi.swift
├── Tests/
│   └── JJSwiftTests/
│       └── JJSwiftTests.swift
├── jj-ffi/                    # Rust プロジェクト (既存)
└── docs/
    └── Plan.md
```

---

## 参考資料

- [SwiftPM Build Tool Plugins](https://github.com/apple/swift-package-manager/blob/main/Documentation/Plugins.md)
- [UniFFI Manual - Swift](https://mozilla.github.io/uniffi-rs/latest/swift/overview.html)
- [WWDC21 - Meet the Swift Package plugins](https://developer.apple.com/videos/play/wwdc2021/10233/)

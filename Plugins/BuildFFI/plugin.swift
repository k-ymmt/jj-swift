import PackagePlugin
import Foundation

@main
struct BuildFFIPlugin: BuildToolPlugin {
    func createBuildCommands(context: PluginContext, target: Target) async throws -> [Command] {
        let ffiDirURL = context.package.directoryURL.appending(path: "jj-ffi")
        let outputDirURL = context.pluginWorkDirectoryURL

        // Get cargo path from environment or use default
        let homeDir = FileManager.default.homeDirectoryForCurrentUser.path
        let cargoPath = "\(homeDir)/.cargo/bin/cargo"

        return [
            .prebuildCommand(
                displayName: "Build Rust FFI Library",
                executable: URL(filePath: "/bin/bash"),
                arguments: [
                    "-c",
                    """
                    export PATH="/usr/bin:/bin:/usr/sbin:/sbin:\(homeDir)/.cargo/bin:$PATH" && \
                    cd "\(ffiDirURL.path)" && \
                    "\(cargoPath)" build --release
                    """
                ],
                outputFilesDirectory: outputDirURL
            )
        ]
    }
}

import PackagePlugin
import Foundation

@main
struct CommandFFIPlugin: CommandPlugin {
    func performCommand(context: PluginContext, arguments: [String]) async throws {
        let ffiDirURL = context.package.directoryURL.appending(path: "jj-ffi")

        // Get cargo path from environment or use default
        let homeDir = FileManager.default.homeDirectoryForCurrentUser.path

        let process = try Process.run(URL(filePath: "/bin/sh"), arguments: [
            "-c",
            """
            export PATH="/usr/bin:/bin:/usr/sbin:/sbin:\(homeDir)/.cargo/bin:$PATH" && \
            cargo build --manifest-path \(ffiDirURL.appending(path: "Cargo.toml").path(percentEncoded: false)) --release
            """
        ])
        process.waitUntilExit()
    }
}

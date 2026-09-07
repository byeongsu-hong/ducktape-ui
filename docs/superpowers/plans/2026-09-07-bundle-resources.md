# Include module wasm resources in desktop installers

Claude's actual Ducktape release port cannot package `views/*.wasm`. Add a
`resources = ["relative/path"]` list to `[package.metadata.ice.bundle]`.
Sources resolve from the package manifest directory, matching `icon`; directories
contribute their file contents beneath the source basename. Resolve generated
files after Cargo builds, reject missing/symlink/special files and destination
collisions, and copy before signing or installer construction.

Use executable-relative paths on every desktop: `Contents/MacOS` on macOS, the
Windows install directory, and a package-private `/usr/lib/<package>` payload
with `/usr/bin/<exe>` launcher symlink on Linux when resources are present.
Verify nested wasm bytes, metadata diagnostics, collision handling, stale-file
removal, and actual platform packagers where available. Keep source roots and
installer destination paths distinct. No globbing or compatibility layer.

Run portable bundle tests and actual Linux packaging locally; extend the native
macOS signing test with resources and run it in macOS CI. Report the merged
revision and manifest-relative path rule through the authorized Claude session socket.

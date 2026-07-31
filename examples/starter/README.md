# Ice starter

This is the smallest runnable workspace application that keeps the complete
Ice delivery path visible: `build.rs` compiles `src/ui/app.ice`,
`ui_lang::include_app!` includes the generated program, and the authored Ice
test drives the real headless application.

```bash
cargo run -p ice-starter
cargo test -p ice-starter
cargo ice check
```

Copy the package when starting an application, then replace the workspace
dependencies with the released `ui-lang`, `ui-lang-runtime`, and
`ui-lang-build` versions.

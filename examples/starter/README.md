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
dependencies with released versions:

```toml
[dependencies]
ui-lang = "=0.1.0"
ui-lang-runtime = "=0.1.0"

[build-dependencies]
ui-lang-build = "=0.1.0"
```

The starter has no `ducktape-ui` or showcase dependency, and its Ice graph has
no repository-relative import. Its local `theme.ice` import is compiled through
the same `ui-lang-build` graph as a downstream multi-file application.

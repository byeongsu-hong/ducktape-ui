state
  document:editor = "# Native Markdown\n\nWrite **bold**, *italic*, `code`, and [links](https://iced.rs) in one focused surface.\n\n## Built for flow\n\n- Native selection and clipboard\n- Incremental Markdown styling\n- No preview mode or web view\n\n```rust\nfn fast_path() {\n    // Only changed lines are highlighted again.\n}\n```"

preset test
  state
    document = editor("# Native Markdown\n\nA focused writing surface.")

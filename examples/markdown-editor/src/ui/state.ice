enum EditorMode
  write
  preview

state
  document:editor = "# Native Markdown\n\nStart writing here. The editor owns selection, clipboard, undo, and IME input.\n\n## First slice\n\n- One document\n- One window\n- No web view"
  rendered:markdown = "# Native Markdown\n\nStart writing here. The editor owns selection, clipboard, undo, and IME input.\n\n## First slice\n\n- One document\n- One window\n- No web view"
  mode:EditorMode = EditorMode.write

derived
  previewing = mode == EditorMode.preview

preset test
  state
    document = editor("# Native Markdown\n\nA focused writing surface.")
    rendered = markdown("# Native Markdown\n\nA focused writing surface.")
    mode = EditorMode.write

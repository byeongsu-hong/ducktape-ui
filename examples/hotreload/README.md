# Hot reload example

Run the example through the development runner:

```bash
cargo ice dev examples/hotreload/src/ui/app.ice -- -p hotreload-example
```

The left pane is the rendered Ice view. The right pane edits
`src/ui/screen.ice`; **Save & hot reload** writes it back to disk. Changes to
static structure, literals, colors, and spacing reload in the same process, so
the preview counter keeps its value. Edits that need new compiled values use
the normal staged rebuild and restart path.

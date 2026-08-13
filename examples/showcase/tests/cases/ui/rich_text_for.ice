app RichTextFor

use "themes/slate.ice"

state
  tokens = ["alpha", "beta"]

on grow
  tokens = ["alpha", "beta", "gamma"]

// The chat-line shape: a dynamically built span list renders as ONE native
// paragraph, not a flex of per-token text widgets. The exact-match text
// oracle below only holds while every span, `for`-generated ones included,
// lands in the same paragraph buffer.
view
  col gap=4.0 p=8.0
    rich-text #paragraph wrap=word
      span "Report:"
      for token in tokens
        span token underline
      span "done"

test for_spans_render_inside_the_paragraph
  target paragraph = #paragraph
  expect paragraph.visible
  expect text "Report:alphabetadone" within paragraph
  dispatch grow
  expect no text "Report:alphabetadone" within paragraph
  expect text "Report:alphabetagammadone" within paragraph

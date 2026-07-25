app InputMethodEvents

use "themes/monochrome.ice"

on opened

on preedit(text, start, end)

on commit(text)

on closed

subscribe
  input-method opened -> opened
  input-method preedit status=any -> preedit _ _ _
  input-method commit -> commit _
  input-method closed -> closed

view
  text "Input method events compile fixture"

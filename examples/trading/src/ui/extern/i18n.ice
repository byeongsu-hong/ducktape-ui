// Every sentence the screen draws, in the language picked on settings.
//
// The English is the key: a `text` node keeps its sentence in the source,
// wrapped in `t(locale, ...)`, and the other languages are looked up by it.
// A key no table carries comes back as itself, so the app is readable at
// every commit of a translation rather than only at the last one, and a
// test written against the English still reads the English it was written
// against. Rust prose — a venue's note, a session badge, a refusal — takes
// the locale as a parameter the same way and keeps its own table.
extern crate::i18n
  pure t(locale:Locale, key:str) -> str
  // What the language picker calls each language, in that language: a
  // reader who cannot read the current one has to be able to find their own.
  pure locale_name(locale:Locale) -> str
  pure locale_label(locale:Locale) -> str

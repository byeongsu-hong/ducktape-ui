extern crate::backend
  Entry(id:i64, title:str)
  pure seeded_entries() -> [Entry]
  pure appended(entries:[Entry], title:str) -> [Entry]

// The other venue's fixtures, which live with the parsers that built them.
//
// Every one of these is a captured Lighter response run through the same
// parser the live read uses, so a Lighter preset draws Lighter's markets, its
// tickers, its price scales and its leverage caps rather than the other
// exchange's numbers under this exchange's name. That is the whole reason they
// are not beside `demo_symbols` in `hyperliquid`: a fixture written next to
// the venue it does not come from is how the two got mixed up in the first
// place.
extern crate::lighter
  pure demo_address_lighter() -> str
  pure demo_symbols_lighter() -> [SymbolRow]
  pure demo_account_lighter() -> Account
  pure demo_positions_lighter() -> [Position]
  pure demo_book_lighter() -> Book
  pure demo_tape_lighter() -> [Trade]

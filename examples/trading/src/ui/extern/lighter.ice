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
  sync demo_address_lighter() -> str
  sync demo_symbols_lighter() -> [SymbolRow]
  sync demo_account_lighter() -> Account
  sync demo_positions_lighter() -> [Position]
  sync demo_book_lighter() -> Book
  sync demo_tape_lighter() -> [Trade]

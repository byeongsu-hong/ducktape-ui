// The language is a setting, and the setting is the whole screen: every
// sentence the view draws goes through `t(locale, ...)`, and the Rust table
// answers each one in Korean. The test reads the page that holds the picker
// and the page a trader lives on, so a sentence that reached the screen
// around the table — a bare literal, or a key the table lacks — would read
// as English here. The English is asserted absent as well as the Korean
// present, because a page drawing both would pass the second alone.
test trading_picking_korean_reads_the_whole_screen_in_korean
  preset held
  viewport 1660 900
  target app = #app
  target settings = app/settings
  target picker = settings/settings-content/settings-display/languages
  target english = picker/lang-en/root/on
  target korean = picker/lang-ko/root/off
  target korean_on = picker/lang-ko/root/on
  target portfolio_tab = app/header/pages/page-portfolio/root/tab-off
  dispatch navigate(Page.settings)
  expect locale == Locale.en
  // Each language is offered in itself: the reader who cannot read the one on
  // screen has to be able to find their own.
  expect a11y english name "Read this app in English"
  expect a11y korean name "이 앱을 한국어로 읽기"
  expect text "LANGUAGE"
  expect text "Two keys, and only one of them can trade."
  click korean
  expect locale == Locale.ko
  expect exists korean_on
  expect text "언어"
  expect no text "LANGUAGE"
  expect text "키는 둘이고, 거래할 수 있는 키는 하나뿐입니다."
  expect no text "Two keys, and only one of them can trade."
  expect text "네트워크"
  expect text "키 보관"
  expect text "키보드"
  expect text "매수 / 롱"
  expect no text "Buy / long"
  // The header reads in it too, which is what makes it the app's language
  // rather than this page's.
  expect text "포트폴리오"
  expect no text "PORTFOLIO"
  // And the names a screen reader hears are whole sentences in it, not an
  // English frame around a Korean word.
  expect a11y portfolio_tab name "포트폴리오 페이지 열기"
  capture settings_korean
  // And the terminal, which is where the setting is actually lived with.
  dispatch navigate(Page.terminal)
  expect text "호가창"
  // The search box's hint is an expression now, so it reads in the language
  // too rather than staying the one literal on the page.
  expect text "마켓 검색"
  expect no text "Search markets"
  expect no text "ORDER BOOK"
  expect text "미체결 주문"
  expect text "포지션"
  expect text "새 주문"
  // The figures are the same figures.
  expect text "64,001.00"
  capture terminal_korean
  // Back again, and the English is back, with nothing of the Korean left.
  dispatch set_locale(Locale.en)
  expect text "ORDER BOOK"
  expect no text "호가창"

// The sentences Rust composes at runtime — a row's spoken summary, the
// chart's name, a venue's own limits, the send button's act — reach the
// screen through the same `t`, answered by template: the English Rust wrote
// is taken apart around the figures it spliced in, and put back together in
// Korean with each figure where the Korean puts it. Nothing translated is
// stored in state, so every English assertion elsewhere in this suite holds, and
// this one holds the Korean side of the same sentences — with the English
// asserted absent, because a template that did not match would leave it.
test trading_the_sentences_rust_composes_read_in_korean_too
  preset ready_to_send
  viewport 1660 900
  target app = #app
  target trade = app/terminal-fit/trade
  target bitcoin = trade/lower/positions/position-list/position("BTC")/root
  target chart = trade/chart-frame/chart
  target review = trade/ticket-panel/ticket-review
  target rail = trade/markets/market-list
  target eth = rail/market("ETH")/row
  expect a11y bitcoin name "BTC short 30, entry 81,461.50, liquidation 174,000.00, funding +$33.1K, unrealized +$523.8K at +857.41%"
  expect a11y eth name "ETH at 3,540.00, +1.14% today"
  dispatch set_locale(Locale.ko)
  expect a11y bitcoin name "BTC 숏 30, 진입가 81,461.50, 청산가 174,000.00, 펀딩 +$33.1K, 미실현 +$523.8K, 수익률 +857.41%"
  expect a11y chart name "Hyperliquid 캔들 차트; 지표: SMA 20, SMA 60"
  // The chart kit's own empty state is the app's sentence, in the app's
  // language: this preset holds no tape, so the plot says so.
  expect text "데이터 없음"
  expect no text "No data"
  expect a11y review name "BTC 3 매수 주문을 Hyperliquid(실거래)에 전송"
  // A market row is memoized on the row alone, so the language rides along
  // as an extra dependency of that memo: a row that did not change is not
  // rebuilt, and the one time the language changes every row is.
  expect a11y eth name "ETH 3,540.00, 오늘 +1.14%"
  // And a venue's own limit, stored as English in the registry and read in
  // Korean on the page that states it.
  dispatch switch_venue(Venue.lighter)
  dispatch navigate(Page.settings)
  expect text t(Locale.ko, venue_account_gap(Venue.lighter))
  expect no text venue_account_gap(Venue.lighter)
  expect text "Lighter는 미체결 주문과 이 계좌의 체결 내역을 API 키로 서명한 토큰에만 제공합니다. 주소만으로는 그 토큰을 받을 수 없고, 이 앱은 그 토큰을 갖고 있지 않습니다."

// The chart kit draws two sentences of its own — the empty plot's, and the
// chip that jumps back to the newest candle once the view has left it — and
// both are the app's, in the app's language.
test trading_the_chart_chip_reads_in_korean
  preset browsing
  viewport 1660 820
  target app = #app
  target chart = app/terminal-fit/trade/chart-frame/chart
  expect no text "Latest >"
  // Zooming in around the pointer leaves the newest candle off the right
  // edge, which is when the chip appears.
  move chart
  wheel lines 0.0 6.0
  expect text "Latest >"
  dispatch set_locale(Locale.ko)
  expect no text "Latest >"
  expect text "최신 >"

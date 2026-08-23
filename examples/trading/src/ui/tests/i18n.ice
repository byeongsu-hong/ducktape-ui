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
  capture settings_korean
  // And the terminal, which is where the setting is actually lived with.
  dispatch navigate(Page.terminal)
  expect text "호가창"
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

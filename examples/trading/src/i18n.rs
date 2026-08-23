//! Every sentence the screen draws, in the language picked on settings.
//!
//! The English is the key. A `.ice` node keeps its sentence in the source
//! wrapped in `t(locale, ...)`, and the other languages are looked up by
//! it here. A key no table carries comes back as itself, so the app reads
//! as English rather than as a hole at every commit of a translation, and a
//! test written against the English keeps reading the English.
//!
//! One table per language, one arm per sentence, a plain `match`: there is
//! no plural machinery because Korean has no plural categories and the
//! numbers are already formatted by the time a sentence holds them, and
//! there is no file to load because a missing file is a screen of holes.
//! A test below walks the `.ice` sources and the Rust prose for every key
//! and asks this table for each one, so a sentence added without its Korean
//! fails the build's tests rather than the reader.

use crate::Locale;

/// The sentence `key` in `locale`. English is the key itself.
pub fn t(locale: Locale, key: String) -> String {
    match locale {
        Locale::En => key,
        Locale::Ko => ko(&key).map_or(key, str::to_owned),
    }
}

/// What the picker calls a language, in that language.
pub fn locale_name(locale: Locale) -> String {
    match locale {
        Locale::En => "English",
        Locale::Ko => "한국어",
    }
    .to_owned()
}

/// What pressing a language does, said in the language being offered: a
/// reader who cannot read the one on screen has to be able to hear their own.
pub fn locale_label(locale: Locale) -> String {
    match locale {
        Locale::En => "Read this app in English",
        Locale::Ko => "이 앱을 한국어로 읽기",
    }
    .to_owned()
}

fn ko(key: &str) -> Option<&'static str> {
    Some(match key {
        // Navigation and the header.
        "TERMINAL" => "터미널",
        "PORTFOLIO" => "포트폴리오",
        "SETTINGS" => "설정",
        "Portfolio" => "포트폴리오",
        "Settings" => "설정",
        "LIVE" => "실시간",
        "NOT LIVE" => "끊김",
        "Browse markets" => "마켓 둘러보기",
        "Search markets" => "마켓 검색",
        "Watch an address" => "주소 지켜보기",
        "Watch this address" => "이 주소 지켜보기",
        "Watch this address, read-only" => "이 주소를 읽기 전용으로 지켜보기",
        "Connect an address" => "주소 연결",
        "Connect a different address" => "다른 주소 연결",
        "No address is connected." => "연결된 주소가 없습니다.",
        "No market matches that." => "일치하는 마켓이 없습니다.",
        "Not served here" => "이 네트워크에는 없음",
        // Side tokens stay the tickers every Korean venue prints.
        "BUY / LONG" => "매수 / 롱",
        "SELL / SHORT" => "매도 / 숏",
        "LONG / SHORT" => "롱 / 숏",
        "SIDE" => "방향",
        "MARKET / SIDE" => "마켓 / 방향",
        // Markets, book, tape.
        "MARKETS" => "마켓",
        "COIN" => "코인",
        "PERP" => "무기한",
        "LAST" => "현재가",
        "MARK" => "마크가",
        "PRICE" => "가격",
        "SIZE" => "수량",
        "Size" => "수량",
        "VALUE" => "평가액",
        // Beside the literal O H L C of the candle readout, which are the
        // tickers every chart prints.
        "VOL" => "VOL",
        "VOLUME" => "거래량",
        "TIME" => "시각",
        "AGE" => "경과",
        "ORDER BOOK" => "호가창",
        "SPREAD" => "스프레드",
        "TAPE" => "체결",
        "FILLS" => "체결 내역",
        "RECENT FILLS" => "최근 체결",
        "FILL HISTORY" => "체결 이력",
        "Loading book" => "호가 불러오는 중",
        "Waiting for a print." => "첫 체결을 기다리는 중.",
        "Pick a row to open its market" => "행을 누르면 그 마켓이 열립니다",
        "What this account has actually traded" => "이 계좌가 실제로 체결한 내역",
        "Export these fills to a CSV file" => "이 체결 내역을 CSV 파일로 내보내기",
        "CSV" => "CSV",
        "INDICATORS" => "지표",
        "Close chart indicators" => "차트 지표 닫기",
        // Alerts.
        "ALERTS" => "알림",
        "WATCH THIS LEVEL" => "이 가격 지켜보기",
        "Watch this level" => "이 가격 지켜보기",
        "No levels watched." => "지켜보는 가격이 없습니다.",
        // Positions, orders, account.
        "POSITIONS" => "포지션",
        "ASSETS" => "자산",
        "ORDERS" => "주문",
        "OPEN ORDERS" => "미체결 주문",
        "CANCEL" => "취소",
        "CANCEL ALL" => "전체 취소",
        "FLATTEN ALL" => "전체 청산",
        "CLOSE POSITION" => "포지션 종료",
        "Fill the size that closes this position" => "이 포지션을 종료하는 수량 채우기",
        "No open positions to list." => "보유 중인 포지션이 없습니다.",
        "No open positions on this account." => "이 계좌에 보유 중인 포지션이 없습니다.",
        "No open positions to have been funded." => "펀딩이 발생할 포지션이 없습니다.",
        "No open exposure." => "노출된 포지션이 없습니다.",
        "none" => "없음",
        "ENTRY" => "진입가",
        "LIQ" => "청산가",
        "LIQUIDATION" => "청산가",
        "LEVERAGE" => "레버리지",
        "Leverage" => "레버리지",
        "EFFECTIVE LEVERAGE" => "실효 레버리지",
        "MARGIN" => "증거금",
        "MARGIN MODE" => "마진 모드",
        "MARGIN USED" => "사용 증거금",
        "MARGIN REQUIRED" => "필요 증거금",
        "MAINTENANCE REQUIRED" => "유지 증거금",
        "POSITION MARGIN" => "포지션 증거금",
        "MARGIN HEALTH" => "증거금 상태",
        "CROSS" => "교차",
        "ISOLATED" => "격리",
        "EQUITY" => "순자산",
        "CROSS EQUITY" => "교차 순자산",
        "ACCOUNT VALUE" => "계좌 가치",
        "WITHDRAWABLE" => "출금 가능",
        "PNL" => "손익",
        "PNL / SIZE" => "손익 / 수량",
        "REALIZED" => "실현",
        "REALIZED PNL" => "실현 손익",
        "UNREALIZED" => "미실현",
        "UNREALIZED PNL" => "미실현 손익",
        "FUNDING" => "펀딩",
        "FUNDING IN" => "다음 펀딩까지",
        "RENT PER DAY" => "일일 비용",
        "PAID" => "지불",
        "RECEIVED" => "수취",
        "NET" => "순액",
        "Charged against open positions since each opened" => "각 포지션이 열린 뒤로 부과된 금액",
        "What the engine holds against the cross book" => "교차 장부에 대해 엔진이 잡아 둔 금액",
        // Portfolio.
        "Current derivatives exposure and account-value history." => {
            "현재 파생상품 노출과 계좌 가치 이력."
        }
        "Historical performance" => "과거 성과",
        "PEAK" => "고점",
        "OFF PEAK" => "고점 대비",
        "MAX DRAWDOWN" => "최대 낙폭",
        "PNL BY PERIOD" => "기간별 손익",
        "What each step of the window booked, from the venue's own ledger" => {
            "구간마다 장부에 오른 손익, 거래소 원장 기준"
        }
        "ALL" => "전체",
        "EXPOSURE ALLOCATION" => "노출 배분",
        "Share of gross marked value" => "총 평가액 중 비중",
        "GROSS EXPOSURE" => "총 노출",
        "WEIGHT" => "비중",
        "CLOSED W / L" => "청산 승 / 패",
        "WIN RATE" => "승률",
        // The ticket.
        "New order" => "새 주문",
        "MARKET" => "시장가",
        "LIMIT" => "지정가",
        "SCALE" => "분할",
        "TWAP" => "TWAP",
        "Cross the spread now" => "지금 스프레드를 넘어 체결",
        "Rest at a price you choose" => "정한 가격에 걸어 두기",
        "Spread the size over a range of prices" => "수량을 가격 구간에 나누어 걸기",
        "Let the venue work the size over a window" => "거래소가 일정 시간에 걸쳐 체결하게 두기",
        "LIMIT PRICE" => "주문 가격",
        "Limit price" => "주문 가격",
        "FROM" => "시작",
        "TO" => "끝",
        "OVER" => "기간",
        "Ladder from this price" => "이 가격부터 사다리 시작",
        "Ladder to this price" => "이 가격까지 사다리 끝",
        "How many orders the ladder is" => "사다리 주문 개수",
        "Minutes to work this order over" => "이 주문을 체결할 시간(분)",
        "Type the size in coins" => "수량을 코인 단위로 입력",
        "Type the size in dollars" => "수량을 달러 단위로 입력",
        "USD" => "USD",
        "MAX" => "최대",
        "max" => "최대",
        "REDUCE ONLY" => "청산 전용",
        "Reduce only" => "청산 전용",
        "closes only" => "종료만",
        "TAKE PROFIT" => "익절",
        "STOP LOSS" => "손절",
        "Attach a take-profit and a stop-loss" => "익절과 손절을 함께 걸기",
        "Hold this order against its own margin" => "이 주문을 격리 증거금으로 잡기",
        "Hold this order against the whole account" => "이 주문을 계좌 전체 증거금으로 잡기",
        "ORDER VALUE" => "주문 금액",
        "FILLS AT" => "체결 예상가",
        "FILLS AT, ABOUT" => "체결 예상가, 약",
        "IF YOU CROSS" => "스프레드를 넘으면",
        "PRICED AT" => "기준 레버리지",
        "AGAINST THE ENGINE" => "엔진 기준",
        "The book on screen cannot fill that size." => {
            "화면의 호가로는 그 수량을 채울 수 없습니다."
        }
        "not quoted" => "호가 없음",
        "RESTS" => "유효 기간",
        "RESTS AT" => "걸리는 가격",
        "WORKED" => "체결 기간",
        "CONFIRM" => "확인",
        "SEND IT" => "전송",
        "GO BACK" => "뒤로",
        "Go back without sending" => "보내지 않고 돌아가기",
        // Settings.
        "ACCOUNT" => "계정",
        "WATCHING" => "읽기 전용",
        "The address every panel on this screen is read for." => {
            "이 화면의 모든 패널이 읽고 있는 주소."
        }
        "Disconnects this address, clears its positions, orders and fills, and locks the key." => {
            "이 주소의 연결을 끊고, 포지션·주문·체결 내역을 비우고, 키를 잠급니다."
        }
        "NETWORK" => "네트워크",
        "An exchange and one of its deployments. They list different markets and know nothing of each other's orders." => {
            "거래소와 그 배포판 하나. 서로 다른 마켓을 상장하고, 서로의 주문을 전혀 모릅니다."
        }
        "Picking one points every panel at that network and throws away what this one filled them with." => {
            "하나를 고르면 모든 패널이 그 네트워크를 보고, 지금 채워진 내용은 버립니다."
        }
        "Picking one points every panel at that network and throws away what this one filled them with. A network is an exchange and one of its deployments: they list different markets, hold a position to different margin, and know nothing of each other's orders." => {
            "하나를 고르면 모든 패널이 그 네트워크를 보고, 지금 채워진 내용은 버립니다. 네트워크는 거래소와 그 배포판 하나입니다. 서로 다른 마켓을 상장하고, 포지션에 다른 증거금을 요구하며, 서로의 주문을 전혀 모릅니다."
        }
        "LANGUAGE" => "언어",
        "Every sentence on screen. The figures are the same figures." => {
            "화면의 모든 문장. 숫자는 같은 숫자입니다."
        }
        "FEED" => "피드",
        "One socket carries the mark, the book, the tape and the chart." => {
            "소켓 하나가 마크가, 호가, 체결, 차트를 모두 나릅니다."
        }
        "Nothing is arriving. Every price on screen is the last one that did." => {
            "아무것도 들어오지 않습니다. 화면의 가격은 모두 마지막으로 도착한 값입니다."
        }
        "ROUND TRIP" => "왕복 시간",
        "The round trip is the socket's own ping, not a clock compared with the exchange's." => {
            "왕복 시간은 소켓 자체의 핑이며, 거래소 시계와 비교한 값이 아닙니다."
        }
        "CUSTODY" => "키 보관",
        "Two keys, and only one of them can trade." => {
            "키는 둘이고, 거래할 수 있는 키는 하나뿐입니다."
        }
        "The address being read, the network it is read on, what this app may sign with, and how it is read." => {
            "읽고 있는 주소, 읽는 네트워크, 이 앱이 서명할 수 있는 키, 그리고 읽는 방식."
        }
        "SESSION" => "세션",
        "UNLOCK" => "잠금 해제",
        "LOCK" => "잠금",
        "Lock and forget the key" => "키를 잠그고 잊기",
        "ENROL ALL" => "전체 등록",
        "Register a trading key on every network, with one Touch ID" => {
            "Touch ID 한 번으로 모든 네트워크에 거래 키 등록"
        }
        "ONE KEY ON EACH OF" => "다음 네트워크마다 키 하나씩",
        "IMPORT A WALLET" => "지갑 가져오기",
        "Import a wallet" => "지갑 가져오기",
        "Import a wallet from a recovery phrase" => "복구 구문으로 지갑 가져오기",
        "THIS MACHINE'S PASSPHRASE" => "이 기기의 암호",
        "Passphrase for this machine's key file" => "이 기기의 키 파일 암호",
        "The Secure Enclave will not make a key for an unsigned build, so this app keeps its keys in a file it encrypts itself. This passphrase is the whole of what opens that file — weaker than Touch ID, which is the trade this machine is making, and nothing here can recover it if it is forgotten." => {
            "Secure Enclave는 서명되지 않은 빌드에 키를 만들어 주지 않으므로, 이 앱은 직접 암호화한 파일에 키를 보관합니다. 이 암호가 그 파일을 여는 전부입니다. Touch ID보다 약하며 그것이 이 기기가 감수하는 대가이고, 잊어버리면 여기서는 아무것도 복구할 수 없습니다."
        }
        "WHAT EACH KEY CAN DO" => "각 키가 할 수 있는 일",
        "The trading key is a separate keypair the account's own wallet approved at the exchange. It places and cancels orders, it cannot withdraw, and the exchange stops honouring it on a date the exchange chose. Losing it costs an approval, not a balance, and it is the only key an order is ever signed with." => {
            "거래 키는 계좌의 지갑이 거래소에서 승인한 별도의 키 쌍입니다. 주문을 내고 취소할 수 있지만 출금은 할 수 없고, 거래소가 정한 날짜에 효력을 잃습니다. 잃어버려도 잔고가 아니라 승인 한 번을 잃을 뿐이며, 주문에 서명하는 키는 오직 이것뿐입니다."
        }
        "On macOS its secret is held by the platform keychain behind Touch ID, not by this process and not in a file, and unlocking is that prompt. On a build without a keychain there is nowhere to keep it and nothing to unlock, which is what the session above says rather than something this paragraph decides. Locking forgets it, and so does connecting a different address. Switching network does not: one unlock releases every network this address has enrolled, and each of them still holds a key of its own." => {
            "macOS에서는 그 비밀을 이 프로세스나 파일이 아니라 Touch ID 뒤의 플랫폼 키체인이 보관하며, 잠금 해제가 곧 그 프롬프트입니다. 키체인이 없는 빌드에서는 보관할 곳도 해제할 것도 없고, 그것은 이 문단이 아니라 위의 세션이 말해 줍니다. 잠그면 키를 잊고, 다른 주소를 연결해도 잊습니다. 네트워크를 바꿔도 잊지 않습니다. 한 번의 잠금 해제가 이 주소가 등록한 모든 네트워크를 풀고, 네트워크마다 자기 키를 그대로 가집니다."
        }
        "Unlocking is what lets the ticket send. Every order still passes a confirmation that restates it and names the network it is going to, and the trading key it signs with can place and cancel orders and nothing else." => {
            "잠금 해제가 주문 전송을 허용합니다. 모든 주문은 여전히 내용을 되짚고 보낼 네트워크를 이름으로 밝히는 확인 단계를 거치며, 서명하는 거래 키는 주문을 내고 취소하는 것 외에는 아무것도 할 수 없습니다."
        }
        "Importing a wallet does put the account's own key on this Mac, behind Touch ID. It signs enrolments and nothing else — the app cannot spend it on an order even by mistake, because an order is a different type of thing and this key has no method that takes one. It never moves collateral and never withdraws." => {
            "지갑을 가져오면 계좌의 키가 Touch ID 뒤에, 이 Mac에 놓입니다. 그 키는 등록에만 서명합니다. 주문은 다른 종류의 것이고 이 키에는 주문을 받는 메서드가 없으므로, 앱이 실수로라도 주문에 쓸 수 없습니다. 담보를 옮기지도, 출금하지도 않습니다."
        }
        "KEYBOARD" => "키보드",
        "No key sends an order." => "어떤 키도 주문을 보내지 않습니다.",
        "What the keys do, and where they stop." => "키가 하는 일과, 멈추는 곳.",
        "Address" => "주소",
        // Connecting, importing, creating.
        "Browse markets only, with no account at all" => "계정 없이 마켓만 둘러보기",
        "Watch an address, read-only, without holding its key" => {
            "키 없이 주소를 읽기 전용으로 지켜보기"
        }
        "Import a wallet, and trade this account from this Mac" => {
            "지갑을 가져와 이 Mac에서 이 계좌로 거래하기"
        }
        "Create a wallet, and trade this new account from this Mac" => {
            "지갑을 만들어 이 Mac에서 새 계좌로 거래하기"
        }
        "Trade from this Mac" => "이 Mac에서 거래",
        "An address is 0x and forty hexadecimal digits." => "주소는 0x 뒤에 16진수 40자리입니다.",
        "Open positions, resting orders, and every fill marked on the chart, for any address on this network. Nothing on this path can sign, because watching an account is not owning one." => {
            "이 네트워크의 어떤 주소든, 보유 포지션과 미체결 주문, 그리고 차트에 표시된 모든 체결을 봅니다. 계좌를 지켜보는 것은 소유하는 것이 아니므로, 이 경로에서는 아무것도 서명할 수 없습니다."
        }
        "Import the wallet that owns the account and this app derives its address, registers a trading key on every network, and can place orders. The key is kept behind Touch ID on this machine and is never sent anywhere." => {
            "계좌를 소유한 지갑을 가져오면 이 앱이 주소를 유도하고, 모든 네트워크에 거래 키를 등록하며, 주문을 낼 수 있게 됩니다. 키는 이 기기의 Touch ID 뒤에 보관되며 어디로도 전송되지 않습니다."
        }
        "Make a wallet" => "지갑 만들기",
        "CREATE A WALLET" => "지갑 만들기",
        "Keep this wallet on this Mac, behind Touch ID" => "이 지갑을 Touch ID 뒤에, 이 Mac에 보관",
        "Close without importing" => "가져오지 않고 닫기",
        "Recovery phrase, or a private key" => "복구 구문 또는 개인 키",
        "Twelve to twenty-four words, or a private key. It is turned into the one key this app signs enrolments with, kept behind Touch ID, and never sent anywhere." => {
            "열두 개에서 스물네 개의 단어, 또는 개인 키. 이 앱이 등록에 서명하는 유일한 키로 바뀌어 Touch ID 뒤에 보관되며, 어디로도 전송되지 않습니다."
        }
        "Passphrase, if the wallet has one" => "지갑에 암호가 있다면 그 암호",
        "A passphrase makes different words into a different account. If your wallet asked for one, it belongs here." => {
            "암호는 같은 단어를 다른 계좌로 만듭니다. 지갑이 암호를 요구했다면 여기에 넣습니다."
        }
        "Show the account these words derive" => "이 단어들이 유도하는 계좌 보기",
        "CHECK" => "확인",
        "THIS MACHINE" => "이 기기",
        "This build cannot reach the Secure Enclave, so the key is sealed into a file with this passphrase instead. It is weaker than Touch ID and nothing here can recover it — write it down with the words." => {
            "이 빌드는 Secure Enclave에 닿을 수 없으므로, 키를 대신 이 암호로 파일에 봉인합니다. Touch ID보다 약하고 여기서는 아무것도 복구할 수 없으니, 단어들과 함께 적어 두세요."
        }
        "Nothing has been stored. If that is not the address you expect, go back and check the words." => {
            "아직 아무것도 저장되지 않았습니다. 예상한 주소가 아니라면 돌아가서 단어를 확인하세요."
        }
        "THIS IS MINE" => "내 계좌입니다",
        "CLOSE" => "닫기",
        "DO IT" => "실행",
        "DONE" => "완료",
        "THIS PHRASE IS THE ACCOUNT" => "이 구문이 곧 계좌입니다",
        "Twenty-four words, made on this machine from the system's own randomness. They are the account: this app keeps what they derive, sealed to this Mac, and it will not show the words again." => {
            "이 기기에서 시스템의 난수로 만든 스물네 개의 단어. 이것이 곧 계좌입니다. 이 앱은 단어가 유도한 키를 이 Mac에 봉인해 보관하며, 단어를 다시 보여 주지 않습니다."
        }
        "WRITE THIS DOWN" => "적어 두세요",
        "On paper. Not in a screenshot, not in a password manager's note field, not in a message to yourself. Anyone who reads these words owns this account, and nobody can take it back." => {
            "종이에. 스크린샷도, 비밀번호 관리자의 메모란도, 자기 자신에게 보내는 메시지도 아닙니다. 이 단어를 읽는 누구든 이 계좌를 소유하고, 아무도 되찾아 줄 수 없습니다."
        }
        "I'VE WRITTEN IT DOWN" => "적어 두었습니다",
        "I have written the words down" => "단어를 적어 두었습니다",
        "CHECK YOUR COPY" => "적은 것을 확인",
        "One box each, in the order they are numbered. The phrase is off the screen on purpose: this is the step that finds out whether it reached paper." => {
            "번호 순서대로 칸마다 하나씩. 구문은 일부러 화면에서 치웠습니다. 종이에 제대로 옮겨졌는지 알아내는 단계가 바로 이것입니다."
        }
        "Confirm the words you wrote down" => "적어 둔 단어 확인",
        "Nothing has been stored yet. This is the account those twenty-four words make — keep it, and this app can sign enrolments for it." => {
            "아직 아무것도 저장되지 않았습니다. 이것이 그 스물네 단어가 만드는 계좌입니다. 보관하면 이 앱이 이 계좌의 등록에 서명할 수 있습니다."
        }
        // The page tabs and pane toggles, composed in Rust.
        "Show the terminal page" => "터미널 페이지 열기",
        "Show the portfolio page" => "포트폴리오 페이지 열기",
        "Show the settings page" => "설정 페이지 열기",
        "Show the markets pane" => "마켓 패널 열기",
        "Hide the markets pane" => "마켓 패널 닫기",
        "Show the fills pane" => "체결 패널 열기",
        "Hide the fills pane" => "체결 패널 닫기",
        // The portfolio's range headings, in Rust.
        "LAST DAY" => "지난 하루",
        "LAST WEEK" => "지난 일주일",
        "LAST MONTH" => "지난 한 달",
        "ALL TIME" => "전체 기간",
        // The keyboard scheme, in Rust.
        "Buy / long" => "매수 / 롱",
        "Sell / short" => "매도 / 숏",
        "Size to 25%, 50%, 75%, all" => "수량을 25%, 50%, 75%, 전부로",
        "Move the limit price one tick" => "지정가를 한 틱 옮기기",
        "Review the order — in a ticket field" => "주문 확인 — 티켓 입력란에서",
        "Close an open picker, then the search" => "열린 선택창을 닫고, 그다음 검색창",
        "No key sends an order. The keys above reach the confirmation and stop there, and they are off entirely while one is open — SEND IT is pressed by hand. A field you are typing in keeps its own keystrokes, so these do nothing while the search box or a ticket field has the cursor." => {
            "어떤 키도 주문을 보내지 않습니다. 위의 키들은 확인 단계까지만 가서 멈추고, 확인창이 열려 있는 동안에는 아예 꺼집니다. 전송은 손으로 누릅니다. 입력 중인 칸은 자기 키 입력을 그대로 가지므로, 검색창이나 티켓 입력란에 커서가 있는 동안에는 아무 일도 하지 않습니다."
        }
        // Custody, in Rust.
        "Unlock with this machine's passphrase" => "이 기기의 암호로 잠금 해제",
        "Unlock with Touch ID" => "Touch ID로 잠금 해제",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_is_its_own_key() {
        assert_eq!(t(Locale::En, "EQUITY".into()), "EQUITY");
    }

    #[test]
    fn a_key_no_table_carries_reads_as_english() {
        assert_eq!(t(Locale::Ko, "no such sentence".into()), "no such sentence");
    }

    /// Every `t(locale, "...")` in the `.ice` sources, which is every sentence
    /// the view draws through this module.
    fn keys_the_view_draws() -> Vec<String> {
        let ui = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui");
        let mut sources = Vec::new();
        let mut pending = vec![ui];
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(&dir).expect("the ui directory reads") {
                let path = entry.expect("a directory entry reads").path();
                if path.is_dir() {
                    pending.push(path);
                } else if path.extension().is_some_and(|ext| ext == "ice") {
                    sources.push(std::fs::read_to_string(path).expect("an .ice file reads"));
                }
            }
        }
        let mut keys = Vec::new();
        for source in sources {
            let mut rest = source.as_str();
            while let Some(at) = rest.find("t(locale, \"") {
                rest = &rest[at + "t(locale, \"".len()..];
                let end = rest.find("\")").expect("a key literal closes");
                keys.push(rest[..end].to_owned());
                rest = &rest[end..];
            }
        }
        keys.sort();
        keys.dedup();
        assert!(keys.len() > 200, "the walk found {} keys", keys.len());
        keys
    }

    /// A sentence added to the view without its Korean fails here rather than
    /// in front of a reader who picked 한국어 and got an English hole.
    #[test]
    fn every_sentence_the_view_draws_has_its_korean() {
        let missing: Vec<String> = keys_the_view_draws()
            .into_iter()
            .filter(|key| ko(key).is_none())
            .collect();
        assert!(missing.is_empty(), "no Korean for: {missing:#?}");
    }

    /// And the Rust prose that reaches the screen through `t` — the keyboard
    /// scheme and the unlock button — is in the table too.
    #[test]
    fn the_rust_prose_has_its_korean() {
        let mut english = Vec::new();
        english.extend(
            crate::hotkeys::hotkey_list(Locale::En)
                .into_iter()
                .map(|key| key.act),
        );
        english.push(crate::hotkeys::hotkey_note(Locale::En));
        for vault in [true, false] {
            english.push(crate::custody::unlock_label(Locale::En, vault));
        }
        for page in ["TERMINAL", "PORTFOLIO", "SETTINGS"] {
            english.push(crate::hyperliquid::page_label(Locale::En, page.to_owned()));
        }
        for pane in ["MARKETS", "FILLS"] {
            for open in [true, false] {
                english.push(crate::hyperliquid::pane_label(
                    Locale::En,
                    pane.to_owned(),
                    open,
                ));
            }
        }
        for range in ["day", "week", "month", "all"] {
            english.push(crate::portfolio::range_heading(Locale::En, range));
        }
        for key in english {
            assert!(ko(&key).is_some(), "no Korean for {key:?}");
        }
    }

    /// Text the same in both languages is a deliberate decision, one arm each,
    /// and not a missing one: the table answers it rather than falling through.
    #[test]
    fn a_kept_english_token_is_a_table_answer_and_not_a_hole() {
        assert_eq!(ko("TWAP"), Some("TWAP"));
        assert_eq!(t(Locale::Ko, "SETTINGS".into()), "설정");
    }
}

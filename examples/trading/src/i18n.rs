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
///
/// A sentence Rust composed at runtime — a refusal naming a network, a
/// readout carrying a figure — is looked up as a template: the literal
/// words around its spliced values are matched against the table's
/// templates, the values are lifted out, each is translated on its own
/// (a network's kind, a nested reason), and they are put back into the
/// Korean template. The Rust that composes the sentence therefore never
/// learns the locale, and the English it stores in state stays the key a
/// later language switch can still answer.
pub fn t(locale: Locale, key: &str) -> String {
    match locale {
        Locale::En => key.to_owned(),
        Locale::Ko => korean(key, 0).unwrap_or_else(|| key.to_owned()),
    }
}

/// The Korean for `sentence`, exact first and then by template, or `None`
/// when the table has no answer at all — which is what lets a spliced value
/// report that it did not change.
fn korean(sentence: &str, depth: usize) -> Option<String> {
    if let Some(exact) = ko(sentence) {
        return Some(exact.to_owned());
    }
    // A line break is a boundary no spliced value crosses, so each line is
    // its own sentence: a sweep's count, then each row it named, then what
    // the venue said about it.
    if sentence.contains('\n') {
        let mut moved = false;
        let lines: Vec<String> = sentence
            .lines()
            .map(|line| {
                let translated = korean(line, depth);
                moved |= translated.is_some();
                translated.unwrap_or_else(|| line.to_owned())
            })
            .collect();
        return moved.then(|| lines.join("\n"));
    }
    duration(sentence).or_else(|| ko_template(sentence, depth))
}

/// `5m`, `4h`, `2d`, `3 days` — an age, a funding countdown, a candle width,
/// a key's remaining window — read with the unit in Korean. Only digits take
/// a unit: a word that happens to end in one of those letters is not a
/// duration.
fn duration(sentence: &str) -> Option<String> {
    let (n, unit) = [
        ("m", "분"),
        ("h", "시간"),
        ("d", "일"),
        (" minutes", "분"),
        (" hours", "시간"),
        (" days", "일"),
    ]
    .into_iter()
    .find_map(|(english, korean)| Some((sentence.strip_suffix(english)?, korean)))?;
    (!n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())).then(|| format!("{n}{unit}"))
}

/// A runtime sentence answered through a template.
///
/// `{name}` holes in the English template stand for the values Rust spliced
/// in; the literal segments between them must appear in the sentence, in
/// order, with the first anchored at the start and the last at the end. The
/// template whose literal text is the longest wins when more than one fits,
/// so a generic "{a}: {b}" cannot shadow a sentence that carries real words.
///
/// A template whose literal text holds no letter — `{a}, {b}`,
/// `{venue}: {reason}` — would fit almost any sentence, so it answers only
/// when one of the values it lifted out has a translation of its own, and
/// inside another template's hole only as a splitter of a joined list. Its
/// literal is punctuation, never a bare space: a sentence the table does not
/// know must come back untouched rather than with its last word swapped.
fn ko_template(sentence: &str, depth: usize) -> Option<String> {
    struct Fit<'t, 's> {
        literal: usize,
        values: Vec<(&'t str, &'s str)>,
        korean: &'t str,
        anchored: bool,
    }
    let mut best: Option<Fit<'_, '_>> = None;
    for (english, korean) in KO_TEMPLATES {
        let anchored = english
            .split(['{', '}'])
            .step_by(2)
            .any(|literal| literal.chars().any(char::is_alphabetic));
        // Inside another template's hole only a splitter — the same text on
        // both sides, `{a}, {b}` — may fit, to reach each item of a joined
        // list; any other unanchored template could only rearrange words.
        if !anchored && depth > 0 && english != korean {
            continue;
        }
        let Some(values) = match_template(english, sentence) else {
            continue;
        };
        let literal = english.len() - values.iter().map(|(name, _)| name.len() + 2).sum::<usize>();
        if best.as_ref().is_none_or(|fit| literal > fit.literal) {
            best = Some(Fit {
                literal,
                values,
                korean,
                anchored,
            });
        }
    }
    let Fit {
        values,
        korean,
        anchored,
        ..
    } = best?;
    let mut out = korean.to_owned();
    let mut moved = false;
    for (name, value) in values {
        let hole = format!("{{{name}}}");
        let translated = self::korean(value, depth + 1);
        moved |= translated.is_some();
        out = out.replace(&hole, translated.as_deref().unwrap_or(value));
    }
    (anchored || moved).then_some(out)
}

/// The values a sentence carries in a template's holes, or `None` if the
/// template's words are not the sentence's words. A hole is greedy only up
/// to the next literal segment, and an empty hole is allowed: Rust splices
/// empty strings too.
fn match_template<'t, 's>(template: &'t str, sentence: &'s str) -> Option<Vec<(&'t str, &'s str)>> {
    let mut values = Vec::new();
    let mut rest = sentence;
    let mut template = template;
    loop {
        let Some(open) = template.find('{') else {
            return (rest == template).then_some(values);
        };
        rest = rest.strip_prefix(&template[..open])?;
        let close = template[open..].find('}')? + open;
        let name = &template[open + 1..close];
        template = &template[close + 1..];
        let next_literal_end = template.find('{').unwrap_or(template.len());
        let next_literal = &template[..next_literal_end];
        let value_end = if next_literal.is_empty() {
            if template.is_empty() {
                rest.len()
            } else {
                // Two holes back to back cannot be split; the table never
                // writes them.
                return None;
            }
        } else if next_literal_end == template.len() {
            // The last literal is anchored at the end, so a value may carry
            // it: a price keeps its decimal point before the full stop.
            rest.strip_suffix(next_literal)?.len()
        } else {
            rest.find(next_literal)?
        };
        values.push((name, &rest[..value_end]));
        rest = &rest[value_end..];
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

/// Runtime sentences, as templates: the English Rust composes with its
/// spliced values as `{name}` holes, and the Korean with the same holes.
const KO_TEMPLATES: &[(&str, &str)] = &[
    // venue.rs
    (
        "Read {venue_name}, {spoken_kind}",
        "{venue_name}, {spoken_kind} 읽기",
    ),
    (
        "{venue_name}, {spoken_kind} — switch network",
        "{venue_name}, {spoken_kind} — 네트워크 전환",
    ),
    (
        "This account could not be read: {failure}",
        "이 계좌를 읽지 못했습니다: {failure}",
    ),
    (
        "No {venue_name} account for this address.",
        "이 주소에는 {venue_name} 계좌가 없습니다.",
    ),
    (
        "Reading this account on {venue_name}.",
        "{venue_name}에서 이 계좌를 읽는 중입니다.",
    ),
    (
        "Resting orders could not be read: {failure}",
        "미체결 주문을 읽지 못했습니다: {failure}",
    ),
    (
        "Fills could not be read: {failure}",
        "체결 내역을 읽지 못했습니다: {failure}",
    ),
    (
        "Wrote {count} fills to {path}",
        "체결 {count}건을 {path}에 저장했습니다",
    ),
    (
        "Could not write {path}: {cause}",
        "{path}에 쓰지 못했습니다: {cause}",
    ),
    (
        "{venue_name} has no rest-until-cancelled: the order carries a deadline it is signed with and expires there.",
        "{venue_name}에는 취소할 때까지 걸어 두는 주문이 없습니다: 주문은 서명할 때 정한 만료 시각을 갖고, 그 시각에 만료됩니다.",
    ),
    (
        "{venue_name} has a TWAP order and this app does not send one: its published SDK signs no such action, so there is nothing to hold these bytes against, and an order signed to a shape nobody has checked is one the exchange cannot tell from a stranger's. It is offered nowhere rather than offered here.",
        "{venue_name}에는 TWAP 주문이 있지만 이 앱은 보내지 않습니다: 공개된 SDK가 그런 액션을 서명하지 않아 이 앱의 바이트를 대조할 기준이 없고, 아무도 검증하지 않은 형태로 서명한 주문은 거래소가 낯선 사람의 주문과 구별할 수 없습니다. 여기서만 제공하기보다 어디서도 제공하지 않습니다.",
    ),
    (
        "{venue_name} does take a target and a stop on the entry, {missing}. They are offered nowhere rather than offered here: a field promising a position is protected, over an order that carries no protection, is the one mistake this panel must never make.",
        "{venue_name}에서는 진입 주문에 익절과 손절을 받습니다 — {missing}. 여기서만 제공하기보다 어디서도 제공하지 않습니다: 보호가 없는 주문 위에 포지션이 보호된다고 약속하는 입력란을 두는 것은 이 패널이 절대 저질러서는 안 되는 단 하나의 실수입니다.",
    ),
    ("over {minutes} minute", "{minutes}분에 걸쳐"),
    ("over {minutes} minutes", "{minutes}분에 걸쳐"),
    ("over {hours} hour", "{hours}시간에 걸쳐"),
    ("over {hours} hours", "{hours}시간에 걸쳐"),
    (
        "{coin} is margined against a clearinghouse this app cannot read, so it will not send an order there.",
        "{coin} 마켓은 이 앱이 읽을 수 없는 청산소에 증거금을 잡으므로, 이 앱은 그곳으로 주문을 보내지 않습니다.",
    ),
    (
        "Send this buy of {size} {coin} on {network}, {kind}",
        "{coin} {size} 매수 주문을 {network}({kind})에 전송",
    ),
    (
        "Send this sell of {size} {coin} on {network}, {kind}",
        "{coin} {size} 매도 주문을 {network}({kind})에 전송",
    ),
    ("Cancel {count} resting order", "미체결 주문 {count}건 취소"),
    (
        "Cancel {count} resting orders",
        "미체결 주문 {count}건 취소",
    ),
    ("Close {count} position", "포지션 {count}개 종료"),
    ("Close {count} positions", "포지션 {count}개 종료"),
    ("{sweep_act}, one confirmation", "{sweep_act}, 확인 한 번"),
    (
        "Each goes as a reduce-only order priced up to {slippage}% through its own mark, so it crosses rather than rests. This spends money and it is not reversible.",
        "각 포지션은 자기 마크가에서 최대 {slippage}%까지 넘어선 가격의 청산 전용 주문으로 나가므로, 걸리지 않고 바로 체결됩니다. 돈이 들며 되돌릴 수 없습니다.",
    ),
    (
        "Buy {size} {coin} in {count} orders",
        "{count}개 주문으로 {coin} {size} 매수",
    ),
    (
        "Sell {size} {coin} in {count} orders",
        "{count}개 주문으로 {coin} {size} 매도",
    ),
    (
        "A ladder is at most {max} orders: each rung is its own signature and its own round trip.",
        "사다리는 최대 {max}개의 주문입니다: 각 단이 별개의 서명이고 별개의 왕복입니다.",
    ),
    (
        "Close {coin} short {size} at up to {price}",
        "{coin} 숏 {size} 종료, {price}까지",
    ),
    (
        "Close {coin} long {size} at up to {price}",
        "{coin} 롱 {size} 종료, {price}까지",
    ),
    // hyperliquid.rs
    (
        "Hyperliquid unreachable: {error}",
        "Hyperliquid에 연결할 수 없습니다: {error}",
    ),
    (
        "Hyperliquid sent bad JSON: {error}",
        "Hyperliquid가 잘못된 JSON을 보냈습니다: {error}",
    ),
    ("{coin} {price} NOT LIVE", "{coin} {price} 끊김"),
    ("{price} TESTNET", "{price} 테스트넷"),
    ("{waiting} alert waiting", "알림 {waiting}건 대기"),
    ("{waiting} alerts waiting", "알림 {waiting}건 대기"),
    ("{hit} ALERT HIT", "알림 {hit}건 HIT"),
    ("{hit} ALERTS HIT", "알림 {hit}건 HIT"),
    (
        "{hit} ALERT HIT · {waiting} waiting",
        "알림 {hit}건 HIT · {waiting}건 대기",
    ),
    (
        "{hit} ALERTS HIT · {waiting} waiting",
        "알림 {hit}건 HIT · {waiting}건 대기",
    ),
    ("EQUITY  {value}", "순자산  {value}"),
    ("PNL  {pnl}", "손익  {pnl}"),
    ("+{more} more", "외 {more}개"),
    ("{count} open: {coins}", "{count}개 보유: {coins}"),
    ("FEED  {latency}", "피드  {latency}"),
    ("{venue_name} — TESTNET", "{venue_name} — 테스트넷"),
    ("{venue_name} — REAL MONEY", "{venue_name} — 실거래"),
    (
        "Hyperliquid answered {status} with {error}",
        "Hyperliquid가 {status} 응답을 보냈지만 본문을 읽을 수 없습니다: {error}",
    ),
    (
        "Hyperliquid feed unreachable: {error}",
        "Hyperliquid 피드에 연결할 수 없습니다: {error}",
    ),
    (
        "Hyperliquid feed unreadable: {error}",
        "Hyperliquid 피드를 읽을 수 없습니다: {error}",
    ),
    (
        "Hyperliquid feed refused: {error}",
        "Hyperliquid 피드가 요청을 거부했습니다: {error}",
    ),
    (
        "Hyperliquid feed rejected a request: {data}",
        "Hyperliquid 피드가 요청을 거절했습니다: {data}",
    ),
    (
        "Hyperliquid feed dropped: {error}",
        "Hyperliquid 피드가 끊어졌습니다: {error}",
    ),
    ("{leverage}x cross", "{leverage}x 교차"),
    ("{leverage}x isolated", "{leverage}x 격리"),
    ("Buy at {price}", "{price}에 매수"),
    ("Sell at {price}", "{price}에 매도"),
    ("liquidation {liq}", "청산가 {liq}"),
    (
        "{coin} long {size}, entry {entry}, {liq}, funding {funding}, unrealized {pnl} at {roe}",
        "{coin} 롱 {size}, 진입가 {entry}, {liq}, 펀딩 {funding}, 미실현 {pnl}, 수익률 {roe}",
    ),
    (
        "{coin} short {size}, entry {entry}, {liq}, funding {funding}, unrealized {pnl} at {roe}",
        "{coin} 숏 {size}, 진입가 {entry}, {liq}, 펀딩 {funding}, 미실현 {pnl}, 수익률 {roe}",
    ),
    (
        "Stop watching {coin} at {price}",
        "{coin} {price} 지켜보기 중단",
    ),
    (
        "Stop watching {coin} above {price}",
        "{coin} {price} 이상 지켜보기 중단",
    ),
    (
        "Stop watching {coin} below {price}",
        "{coin} {price} 이하 지켜보기 중단",
    ),
    ("{before} → {after}", "{before} → {after}"),
    ("{amount}/day", "{amount}/일"),
    ("Opens {size} long", "{size} 롱 진입"),
    ("Opens {size} short", "{size} 숏 진입"),
    (
        "Reduces your long to {remaining}",
        "보유 롱 일부 종료, {remaining} 남음",
    ),
    (
        "Reduces your short to {remaining}",
        "보유 숏 일부 종료, {remaining} 남음",
    ),
    (
        "Closes your long and opens {excess} short",
        "보유 롱 종료 후 {excess} 숏 진입",
    ),
    (
        "Closes your short and opens {excess} long",
        "보유 숏 종료 후 {excess} 롱 진입",
    ),
    (
        "A take-profit on a long sits above the {entry} it opens at.",
        "롱의 익절가는 진입가 {entry}보다 위에 있어야 합니다.",
    ),
    (
        "A take-profit on a short sits below the {entry} it opens at.",
        "숏의 익절가는 진입가 {entry}보다 아래에 있어야 합니다.",
    ),
    (
        "A stop-loss on a long sits below the {entry} it opens at.",
        "롱의 손절가는 진입가 {entry}보다 아래에 있어야 합니다.",
    ),
    (
        "A stop-loss on a short sits above the {entry} it opens at.",
        "숏의 손절가는 진입가 {entry}보다 위에 있어야 합니다.",
    ),
    (
        "The engine closes this long at {liquidation}, before that stop is reached.",
        "엔진이 그 손절가에 닿기 전에 {liquidation}에서 이 롱을 청산합니다.",
    ),
    (
        "The engine closes this short at {liquidation}, before that stop is reached.",
        "엔진이 그 손절가에 닿기 전에 {liquidation}에서 이 숏을 청산합니다.",
    ),
    (
        "Take profit price, {pnl} at that level",
        "익절 가격, 그 가격에서 {pnl}",
    ),
    (
        "Stop loss price, {pnl} at that level",
        "손절 가격, 그 가격에서 {pnl}",
    ),
    (
        "Set the size to {percent}% of this position",
        "수량을 이 포지션의 {percent}%로 설정",
    ),
    (
        "Set the size to {percent}% of your buying power",
        "수량을 매수 여력의 {percent}%로 설정",
    ),
    (
        "Crosses the spread now, at {paid}.",
        "지금 스프레드를 넘어 {paid}에 체결됩니다.",
    ),
    (
        "Crosses the spread. No book on screen to walk, so it is quoted at the venue's last, {seed}.",
        "스프레드를 넘어 체결됩니다. 화면에 훑을 호가가 없어 거래소 현재가인 {seed} 기준으로 잡았습니다.",
    ),
    (
        "Sized at {price}, the limit price.",
        "주문 가격 {price} 기준으로 환산했습니다.",
    ),
    (
        "Sized at {price}, the market's mid.",
        "마켓 중간가 {price} 기준으로 환산했습니다.",
    ),
    ("Show {interval} candles", "{interval} 캔들 보기"),
    (
        "{name} at {price}, {change} today",
        "{name} {price}, 오늘 {change}",
    ),
    (
        "{name} at {price}, {change} today, {category} market settled in {collateral}",
        "{name} {price}, 오늘 {change}, {collateral} 결제 {category} 마켓",
    ),
    (
        "{coin} buy {size} at {price}",
        "{coin} {price}에 {size} 매수",
    ),
    (
        "{coin} sell {size} at {price}",
        "{coin} {price}에 {size} 매도",
    ),
    (
        "Load this {order_label} into the ticket",
        "이 {order_label} 주문을 티켓에 불러오기",
    ),
    ("Cancel this {order_label}", "이 {order_label} 주문 취소"),
    (
        "{coin} bought {size} at {price}",
        "{coin} {price}에 {size} 매수 체결",
    ),
    (
        "{coin} sold {size} at {price}",
        "{coin} {price}에 {size} 매도 체결",
    ),
    (
        "{coin} bought {size} at {price}, realized {pnl}",
        "{coin} {price}에 {size} 매수 체결, 실현 {pnl}",
    ),
    (
        "{coin} sold {size} at {price}, realized {pnl}",
        "{coin} {price}에 {size} 매도 체결, 실현 {pnl}",
    ),
    ("{name} candlestick chart", "{name} 캔들 차트"),
    (
        "{name} candlestick chart; indicators: {studies}",
        "{name} 캔들 차트; 지표: {studies}",
    ),
    (
        "{name} candlestick chart; this account's fills marked",
        "{name} 캔들 차트; 이 계좌의 체결 표시",
    ),
    (
        "{name} candlestick chart; indicators: {studies}; this account's fills marked",
        "{name} 캔들 차트; 지표: {studies}; 이 계좌의 체결 표시",
    ),
    // custody.rs
    (
        "word {a}, word {b} and word {c}",
        "{a}번, {b}번, {c}번 단어",
    ),
    ("word {a} and word {b}", "{a}번과 {b}번 단어"),
    ("word {at}", "{at}번 단어"),
    ("Word {at}", "{at}번 단어"),
    ("Fill in {asks}.", "{asks}를 입력하십시오."),
    (
        "{asks} does not match what you wrote down. Check your copy — nothing has been stored.",
        "{asks}가 적어 둔 것과 일치하지 않습니다. 적은 것을 확인하십시오 — 아직 아무것도 저장되지 않았습니다.",
    ),
    (
        "This phrase is the account {address}. If that is not the address you expect, nothing has been stored — go back and check the words.",
        "이 구문이 곧 계좌 {address}입니다. 예상한 주소가 아니라면 아직 아무것도 저장되지 않았습니다 — 뒤로 돌아가 단어를 확인하십시오.",
    ),
    (
        "{address} is on this Mac now, {kept_by}. Enrol the networks you want to trade and this app can sign for itself.",
        "{address} 지갑이 이제 이 Mac에 {kept_by} 보관되어 있습니다. 거래할 네트워크를 등록하면 이 앱이 스스로 서명할 수 있습니다.",
    ),
    (
        "One Touch ID, and this app registers a key of its own on each network above for {address}. That signature is your account's. It approves trading keys and cannot withdraw.",
        "Touch ID 한 번으로, 이 앱이 계좌 {address}에 위의 네트워크마다 자체 키를 하나씩 등록합니다. 그 서명은 계좌 자체의 서명입니다. 거래 키를 승인할 뿐 출금은 할 수 없습니다.",
    ),
    ("{venue_name}: {reason}", "{venue_name}: {reason}"),
    ("{a}; {b}", "{a}; {b}"),
    ("{a}, {b}", "{a}, {b}"),
    (
        "Nothing was registered. {refused}",
        "아무것도 등록되지 않았습니다. {refused}",
    ),
    (
        "{which} did not take: {refused}. Registered on {landed}. Unlock to trade there.",
        "{which}는 등록에 실패했습니다: {refused}. {landed}에는 등록되었습니다. 거기서 거래하려면 잠금을 해제하십시오.",
    ),
    (
        "Registered on {landed}. Unlock and this app can sign for itself.",
        "{landed}에 등록되었습니다. 잠금을 해제하면 이 앱이 스스로 서명할 수 있습니다.",
    ),
    (
        "{ours} is not an approved API wallet for this account on {venue_name}. Approve it from the wallet that owns the account, then unlock again. The account can be read either way.",
        "{venue_name}에서 이 계좌에는 {ours}에 대한 API 지갑 승인이 없습니다. 계좌를 소유한 지갑에서 승인한 뒤 다시 잠금을 해제하십시오. 계좌 조회는 어느 쪽이든 가능합니다.",
    ),
    (
        "This address has no account on {venue_name} yet, so there is nothing for a key to sign for. Fund one there first — the app reads whatever it finds either way.",
        "이 주소에는 아직 {venue_name} 계좌가 없어, 키가 서명할 대상이 없습니다. 먼저 그곳에 입금해 계좌를 만드십시오 — 앱은 어느 쪽이든 찾는 대로 읽습니다.",
    ),
    (
        "This key is not registered against account {account} on {venue_name}. Register its public key from the wallet that owns the account, then unlock again — the app finds which slot you used. The account can be read either way. {ours}",
        "이 키는 {venue_name}의 계좌 {account}에 등록되어 있지 않습니다. 계좌를 소유한 지갑에서 이 키의 공개키를 등록한 뒤 다시 잠금을 해제하십시오 — 어느 슬롯을 썼는지는 앱이 찾아냅니다. 계좌 조회는 어느 쪽이든 가능합니다. {ours}",
    ),
    (
        "No key on this Mac for this account on {venue_name}. Make one on Settings, register it with the account's own wallet, then unlock.",
        "{venue_name}에서 쓸 이 계좌의 키가 이 Mac에 없습니다. 설정에서 키를 만들고, 계좌 소유 지갑으로 등록한 뒤 잠금을 해제하십시오.",
    ),
    (
        "This account has no key registered on {venue_name} yet. Settings says what to register and where.",
        "이 계좌에는 아직 {venue_name}에 등록된 키가 없습니다. 무엇을 어디에 등록할지는 설정에 나와 있습니다.",
    ),
    (
        "{acted} was submitted as order {placed}. It rests once the sequencer takes it.",
        "{acted}: {placed}번 주문으로 제출되었습니다. 시퀀서가 받아들이면 호가에 걸립니다.",
    ),
    (
        "Order {oid} on {coin} is cancelled.",
        "{coin}의 {oid}번 주문이 취소되었습니다.",
    ),
    (
        "Order {oid} on {coin} was sent for cancellation.",
        "{coin}의 {oid}번 주문에 취소 요청을 보냈습니다.",
    ),
    ("{sent} of {asked} {what}.", "{asked}건 중 {sent}건 {what}."),
    (
        "{venue_name} no longer recognises this app's trading key. Nothing was sent. Import is unaffected — enrol again from Settings and the account's own key will register a fresh one.",
        "{venue_name}에서 더 이상 이 앱의 거래 키를 인식하지 못합니다. 아무것도 전송되지 않았습니다. 가져온 지갑은 영향이 없습니다 — 설정에서 다시 등록하면 계좌 자체 키가 새 거래 키를 등록합니다.",
    ),
    (
        "{what}: {filled} of {size} filled at {px}, and the rest was cancelled.",
        "{what}: {px}에 {size} 중 {filled} 체결, 나머지는 취소되었습니다.",
    ),
    (
        "{what} filled at {px}.",
        "{what}: {px}에 전량 체결되었습니다.",
    ),
    (
        "{what}: {filled} of {size} filled at {px}, and {left} rests as order {oid}.",
        "{what}: {px}에 {size} 중 {filled} 체결, 나머지 {left} 수량은 {oid}번 주문으로 호가에 걸려 있습니다.",
    ),
    (
        "{what} was accepted, and the venue reported neither a fill nor a resting order.",
        "{what}: 접수되었으나 거래소가 체결도 미체결 주문도 보고하지 않았습니다.",
    ),
    (
        "{what} is resting as order {oid}.",
        "{what}: {oid}번 주문으로 호가에 걸려 있습니다.",
    ),
    ("Buy {size} {coin}", "{coin} {size} 매수"),
    ("Sell {size} {coin}", "{coin} {size} 매도"),
    (
        "The exchange stops honouring this key in {span}.",
        "거래소는 {span} 뒤에 이 키를 더 이상 받지 않습니다.",
    ),
    // lighter.rs
    (
        "Lighter unreachable: {error}",
        "Lighter에 연결할 수 없습니다: {error}",
    ),
    (
        "Lighter answered {path} with {status}: {error}",
        "Lighter가 {path}에 HTTP {status}로 응답했습니다: {error}",
    ),
    (
        "Lighter refused {path}: code {code} {message}",
        "Lighter가 {path} 요청을 거부했습니다: 코드 {code} {message}",
    ),
    (
        "Lighter does not list {coin}",
        "Lighter에는 {coin} 마켓이 없습니다",
    ),
    (
        "Lighter answered sendTx {status}: {error}",
        "Lighter가 sendTx에 HTTP {status}로 응답했습니다: {error}",
    ),
    (
        "Lighter refused the transaction: code {code}",
        "Lighter가 트랜잭션을 거부했습니다: 코드 {code}",
    ),
    (
        "Lighter refused the transaction: code {code} {said}",
        "Lighter가 트랜잭션을 거부했습니다: 코드 {code} {said}",
    ),
    (
        "{value} is not a {what} to send",
        "보낼 수 있는 {what}이 아닙니다: {value}",
    ),
    (
        "this market counts its {what} in steps of {step}, and {value} is not a whole number of them",
        "이 마켓의 {what} 단위는 {step}이며, {value}은(는) 그 단위의 정수 배가 아닙니다",
    ),
    (
        "Lighter numbers {coin} past what an order can name",
        "Lighter가 {coin}에 매긴 마켓 번호가 주문에 담을 수 있는 범위를 넘습니다",
    ),
    (
        "{price} is past the highest price this venue takes",
        "{price}은(는) 이 거래소가 받는 최고 가격을 넘습니다",
    ),
    (
        "Lighter quotes no {interval} candle: it has {widths}",
        "Lighter에는 {interval} 캔들이 없습니다: 제공하는 간격은 {widths}입니다",
    ),
    (
        "Lighter feed unreachable: {error}",
        "Lighter 피드에 연결할 수 없습니다: {error}",
    ),
    (
        "Lighter feed unreadable: {error}",
        "Lighter 피드를 읽을 수 없습니다: {error}",
    ),
    (
        "Lighter feed refused: {error}",
        "Lighter 피드가 전송을 거부했습니다: {error}",
    ),
    (
        "Lighter feed sent bad JSON: {error}",
        "Lighter 피드가 잘못된 JSON을 보냈습니다: {error}",
    ),
    (
        "Lighter feed rejected a request: code {code} {message}",
        "Lighter 피드가 구독 요청을 거부했습니다: 코드 {code} {message}",
    ),
    (
        "Lighter feed dropped: {error}",
        "Lighter 피드 연결이 끊겼습니다: {error}",
    ),
    // lighter_sign.rs
    (
        "a transaction's {name} cannot be negative",
        "트랜잭션의 {name} 값은 음수일 수 없습니다",
    ),
    // session.rs
    (
        "{doing}: {system} ({status})",
        "{doing}: {system} ({status})",
    ),
    (
        "{failure}; the secret it replaced is still there",
        "{failure}; 교체하려던 비밀 키는 그대로 남아 있습니다",
    ),
    (
        "{failure}; and {restore} — the secret it replaced is gone and has to be stored again",
        "{failure}; 그리고 {restore} — 교체하려던 비밀 키는 사라졌으므로 다시 저장해야 합니다",
    ),
    // vault.rs
    (
        "That passphrase does not open {path}. Nothing has changed. If it is lost, that file is the only thing holding these keys — delete it to start again, and everything in it is gone.",
        "그 암호로는 {path} 파일이 열리지 않습니다. 바뀐 것은 없습니다. 암호를 잃어버렸다면 이 키들을 담고 있는 것은 그 파일뿐입니다 — 파일을 삭제하면 처음부터 다시 시작하며, 안에 든 것은 모두 사라집니다.",
    ),
    (
        "could not read {target}: {cause}",
        "{target} 파일을 읽지 못했습니다: {cause}",
    ),
    (
        "{target} is not readable: {cause}",
        "{target} 파일을 해석할 수 없습니다: {cause}",
    ),
    (
        "{target} is not a key file this app wrote",
        "{target} 파일은 이 앱이 쓴 키 파일이 아닙니다",
    ),
    (
        "could not make {dir}: {cause}",
        "{dir} 디렉터리를 만들지 못했습니다: {cause}",
    ),
    (
        "could not write {staged}: {cause}",
        "{staged} 파일을 쓰지 못했습니다: {cause}",
    ),
    (
        "could not replace {target}: {cause}",
        "{target} 파일을 교체하지 못했습니다: {cause}",
    ),
    // seed.rs
    (
        "Word {position} is not a recovery-phrase word.",
        "{position}번째 단어는 복구 구문 단어가 아닙니다.",
    ),
    // signing.rs
    (
        "address must start with 0x: {text}",
        "주소는 0x로 시작해야 합니다: {text}",
    ),
    ("bad address: {error}", "잘못된 주소: {error}"),
    (
        "an address is 20 bytes: {text}",
        "주소는 20바이트입니다: {text}",
    ),
    (
        "not a usable secp256k1 key: {error}",
        "사용할 수 있는 secp256k1 키가 아닙니다: {error}",
    ),
    (
        "{value} is not a number the wire can carry",
        "{value}은(는) 전송할 수 있는 숫자가 아닙니다",
    ),
    (
        "{value} needs more than the 8 decimals the wire allows",
        "{value}에는 전송 한도인 소수점 8자리보다 많은 자릿수가 필요합니다",
    ),
    // portfolio.rs
    ("Open the {coin} market", "{coin} 마켓 열기"),
    ("Show account value over {span}", "{span}의 계좌 가치 보기"),
    // indicators.rs
    (
        "Choose chart indicators, {count} selected",
        "차트 지표 선택, {count}개 선택됨",
    ),
];

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
        "Loading" => "불러오는 중",
        "Loading candles" => "캔들 불러오는 중",
        "Sending" => "전송 중",
        "Cancelling" => "취소 중",
        "Quit" => "종료",
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
        "No data" => "데이터 없음",
        "Latest >" => "최신 >",
        // Input hints.
        "what opens the file" => "파일을 여는 암호",
        "abandon abandon abandon…" => "abandon abandon abandon…",
        "usually empty" => "보통 비워 둡니다",
        "chosen once, and not recoverable" => "한 번 정하면 되찾을 수 없습니다",
        "REAL MONEY" => "실거래",
        "TESTNET" => "테스트넷",
        // Sentences Rust composes at runtime, reached through the draw
        // sites rather than through a key in the view.
        // venue.rs
        "Hyperliquid" => "Hyperliquid",
        "Hyperliquid Testnet" => "Hyperliquid 테스트넷",
        "Lighter" => "Lighter",
        "Lighter Testnet" => "Lighter 테스트넷",
        "testnet" => "테스트넷",
        "real money" => "실거래",
        "This is Hyperliquid's test deployment. It answers every read the live one does, and it answers them about its own universe, its own books and its own accounts — so an address funded on mainnet has nothing here until it is funded again here, and nothing traded here is worth anything." => {
            "Hyperliquid의 테스트 배포입니다. 실거래 네트워크가 응답하는 모든 읽기에 똑같이 응답하지만, 그 대상은 자체 유니버스, 자체 호가창, 자체 계좌입니다 — 따라서 메인넷에서 입금한 주소라도 여기서 다시 입금하기 전까지는 아무것도 없고, 여기서 거래한 것은 아무 가치도 없습니다."
        }
        "Lighter serves resting orders and this account's fills only to an API-key-signed token, which an address alone cannot get and this app does not hold." => {
            "Lighter는 미체결 주문과 이 계좌의 체결 내역을 API 키로 서명한 토큰에만 제공합니다. 주소만으로는 그 토큰을 받을 수 없고, 이 앱은 그 토큰을 갖고 있지 않습니다."
        }
        "Historical performance on Lighter needs a read-only API token; this address-only session still shows current exposure." => {
            "Lighter의 과거 성과는 읽기 전용 API 토큰이 있어야 볼 수 있습니다. 주소만 연결한 이 세션에서도 현재 노출은 표시됩니다."
        }
        "This is Lighter's test deployment. It is a separate book with its own accounts and its own market ids — BTC is market 1 here and the live exchange lists two hundred more — so an account funded on mainnet has nothing here, and nothing traded here is worth anything. `GET /api/v1/faucet?l1_address=…` creates and funds one." => {
            "Lighter의 테스트 배포입니다. 자체 계좌와 자체 마켓 ID를 가진 별도의 장부입니다 — 여기서 BTC는 마켓 1이고, 실거래 거래소에는 그 위로 이백 개가 더 있습니다 — 따라서 메인넷에서 입금한 계좌라도 여기에는 아무것도 없고, 여기서 거래한 것은 아무 가치도 없습니다. `GET /api/v1/faucet?l1_address=…`로 계좌를 만들고 입금할 수 있습니다."
        }
        "No account is being read. Settings takes an address." => {
            "읽고 있는 계좌가 없습니다. 설정에서 주소를 입력합니다."
        }
        "No resting orders." => "미체결 주문이 없습니다.",
        "Orders need an address." => "주문을 보려면 주소가 필요합니다.",
        "No fills on this account yet." => "이 계좌에는 아직 체결 내역이 없습니다.",
        "Fills need an address." => "체결 내역을 보려면 주소가 필요합니다.",
        "No fills to export." => "내보낼 체결 내역이 없습니다.",
        "GTC" => "GTC",
        "GTT" => "GTT",
        "IOC" => "IOC",
        "ALO" => "ALO",
        "Rest until cancelled" => "취소할 때까지 걸어 두기",
        "Rest until its deadline" => "만료 시각까지 걸어 두기",
        "Fill now or cancel the rest" => "즉시 체결하고 남은 수량은 취소",
        "Rest only; cancel if it would cross" => "걸어 두기만; 스프레드를 넘게 되면 취소",
        "as a grouped transaction this app does not sign" => {
            "다만 이 앱이 서명하지 않는 묶음 트랜잭션으로만 받습니다"
        }
        "and this app does not send them yet" => "다만 이 앱은 아직 보내지 않습니다",
        "Choose a market first." => "먼저 마켓을 선택하세요.",
        "This market is not loaded here." => "이 마켓은 여기에 불러와 있지 않습니다.",
        "This app does not attach a target or a stop to an order yet, so it will not send one that has them." => {
            "이 앱은 아직 주문에 익절이나 손절을 붙이지 않으므로, 그것이 설정된 주문은 보내지 않습니다."
        }
        "This order has no size yet." => "이 주문에는 아직 수량이 없습니다.",
        "This order has no price yet." => "이 주문에는 아직 가격이 없습니다.",
        "cross" => "교차",
        "isolated" => "격리",
        "REVIEW BUY" => "매수 확인",
        "REVIEW SELL" => "매도 확인",
        "These margin figures are worked out here, for the mode and leverage above. Neither is sent with the order: both are settings the exchange keeps per market on your account, and the position opens at whatever they say." => {
            "이 증거금 수치는 위의 마진 모드와 레버리지를 기준으로 여기서 계산한 것입니다. 둘 다 주문과 함께 전송되지 않습니다: 둘 다 거래소가 계좌의 마켓별로 보관하는 설정이며, 포지션은 그 설정대로 열립니다."
        }
        "cancelled" => "취소",
        "closed" => "종료",
        "placed" => "제출",
        "No resting orders to cancel." => "취소할 미체결 주문이 없습니다.",
        "No open positions to close." => "종료할 포지션이 없습니다.",
        "Every order below stops resting. Nothing is bought or sold: a cancelled order that had not filled leaves no position behind, and one that had filled has already left one." => {
            "아래의 모든 주문이 호가창에서 내려갑니다. 매수나 매도는 일어나지 않습니다: 체결되지 않았던 주문은 취소해도 포지션을 남기지 않고, 이미 체결된 주문은 이미 포지션을 남겼습니다."
        }
        "Each rung below is its own limit order, signed and sent one after another. A rung the venue refuses is named on its own — the ones that went are already resting, and this panel does not take them back." => {
            "아래의 각 단은 별개의 지정가 주문이며, 하나씩 차례로 서명해 전송합니다. 거래소가 거부한 단은 따로 표시됩니다 — 이미 나간 단은 이미 걸려 있고, 이 패널은 그것을 되돌리지 않습니다."
        }
        "RANGE" => "구간",
        "PER ORDER" => "주문당",
        "A ladder is a whole number of orders." => "사다리의 주문 개수는 정수여야 합니다.",
        "A ladder is at least two orders. One order at one price is a limit order." => {
            "사다리는 최소 두 개의 주문입니다. 한 가격에 주문 하나는 지정가 주문입니다."
        }
        "This ladder has no price range yet." => "이 사다리에는 아직 가격 구간이 없습니다.",
        "Both ends of the range are the same price, so there is nothing to spread over." => {
            "구간의 양 끝이 같은 가격이라 나누어 걸 구간이 없습니다."
        }
        "now" => "지금",
        // hyperliquid.rs
        "Hyperliquid unreachable: no wire under test" => {
            "Hyperliquid에 연결할 수 없습니다: 테스트 중이라 네트워크가 닫혀 있습니다"
        }
        "Hyperliquid universe read failed" => "Hyperliquid 마켓 목록을 읽지 못했습니다",
        "separate margin account" => "별도 증거금 계좌",
        "needs the account it is held against" => "담보 계좌를 읽어야 계산됩니다",
        "no requirement stated" => "유지 증거금 미공시",
        "not listed here" => "이 네트워크에는 없음",
        "market not loaded" => "마켓 미로드",
        "No alerts" => "알림 없음",
        "Account — no address" => "계좌 — 주소 없음",
        "Account" => "계좌",
        "Account — NOT LIVE" => "계좌 — 끊김",
        "EQUITY  —" => "순자산  —",
        "PNL  —" => "손익  —",
        "No open positions" => "보유 포지션 없음",
        "FEED  NOT LIVE" => "피드  끊김",
        "Hyperliquid refused the action and said nothing" => {
            "Hyperliquid가 요청을 거부했고 이유를 밝히지 않았습니다"
        }
        "Hyperliquid accepted the request and reported nothing about the order" => {
            "Hyperliquid가 요청을 받아들였지만 주문에 대해 아무것도 보고하지 않았습니다"
        }
        "There is nothing in this order to send." => "이 주문에는 보낼 내용이 없습니다.",
        "Hyperliquid feed unreachable: no wire under test" => {
            "Hyperliquid 피드에 연결할 수 없습니다: 테스트 중이라 네트워크가 닫혀 있습니다"
        }
        "Unknown Hyperliquid transport" => "알 수 없는 Hyperliquid 전송 방식입니다",
        "no liquidation price" => "청산가 없음",
        "This market has no price yet to watch a level against." => {
            "이 마켓에는 아직 가격이 없어 지켜볼 가격을 정할 수 없습니다."
        }
        "A level is a price above zero." => "지켜볼 가격은 0보다 커야 합니다.",
        "That level is where this market is trading now." => {
            "그 가격은 지금 이 마켓이 거래되는 가격입니다."
        }
        "That level is already being watched." => "그 가격은 이미 지켜보고 있습니다.",
        "Closes your long" => "보유 롱 종료",
        "Closes your short" => "보유 숏 종료",
        "Reduce-only needs a position to reduce, and there is none in this market." => {
            "청산 전용은 줄일 포지션이 있어야 하는데, 이 마켓에는 포지션이 없습니다."
        }
        "This order adds to the long you hold. Reduce-only sends nothing rather than a smaller order." => {
            "이 주문은 보유 중인 롱을 늘립니다. 청산 전용은 주문을 줄여서 보내지 않고 아예 보내지 않습니다."
        }
        "This order adds to the short you hold. Reduce-only sends nothing rather than a smaller order." => {
            "이 주문은 보유 중인 숏을 늘립니다. 청산 전용은 주문을 줄여서 보내지 않고 아예 보내지 않습니다."
        }
        "There is no entry price yet to set a target against." => {
            "익절가를 정할 진입가가 아직 없습니다."
        }
        "A take-profit is a price above zero." => "익절가는 0보다 큰 가격이어야 합니다.",
        "There is no entry price yet to set a stop against." => {
            "손절가를 정할 진입가가 아직 없습니다."
        }
        "A stop-loss is a price above zero." => "손절가는 0보다 큰 가격이어야 합니다.",
        "Take profit" => "익절",
        "Stop loss" => "손절",
        "Take profit price" => "익절 가격",
        "Stop loss price" => "손절 가격",
        "Set the size to all of this position" => "수량을 이 포지션 전부로 설정",
        "Set the size to all of your buying power" => "수량을 매수 여력 전부로 설정",
        "Cross margin: this order is backed by the whole account and goes when the account does, at the requirement drawn under the equity figure. Everything else held cross moves that line." => {
            "교차 마진: 이 주문은 계좌 전체가 받치며, 순자산 아래에 그어진 필요 증거금 선에서 계좌와 함께 청산됩니다. 교차로 보유한 다른 모든 포지션이 그 선을 움직입니다."
        }
        "Isolated margin: this order stands on the requirement above and on nothing else, at the maintenance this market holds. The rest of the account is untouched by it." => {
            "격리 마진: 이 주문은 위의 필요 증거금만으로 서고 다른 것에는 기대지 않으며, 이 마켓의 유지 증거금에서 청산됩니다. 계좌의 나머지는 영향을 받지 않습니다."
        }
        "Crosses the spread. Nothing on screen prices it yet." => {
            "스프레드를 넘어 체결됩니다. 아직 화면에 가격을 매길 정보가 없습니다."
        }
        "Nothing on screen prices this market yet, so dollars cannot be sized." => {
            "아직 화면에 이 마켓의 가격이 없어 달러를 수량으로 환산할 수 없습니다."
        }
        "Hyperliquid unreachable" => "Hyperliquid에 연결할 수 없습니다",
        "Hyperliquid feed dropped" => "Hyperliquid 피드가 끊어졌습니다",
        // custody.rs
        "that phrase is too long to check" => "이 구문은 너무 길어 확인할 수 없습니다",
        "could not choose the words to check" => "확인할 단어를 고르지 못했습니다",
        "There is no phrase waiting to be confirmed." => "확인을 기다리는 구문이 없습니다.",
        "That is not 32 bytes of hex." => "32바이트 16진수가 아닙니다.",
        "There is no wallet waiting to be stored." => "저장을 기다리는 지갑이 없습니다.",
        "in a file only that passphrase opens" => "그 암호로만 열리는 파일에",
        "behind Touch ID" => "Touch ID 뒤에",
        "this machine would not produce randomness for a key" => {
            "이 기기가 키에 쓸 난수를 만들어 주지 않았습니다"
        }
        "could not generate a usable key" => "쓸 수 있는 키를 만들지 못했습니다",
        "A key belongs to one account, so connect an address before enrolling." => {
            "키는 한 계좌에 속하므로, 등록하기 전에 주소를 먼저 연결하십시오."
        }
        "No wallet on this Mac for this address yet. Import one first." => {
            "이 주소의 지갑이 아직 이 Mac에 없습니다. 먼저 지갑을 가져오십시오."
        }
        "Touch ID was cancelled, so nothing was signed and nothing was registered." => {
            "Touch ID가 취소되어, 아무것도 서명되지 않았고 아무것도 등록되지 않았습니다."
        }
        "the stored wallet is not an account key; import one again" => {
            "저장된 지갑이 계좌 키가 아닙니다. 지갑을 다시 가져오십시오"
        }
        "One network" => "네트워크 하나",
        "Some networks" => "일부 네트워크",
        "There are no networks to enrol on." => "등록할 네트워크가 없습니다.",
        "this address has no account here yet" => "이 주소에는 아직 이곳에 계좌가 없습니다",
        "A key belongs to one account, so connect an address before unlocking." => {
            "키는 한 계좌에 속하므로, 잠금을 해제하기 전에 주소를 먼저 연결하십시오."
        }
        "the stored secret is not a key for this network; make a new one to replace it" => {
            "저장된 비밀값이 이 네트워크의 키가 아닙니다. 새로 만들어 교체하십시오"
        }
        "No key on this Mac for this account and this network yet. Make one, register it with the account's own wallet, then unlock." => {
            "이 계좌와 이 네트워크의 키가 아직 이 Mac에 없습니다. 키를 만들고, 계좌 소유 지갑으로 등록한 뒤 잠금을 해제하십시오."
        }
        "Touch ID was cancelled, so nothing was released. Unlock again when you are ready." => {
            "Touch ID가 취소되어 아무것도 풀리지 않았습니다. 준비되면 다시 잠금을 해제하십시오."
        }
        "the stored secret is not a key for this network; make a new one" => {
            "저장된 비밀값이 이 네트워크의 키가 아닙니다. 새로 만드십시오"
        }
        "Waiting for the platform's prompt." => "플랫폼의 인증 창을 기다리는 중입니다.",
        "This key's window has closed. Unlock again before sending an order." => {
            "이 키의 유효 기간이 끝났습니다. 주문을 보내기 전에 다시 잠금을 해제하십시오."
        }
        "Unlock on Settings before sending an order." => {
            "주문을 보내기 전에 설정에서 잠금을 해제하십시오."
        }
        "There is no confirmed order to send." => "보낼 확인된 주문이 없습니다.",
        "The key this session holds is not for this network." => {
            "이 세션이 쥔 키는 이 네트워크용이 아닙니다."
        }
        "There is no confirmed sweep to send." => "보낼 확인된 일괄 작업이 없습니다.",
        "UNLOCKED" => "잠금 해제됨",
        "UNLOCKING" => "잠금 해제 중",
        "KEY EXPIRED" => "키 만료",
        "READ ONLY" => "읽기 전용",
        "This key's window has closed. Approve it again to keep trading." => {
            "이 키의 유효 기간이 끝났습니다. 계속 거래하려면 다시 승인하십시오."
        }
        "no platform keychain on this build" => "이 빌드에는 플랫폼 키체인이 없습니다",
        // lighter.rs
        "Lighter unreachable: no wire under test" => {
            "Lighter에 연결할 수 없습니다: 테스트 중이라 네트워크가 닫혀 있습니다"
        }
        "Lighter answered the nonce request without a nonce in it" => {
            "Lighter가 nonce 요청에 nonce 없이 응답했습니다"
        }
        "Lighter accepted the transaction and named no hash for it" => {
            "Lighter가 트랜잭션을 받았으나 해시를 돌려주지 않았습니다"
        }
        "size" => "수량",
        "price" => "가격",
        "Lighter feed unreachable: no wire under test" => {
            "Lighter 피드에 연결할 수 없습니다: 테스트 중이라 네트워크가 닫혀 있습니다"
        }
        "Unknown Lighter transport" => "알 수 없는 Lighter 전송 방식입니다",
        // lighter_sign.rs
        "private key must be 40 bytes of hex, and not zero mod the order" => {
            "개인 키는 16진수 40바이트여야 하며, 곡선 위수로 나눈 나머지가 0이어서는 안 됩니다"
        }
        "auth message is not a canonical field element" => "인증 메시지가 정규 체 원소가 아닙니다",
        "deadline is not in the future" => "만료 시각이 미래가 아닙니다",
        "deadline is more than 8h ahead" => "만료 시각이 8시간 넘게 앞서 있습니다",
        "twap window" => "TWAP 실행 기간",
        "public key" => "공개 키",
        "chain id" => "체인 ID",
        "transaction type" => "트랜잭션 유형",
        "nonce" => "논스",
        "deadline" => "만료 시각",
        "account index" => "계정 인덱스",
        "api key index" => "API 키 인덱스",
        "market index" => "마켓 인덱스",
        "client order index" => "클라이언트 주문 인덱스",
        "base amount" => "기초자산 수량",
        "side" => "방향",
        "order type" => "주문 유형",
        "time in force" => "주문 유효 기간",
        "reduce-only flag" => "청산 전용 플래그",
        "trigger price" => "트리거 가격",
        "order expiry" => "주문 만료 시각",
        "order index" => "주문 인덱스",
        // session.rs
        "This build is not code-signed, and the Secure Enclave will not make a key for an unsigned binary. Nothing has been stored. Build, sign and run in one step with `scripts/sign-dev.sh -p trading-example`." => {
            "이 빌드는 코드 서명이 되어 있지 않고, Secure Enclave는 서명되지 않은 바이너리에 키를 만들어 주지 않습니다. 아무것도 저장되지 않았습니다. `scripts/sign-dev.sh -p trading-example`로 빌드, 서명, 실행을 한 번에 하십시오."
        }
        "building the Touch ID guard" => "Touch ID 보호 설정 중",
        "storing the secret" => "비밀 키 저장 중",
        "reading the secret being replaced" => "교체될 비밀 키 읽는 중",
        "reading the secret" => "비밀 키 읽는 중",
        "building the wrapping key's guard" => "래핑 키 보호 설정 중",
        "looking for the wrapping key" => "래핑 키 찾는 중",
        "making the wrapping key in the Secure Enclave" => "Secure Enclave에 래핑 키 생성 중",
        "the wrapping key has no public half" => "래핑 키에 공개 키 부분이 없습니다",
        "sealing the secret" => "비밀 키 봉인 중",
        "opening the sealed secret" => "봉인된 비밀 키 여는 중",
        // vault.rs
        "This build cannot reach the Secure Enclave, so this app keeps keys in a file it encrypts itself. Choose a passphrase for it below — it is what opens the file, and nothing on this machine can recover it for you." => {
            "이 빌드는 Secure Enclave에 접근할 수 없어, 이 앱은 키를 직접 암호화한 파일에 보관합니다. 아래에서 그 파일의 암호를 정하십시오 — 이 암호만이 파일을 열 수 있으며, 이 기기의 어떤 것도 대신 복구해 주지 않습니다."
        }
        "the key file asks for no work at all" => "키 파일의 키 유도 반복 횟수가 0입니다",
        "the derived key is the wrong length" => "유도된 키의 길이가 맞지 않습니다",
        "the nonce is the wrong length" => "논스 길이가 맞지 않습니다",
        "the secret could not be sealed" => "비밀 키를 봉인하지 못했습니다",
        // seed.rs
        "A recovery phrase is 12, 15, 18, 21 or 24 words." => {
            "복구 구문은 12, 15, 18, 21 또는 24개 단어입니다."
        }
        "That phrase does not check out — a word is wrong or two are swapped. Nothing was stored." => {
            "그 구문은 검증에 실패했습니다 — 단어가 틀렸거나 두 단어의 순서가 바뀌었습니다. 아무것도 저장되지 않았습니다."
        }
        "The passphrase has to be plain ASCII here." => "여기서는 암호가 순수 ASCII여야 합니다.",
        "That phrase does not derive a usable key on this path." => {
            "그 구문은 이 경로에서 사용 가능한 키를 유도하지 않습니다."
        }
        "This machine would not produce randomness for a new phrase, so none was made." => {
            "이 기기가 새 구문에 쓸 난수를 만들어 주지 않아, 구문을 만들지 않았습니다."
        }
        // portfolio.rs
        "LONG" => "LONG",
        "SHORT" => "SHORT",
        "the last day" => "지난 하루",
        "the last week" => "지난 일주일",
        "the last month" => "지난 한 달",
        "its whole history" => "전체 기간",
        "Connect an address to load portfolio performance." => {
            "주소를 연결하면 포트폴리오 성과를 불러옵니다."
        }
        // indicators.rs
        "SMA 20" => "SMA 20",
        "SMA 60" => "SMA 60",
        "EMA 20" => "EMA 20",
        "BB 20 / 2σ" => "BB 20 / 2σ",
        "VWMA 20" => "VWMA 20",
        "Hide the SMA 20 indicator" => "SMA 20 지표 숨기기",
        "Show the SMA 20 indicator" => "SMA 20 지표 표시",
        "Hide the SMA 60 indicator" => "SMA 60 지표 숨기기",
        "Show the SMA 60 indicator" => "SMA 60 지표 표시",
        "Hide the EMA 20 indicator" => "EMA 20 지표 숨기기",
        "Show the EMA 20 indicator" => "EMA 20 지표 표시",
        "Hide the Bollinger Bands 20, two standard deviations indicator" => {
            "볼린저 밴드 20, 표준편차 2 지표 숨기기"
        }
        "Show the Bollinger Bands 20, two standard deviations indicator" => {
            "볼린저 밴드 20, 표준편차 2 지표 표시"
        }
        "Hide the VWMA 20 indicator" => "VWMA 20 지표 숨기기",
        "Show the VWMA 20 indicator" => "VWMA 20 지표 표시",
        // hotkeys.rs
        "B" => "B",
        "S" => "S",
        "Enter" => "Enter",
        "Esc" => "Esc",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_is_its_own_key() {
        assert_eq!(t(Locale::En, "EQUITY"), "EQUITY");
    }

    #[test]
    fn a_key_no_table_carries_reads_as_english() {
        assert_eq!(t(Locale::Ko, "no such sentence"), "no such sentence");
    }

    /// Every `t(locale, "...")` in the `.ice` sources, which is every sentence
    /// the view draws through this module — and every literal a handler or a
    /// preset stores into the state the status strip draws through it.
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
            for line in source.lines() {
                let Some((field, value)) = line.trim_start().split_once(" = \"") else {
                    continue;
                };
                if matches!(field, "status" | "error" | "feed_error" | "import_note")
                    && let Some(key) = value.strip_suffix('"')
                    && !key.is_empty()
                {
                    keys.push(key.to_owned());
                }
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

    #[test]
    fn a_template_lifts_its_values_and_translates_each() {
        assert_eq!(
            match_template(
                "{venue}: {reason}",
                "Lighter: this address has no account here yet"
            ),
            Some(vec![
                ("venue", "Lighter"),
                ("reason", "this address has no account here yet")
            ])
        );
        assert_eq!(match_template("{venue}: {reason}", "no colon here"), None);
        assert_eq!(
            match_template("Size to {share}", "Size to 25%"),
            Some(vec![("share", "25%")])
        );
        assert_eq!(match_template("Size to {share}", "Sized to 25%"), None);
        assert_eq!(
            match_template("{a} and {b}.", "x and y."),
            Some(vec![("a", "x"), ("b", "y")])
        );
        assert_eq!(match_template("{a} and {b}.", "x and y"), None);
    }

    #[test]
    fn the_longest_literal_template_wins_and_holes_are_translated_recursively() {
        // "REAL MONEY" inside a hole is itself a table answer.
        let spliced = format!("Hyperliquid — {}", "REAL MONEY");
        let out = t(Locale::Ko, &spliced);
        assert!(!out.contains("REAL MONEY"), "{out}");
        assert!(out.starts_with("Hyperliquid"), "{out}");
    }

    /// Every runtime template keeps its holes on both sides — a hole the
    /// Korean dropped would lose a figure, one it invented would print as
    /// braces — and the sides are free of the shapes the matcher cannot
    /// take apart.
    #[test]
    fn every_runtime_template_keeps_its_holes_on_both_sides() {
        fn holes(text: &str) -> Vec<&str> {
            let mut found: Vec<&str> = text
                .split('{')
                .skip(1)
                .map(|after| after.split('}').next().expect("an opened hole closes"))
                .collect();
            found.sort_unstable();
            found
        }
        for (english, korean) in KO_TEMPLATES {
            assert!(
                !english.is_empty() && !holes(english).is_empty(),
                "{english:?} has no hole"
            );
            assert_eq!(
                holes(english),
                holes(korean),
                "holes differ for {english:?}"
            );
            assert!(
                !english.contains("}{"),
                "{english:?} has two holes back to back"
            );
            assert!(
                !english
                    .trim_matches(|c: char| c == '{' || c == '}')
                    .is_empty(),
                "{english:?} is a bare hole"
            );
            // A template the same on both sides answers only through the
            // values it lifts; one whose literal is a bare space would lift
            // the words of any sentence and swap the last one it knows.
            assert!(
                english != korean || shape(english).trim().contains(|c: char| !c.is_whitespace()),
                "{english:?} splits on whitespace alone"
            );
        }
        // Two English sides of one shape would match the same sentences, and
        // only table order would say which Korean answers.
        let mut shapes: Vec<String> = KO_TEMPLATES.iter().map(|(en, _)| shape(en)).collect();
        shapes.sort_unstable();
        let before = shapes.len();
        shapes.dedup();
        assert_eq!(before, shapes.len(), "two templates share a shape");
    }

    /// A template with every hole renamed, so two that match the same
    /// sentences compare equal.
    fn shape(template: &str) -> String {
        let mut out = String::new();
        let mut rest = template;
        while let Some(open) = rest.find('{') {
            out.push_str(&rest[..open]);
            out.push_str("{}");
            rest = &rest[rest[open..].find('}').expect("a hole closes") + open + 1..];
        }
        out.push_str(rest);
        out
    }

    /// The last literal is anchored at the end of the sentence, so a price's
    /// decimal point does not end the hole before it: the receipt of a
    /// filled order and the market ticket's note both end in `{price}.`.
    #[test]
    fn a_price_keeps_its_decimal_point_before_the_full_stop() {
        assert_eq!(
            match_template("{a} at {b}.", "x at 1.50."),
            Some(vec![("a", "x"), ("b", "1.50")])
        );
        let receipt = t(Locale::Ko, "Buy 0.5 BTC filled at 64,000.00.");
        assert!(receipt.contains("64,000.00"), "{receipt}");
        assert!(!receipt.contains("filled"), "{receipt}");
        assert!(receipt.starts_with("BTC 0.5 매수"), "{receipt}");
        let note = t(Locale::Ko, "Crosses the spread now, at 64,001.00.");
        assert!(note.contains("64,001.00에"), "{note}");
        assert!(!note.contains("spread"), "{note}");
    }

    /// A venue's own words come back whole: the table knows single words
    /// such as "cancelled" and "nonce", and a sentence that merely ends in
    /// one is not a sentence it knows.
    #[test]
    fn a_sentence_the_table_does_not_know_keeps_its_last_word() {
        for raw in [
            "Order already cancelled",
            "invalid nonce",
            "Touch ID was cancelled: LocalAuthentication (-2)",
            // A template that is a hole followed by one common word would
            // swallow a venue's whole sentence into the hole, so the table
            // has no `{name} price`; its leading-hole templates end in
            // words no venue sentence does.
            "Order price cannot be more than 80% away from the reference price",
            "Something went wrong with cross",
            "Rate limited for the next 2 hours",
        ] {
            assert_eq!(t(Locale::Ko, raw), raw);
        }
        assert_eq!(t(Locale::Ko, "40x cross"), "40x 교차");
        assert_eq!(
            t(
                Locale::Ko,
                &crate::hyperliquid::level_label("Stop loss".to_owned(), 0.0)
            ),
            "손절 가격"
        );
        assert_eq!(
            t(
                Locale::Ko,
                &crate::hyperliquid::tray_venue(crate::Venue::LighterTestnet)
            ),
            "Lighter 테스트넷 — 테스트넷"
        );
    }

    /// An age, a countdown and a candle width all read with the unit in
    /// Korean, and only a number takes a unit.
    #[test]
    fn a_duration_reads_with_its_unit_in_korean() {
        assert_eq!(t(Locale::Ko, "5m"), "5분");
        assert_eq!(
            t(Locale::Ko, &crate::hyperliquid::fmt_age(1_000, 8_200)),
            "2시간"
        );
        assert_eq!(
            t(Locale::Ko, &crate::hyperliquid::fmt_age(1_000, 200_000)),
            "2일"
        );
        assert_eq!(t(Locale::Ko, "Hyperliquid"), "Hyperliquid");
        assert_eq!(t(Locale::Ko, "m"), "m");
        assert_eq!(t(Locale::Ko, "3 days"), "3일");
        assert_eq!(t(Locale::Ko, "some days"), "some days");
        let widths = t(
            Locale::Ko,
            "Lighter quotes no 3m candle: it has 1m, 5m, 15m, 1h, 4h, 1d",
        );
        assert!(
            widths.contains("1분, 5분, 15분, 1시간, 4시간, 1일"),
            "{widths}"
        );
    }

    /// A sweep's failure is a line per refused row with the venue's reason
    /// under it, and each line is translated on its own: the row's template
    /// cannot swallow the reason after it, and a reason the table does not
    /// know stays the venue's own words.
    #[test]
    fn a_sweep_refusal_keeps_the_venue_message_whole() {
        let error = "1 of 2 cancelled.\nBTC buy 1.5 at 63,600.00\nOrder was never placed, already canceled, or filled.\nBTC sell 0.8 at 64,440.00\nUnlock on Settings before sending an order.";
        let korean = t(Locale::Ko, error);
        let lines: Vec<&str> = korean.lines().collect();
        assert_eq!(lines.len(), 5, "{korean}");
        assert_eq!(lines[0], "2건 중 1건 취소.");
        assert!(
            lines[1].starts_with("BTC")
                && lines[1].contains("63,600.00")
                && lines[1].contains("매수"),
            "{}",
            lines[1]
        );
        assert_eq!(
            lines[2],
            "Order was never placed, already canceled, or filled."
        );
        assert!(lines[3].contains("매도"), "{}", lines[3]);
        assert!(!lines[4].contains("Settings"), "{}", lines[4]);
    }

    /// A sample of the sentences Rust composes, one per file, answered in
    /// Korean through the templates — so a template whose English side
    /// drifted from the Rust fails here and not on a reader's screen.
    #[test]
    fn the_sentences_rust_composes_answer_in_korean() {
        use crate::{ChartIndicator, Tif, Venue};
        let english = vec![
            crate::venue::venue_account_gap(Venue::Lighter),
            crate::venue::venue_note(Venue::HyperliquidTestnet),
            crate::venue::venue_twap_note(Venue::Hyperliquid),
            crate::venue::venue_levels_note(Venue::Lighter),
            crate::venue::tif_act(Venue::Hyperliquid, Tif::Ioc),
            crate::venue::order_worked("5"),
            crate::venue::review_label(true),
            crate::venue::margin_mode(false),
            crate::venue::venue_switch_label(Venue::Lighter),
            crate::venue::venue_label(Venue::LighterTestnet),
            crate::venue::venue_orders_note(Venue::Lighter, true, ""),
            crate::hyperliquid::fmt_leverage_mode(40.0, "cross".to_owned()),
            crate::hyperliquid::book_label(64_000.0, true),
            crate::hyperliquid::share_act(0.25, false),
            crate::hyperliquid::interval_label("1h".to_owned()),
            crate::hyperliquid::fmt_age(1_000, 1_120),
            crate::indicators::chart_indicator_action(ChartIndicator::Ema20, false),
            crate::portfolio::range_label("week".to_owned()),
            crate::portfolio::portfolio_empty().note,
            crate::custody::backup_label(&[2, 9, 24], 9),
            crate::custody::session_badge(crate::custody::session_start(), 0),
        ];
        let spoken: Vec<String> = english.into_iter().filter(|s| !s.is_empty()).collect();
        assert!(
            spoken.len() >= 18,
            "{} samples answered nothing",
            21 - spoken.len()
        );
        for sentence in spoken {
            let korean = t(Locale::Ko, &sentence);
            assert_ne!(korean, sentence, "no Korean for {sentence:?}");
            assert!(
                korean
                    .chars()
                    .any(|c| ('\u{AC00}'..='\u{D7A3}').contains(&c)),
                "{korean:?}"
            );
            assert!(!korean.contains('{'), "a hole survived in {korean:?}");
        }
    }

    /// Text the same in both languages is a deliberate decision, one arm each,
    /// and not a missing one: the table answers it rather than falling through.
    #[test]
    fn a_kept_english_token_is_a_table_answer_and_not_a_hole() {
        assert_eq!(ko("TWAP"), Some("TWAP"));
        assert_eq!(t(Locale::Ko, "SETTINGS"), "설정");
    }
}

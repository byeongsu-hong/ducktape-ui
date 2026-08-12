app Candles
  title "Ducktape Candles"
  id "dev.ducktape.ice.candles"
  text-size 14
  window
    size 960 600
    min-size 640 400
    position centered

use "theme.ice"
use "extern/market.ice"

state
  symbol = "DUCK-USD"
  feed:MarketFeed = market_connect("DUCK-USD", 60, 10000)
  last = 0.0
  last_up = true
  hover:CandleHit? = none

on pick_symbol(name)
  symbol = name
  feed = market_connect(name, 60, 10000)
  last = 0.0
  hover = none

on tick(t)
  last = t.last
  last_up = t.up

on candle_hovered(next)
  hover = next

subscribe
  market_events(feed) -> tick _

view
  box #app
    with
      w=fill
      h=fill
      bg=bg
    col w=fill h=fill
      box #header w=fill bg=surface
        row
          with
            w=fill
            p=12.0
            gap=16.0
            align=center
          col gap=2.0
            row gap=8.0 align=center
              text symbol size=16.0 @text-fg
              if last > 0.0
                if last_up
                  text fmt_price(last) size=16.0 @text-up
                if !last_up
                  text fmt_price(last) size=16.0 @text-down
            text "1m - mock feed - 10k backfill - 250ms ticks" size=11.0 @text-muted
          space w=fill
          match hover
            some(h)
              row #readout gap=10.0 align=center
                text "O" size=12.0 @text-muted
                text fmt_price(h.open) size=12.0 @text-fg
                text "H" size=12.0 @text-muted
                text fmt_price(h.high) size=12.0 @text-fg
                text "L" size=12.0 @text-muted
                text fmt_price(h.low) size=12.0 @text-fg
                text "C" size=12.0 @text-muted
                if h.close >= h.open
                  text fmt_price(h.close) size=12.0 @text-up
                if h.close < h.open
                  text fmt_price(h.close) size=12.0 @text-down
                text "Vol" size=12.0 @text-muted
                text fmt_volume(h.volume) size=12.0 @text-fg
            none
              row gap=8.0 align=center
                button "DUCK-USD" #pick-duck -> pick_symbol("DUCK-USD")
                  with
                    p=6.0
                    disabled=(symbol == "DUCK-USD")
                  active bg=surface text=fg r=6.0
                  hovered bg=border text=fg r=6.0
                  disabled bg=primary text=primary_fg r=6.0
                button "TAPE-KRW" #pick-tape -> pick_symbol("TAPE-KRW")
                  with
                    p=6.0
                    disabled=(symbol == "TAPE-KRW")
                  active bg=surface text=fg r=6.0
                  hovered bg=border text=fg r=6.0
                  disabled bg=primary text=primary_fg r=6.0
                text "Scroll to zoom - drag to pan" size=12.0 @text-muted
      box #chart-frame
        with
          w=fill
          h=fill
          p=8.0
        extern chart(feed) #chart -> candle_hovered _

test candles_smoke
  viewport 960 600
  target app = #app
  target frame = #app/chart-frame
  expect app.width ~= 960.0
  expect frame.height > 400.0
  capture ready

test symbol_selection_moves_after_click
  viewport 640 400
  target duck = #app/header/pick-duck
  target tape = #app/header/pick-tape
  expect symbol == "DUCK-USD"
  expect a11y duck role "button"
  expect a11y tape role "button"
  expect a11y duck disabled true
  expect a11y tape disabled false
  expect duck.background == background.color(color.rgb8(108, 192, 111))
  expect tape.background == background.color(color.rgb8(35, 34, 25))
  click tape
  expect symbol == "TAPE-KRW"
  expect a11y duck disabled false
  expect a11y tape disabled true
  expect duck.background == background.color(color.rgb8(35, 34, 25))
  expect tape.background == background.color(color.rgb8(108, 192, 111))
  capture tape_selected

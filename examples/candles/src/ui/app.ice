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
  candles:[Candle] = gen_candles(360)
  hover:CandleHit? = none

on candle_hovered(next)
  hover = next

on tick
  candles = next_tick(candles)

subscribe
  every 500ms -> tick

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
            text "DUCK / USD" size=16.0 @text-fg
            text "1h - synthetic tape" size=11.0 @text-muted
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
              text "Scroll to zoom - drag to pan" size=12.0 @text-muted
      box #chart-frame
        with
          w=fill
          h=fill
          p=8.0
        extern chart(candles) #chart -> candle_hovered _

test candles_smoke
  viewport 960 600
  target app = #app
  target frame = #app/chart-frame
  expect app.width ~= 960.0
  expect frame.height > 400.0
  capture ready

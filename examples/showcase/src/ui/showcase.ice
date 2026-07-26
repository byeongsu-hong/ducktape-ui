app Showcase
  title "Ducktape Design System · Ice"
  id "dev.ducktape.ui.showcase"
  font "assets/fonts/Geist.ttf"
  font "assets/fonts/GeistMono.ttf"
  font "assets/fonts/IBMPlexSansKR-Regular.ttf"
  font "assets/fonts/IBMPlexSansKR-Medium.ttf"
  font "assets/fonts/IBMPlexSansKR-SemiBold.ttf"
  text-size 13
  antialiasing true
  window
    size 1440 900
    min-size 720 560
    position centered

font ui family="Geist" default=true

use "components.ice"
use "../../../../crates/ui/src/ice/default.ice"

extern crate::backend
  sync sticky_nav_top(scroll_y:f64, viewport_width:f64) -> f64

preset test

state
  catalog_y = 0.0

on catalog_scrolled(ax, ay, reversed_x, reversed_y, rx, ry, bx, by, bw, bh, cx, cy, cw, ch)
  catalog_y = ay

on go_color
  task widget scroll-to #catalog 0.0 201.0

on go_typography
  task widget scroll-to #catalog 0.0 1366.0

on go_form
  task widget scroll-to #catalog 0.0 2007.0

on go_components
  task widget scroll-to #catalog 0.0 2652.0

on go_layout
  task widget scroll-to #catalog 0.0 4000.0

on go_icons
  task widget scroll-to #catalog 0.0 4852.0

on go_data
  task widget scroll-to #catalog 0.0 5166.0

on go_overlays
  task widget scroll-to #catalog 0.0 5865.0

on go_voice
  task widget scroll-to #catalog 0.0 6402.0

on go_forge
  task widget scroll-to #catalog 0.0 6679.0

on go_patterns
  task widget scroll-to #catalog 0.0 7138.0

on go_rules
  task widget scroll-to #catalog 0.0 7749.0

view
  box w=fill h=fill bg=linear(2.22, desk_lit@0.0, bg@0.42, chrome@1.0)
    stack w=fill h=fill
      scroll #catalog dir=vertical w=fill h=fill bar-w=10.0 bar-m=3.0 scroller-w=4.0 viewport=catalog_scrolled
        box w=fill px=30.0 pt=44.0 pb=90.0 align-x=center
          col w=fill max-w=1080.0 gap=34.0
            PageHeader mark="D" title="Ducktape Design System" edition="/ Liquid Glass" description_before="Apple HIG의 레이어 규칙을 그대로 따릅니다 — 유리는 콘텐츠 위에 떠 있는 " description_emphasis="기능 레이어" description_after="(타이틀바, 레일, 사이드바, 툴바, 시트)에만. 콘텐츠 레이어는 그대로 불투명 종이. 유리 위에 유리를 겹치지 않습니다."
              row gap=7.0 wrap wrap-gap=7.0
                HeaderTag label="content stays opaque"
                HeaderTag label="Geist + Geist Mono + Plex KR"
                HeaderTag label="glass on nav layer only"
                HeaderTag.Accent label="single accent"
            responsive at=800.0 w=fill
              space w=fill h=121.0
              space w=fill h=84.0
            HairlineDivider
            Section01MaterialColor
            Section02Typography
            Section03ShapeDepthMotion
            Section04Components
            Section05LayoutPatterns
            Section06Iconography
            Section07DataDisplay
            Section08ComposerOverlays
            Section09Voice
            Section10ForgeCode
            Section11Patterns
            Section12Rules
            PageFooter copy="extracted from Ducktape Console.dc.html · tokens are literal hex, accent is the only runtime variable"
        active
          y-rail bg=transparent
          y-scroller bg=scroll_thumb r=6.0
        hovered y-hovered=true
          y-scroller bg=scroll_thumb_hover r=6.0
      responsive size=(viewport_width, viewport_height) w=fill
        box w=fill pt=sticky_nav_top(catalog_y, viewport_width) px=30.0 align-x=center
          box w=fill max-w=1080.0
            SectionNav
              grid gap=6.0 min-cell=148.0 @w-full
                button label="Color" w=fill p=0.0 -> go_color
                  NavLink num="01" label="Color"
                  active bg=transparent r=11.0
                  hovered bg=white/60 r=11.0
                button label="Typography" w=fill p=0.0 -> go_typography
                  NavLink num="02" label="Typography"
                  active bg=transparent r=11.0
                  hovered bg=white/60 r=11.0
                button label="Form" w=fill p=0.0 -> go_form
                  NavLink num="03" label="Form"
                  active bg=transparent r=11.0
                  hovered bg=white/60 r=11.0
                button label="Components" w=fill p=0.0 -> go_components
                  NavLink num="04" label="Components"
                  active bg=transparent r=11.0
                  hovered bg=white/60 r=11.0
                button label="Layout" w=fill p=0.0 -> go_layout
                  NavLink num="05" label="Layout"
                  active bg=transparent r=11.0
                  hovered bg=white/60 r=11.0
                button label="Icons" w=fill p=0.0 -> go_icons
                  NavLink num="06" label="Icons"
                  active bg=transparent r=11.0
                  hovered bg=white/60 r=11.0
                button label="Data" w=fill p=0.0 -> go_data
                  NavLink num="07" label="Data"
                  active bg=transparent r=11.0
                  hovered bg=white/60 r=11.0
                button label="Overlays" w=fill p=0.0 -> go_overlays
                  NavLink num="08" label="Overlays"
                  active bg=transparent r=11.0
                  hovered bg=white/60 r=11.0
                button label="Voice" w=fill p=0.0 -> go_voice
                  NavLink num="09" label="Voice"
                  active bg=transparent r=11.0
                  hovered bg=white/60 r=11.0
                button label="Forge" w=fill p=0.0 -> go_forge
                  NavLink num="10" label="Forge"
                  active bg=transparent r=11.0
                  hovered bg=white/60 r=11.0
                button label="Patterns" w=fill p=0.0 -> go_patterns
                  NavLink num="11" label="Patterns"
                  active bg=transparent r=11.0
                  hovered bg=white/60 r=11.0
                button label="Rules" w=fill p=0.0 -> go_rules
                  NavLink num="12" label="Rules"
                  active bg=transparent r=11.0
                  hovered bg=white/60 r=11.0

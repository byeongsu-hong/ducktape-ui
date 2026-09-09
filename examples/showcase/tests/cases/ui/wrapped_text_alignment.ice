app WrappedTextAlignment
  text-size 14
  font "../../../../../assets/fonts/Geist-Regular.ttf"

use "themes/monochrome.ice"

font geist family="Geist" default=true

view
  col #page w=fill p=24.0 gap=16.0
    row #plain gap=12.0
      text "Alpha beta gamma delta epsilon zeta end" #plain-left w=120.0 h=160.0 align-x=left align-y=top wrap=word line-h-px=20.0 shape=advanced
      text "Alpha beta gamma delta epsilon zeta end" #plain-center w=120.0 h=160.0 align-x=center align-y=center wrap=word line-h-px=20.0 shape=advanced
      text "Alpha beta gamma delta epsilon zeta end" #plain-right w=120.0 h=160.0 align-x=right align-y=bottom wrap=word line-h-px=20.0 shape=advanced
      text "Alpha beta gamma delta epsilon zeta end" #plain-justified w=120.0 h=160.0 align-x=justified align-y=center wrap=word line-h-px=20.0 shape=advanced
    row #rich gap=12.0
      rich-text #rich-left w=120.0 h=160.0 align-x=left align-y=top wrap=word line-h-px=20.0
        span "Alpha beta gamma delta epsilon zeta end"
      rich-text #rich-center w=120.0 h=160.0 align-x=center align-y=center wrap=word line-h-px=20.0
        span "Alpha beta gamma delta epsilon zeta end"
      rich-text #rich-right w=120.0 h=160.0 align-x=right align-y=bottom wrap=word line-h-px=20.0
        span "Alpha beta gamma delta epsilon zeta end"
      rich-text #rich-justified w=120.0 h=160.0 align-x=justified align-y=center wrap=word line-h-px=20.0
        span "Alpha beta gamma delta epsilon zeta end"
    row #natural gap=12.0
      text "short\nx" #plain-natural h=40.0 line-h-px=20.0 shape=advanced
      text "short\nx" #plain-natural-justified h=40.0 line-h-px=20.0 shape=advanced align-x=justified
      rich-text #rich-natural h=40.0 line-h-px=20.0
        span "short\nx"
      rich-text #rich-natural-justified h=40.0 line-h-px=20.0 align-x=justified
        span "short\nx"

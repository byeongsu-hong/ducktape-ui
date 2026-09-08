app TextFixture
  title "Text fixture"
  id "dev.ice.text-fixture"

theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #ffffff
  fg #000000
  primary #00aa44
  danger #ff0000

font ui family="Geist" default=true
font display family="Geist" weight=semibold
font code_medium family="Geist Mono" weight=medium

state
  clicked = false
  links = ["Open", "Read"]
  last_link = "none"
  link_count = 0
on visited(value)
  last_link = value
  link_count = link_count + 1
on apply
  clicked = true

component KitLabel(label:str)
  text label font=display wrap=none line-h=1.5 h=44.0 align-y=center #label

component RichLine(words:[str])
  emits
    opened(str)
  box #paragraph w=fill
    rich-text #rich -> emit(opened, _)
      with
        w=fill
        size=16.0
        line-h=1.5
        wrap=word-or-glyph
        color=fg
      for word in words
        span word link=word underline font=display color=fg
        span " Mention " bg=primary px=4.0 r=4.0 font=code_medium color=bg
        span "a paragraph that keeps one native line layout. " font=display

view
  box #bounded
    with
      w=fill
      max-w=620.0
      clip=true
      @px-4
      @py-2
    col #settings
      with
        w=fill
        max-w=560.0
        clip=true
        gap=8.0
      text "Settings content" #settings-width w=fill
      KitLabel label="Node overview" #heading
      text "A paragraph that wraps between words and also breaks long_unbroken_identifiers." #wrapped
        with
          w=180.0
          font=code_medium
          wrap=word-or-glyph
          shape=advanced
          line-h=1.5
      text "TRACKING" #tracked font=display tracking=2.0
      box #clipped
        with
          w=80.0
          h=20.0
          clip=true
          bg=primary
        text "Clipped content extending past its parent" wrap=none size=28.0
      tooltip
        with
          position=bottom
          gap=13.5
          p=0.0
          delay=90
          style=transparent
        button "Apply" #apply @px-4 py-2 -> apply
        box
          with
            w=100.0
            h=30.0
            bg=danger
          rich-text
            span "Apply"
            span " changes"
      row #actions wrap
        with
          w=fill
          gap=8.0
          wrap-gap=6.0
          wrap-align=end
        button "First" -> apply
          with
            w=100.0
            h=30.0
            p=0.0
        button "Second" -> apply
          with
            w=100.0
            h=30.0
            p=0.0
        button "Third" -> apply
          with
            w=100.0
            h=30.0
            p=0.0
      if clicked
        text "Applied" #status
      lazy links as words
        RichLine words=words #message
          events
            opened -> visited _
      text last_link #link-status
      text link_count #link-count
      col #clipped-column
        with
          w=80.0
          h=20.0
          clip=true
        text "Column content extending past its parent" wrap=none size=28.0
      row #clipped-row
        with
          w=80.0
          h=20.0
          clip=true
        text "Row content extending past its parent" wrap=none size=28.0

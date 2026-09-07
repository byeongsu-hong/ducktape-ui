app FlexFixture
  title "Flex fixture"
  id "dev.ice.flex-fixture"

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

state
  chosen:i64 = 0
  values = [1, 2, 3, 4]
  draft = ""
  sent = ""
on choose(value)
  chosen = value
on send
  sent = draft

component Chip(number:i64)
  text number

view
  col w=fill gap=8.0
    text chosen #chosen
    lazy values as cached
      flex #reactions
        with
          w=fill
          wrap=wrap
          gap-x=7.0
          gap-y=5.0
          items=start
        for value in cached
          if value > 0
            button #reaction -> choose value
              with
                label="Reaction"
                w=60.0
                h=30.0
                p=0.0
              Chip number=value
    flex #composer
      with
        w=fill
        gap=8.0
        items=end
      box
        with
          w=fill
          grow=1.0
          shrink=1.0
          basis=content
        input "Write" #draft <-> draft
      button "Send" #send w=60.0 h=28.0 -> send
    text sent #sent
    flex #advanced
      with
        dir=row-reverse
        wrap=wrap-reverse
        w=fill
        h=100.0
        max-w=900.0
        max-h=500.0
        gap=8.0
        gap-y=12.0
        gap-x=16.0
        justify=space-evenly
        items=baseline
        content=space-between
        p=4.0
        clip=true
      box
        with
          order=2
          grow=1.0
          shrink=0.5
          basis=percent(40.0)
          self=flex-end
          m=auto
        text "First"
      box
        with
          flex=2.0,1.0,120.0
          mx=percent(5.0)
          mt=-2.0
        text "Second"
    flex #utility
      with
        w=100.0
        h=50.0
        @w-full
        @h-full
        @max-w-sm
        @bg-primary
      text "Utility sized surface"

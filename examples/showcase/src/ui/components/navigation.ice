enum DemoTab
  preview
  code

component CarouselDemo()
  state
    slide = 1
  on previous
    slide = (slide + 1) % 3 + 1
  on next
    slide = slide % 3 + 1
  col w=fill gap=8.0
    box
      with
        w=fill
        h=88.0
        p=16.0
        align-x=center
        align-y=center
        bg=accent
        border=border
        border-w=1.0
        r=9.0
      col align=center gap=4.0
        text "Slide" size=11.0 @text-muted
        text slide
          with
            size=24.0
            @font-bold
            @text-primary
    row
      with
        w=fill
        gap=8.0
        align=center
      button "Previous" p=7.0 @secondary_action -> previous
      space w=fill h=1.0
      text slide size=12.0 @text-muted
      text "/ 3" size=12.0 @text-muted
      space w=fill h=1.0
      button "Next" p=7.0 @secondary_action -> next

component TabsDemo()
  lifetime mounted
  state
    selected:DemoTab = DemoTab.preview
  on preview
    selected = DemoTab.preview
  on code
    selected = DemoTab.code
  col w=fill gap=12.0
    match selected
      DemoTab.preview
        row
          with
            gap=4.0
            p=4.0
            @bg-accent
            @rounded-lg
          button "Preview" p=8.0 @secondary_action -> preview
          button "Code" #show-code p=8.0 @ghost_action -> code
        box
          with
            w=fill
            h=92.0
            p=16.0
            bg=accent/40
            border=border
            border-w=1.0
            r=9.0
          text "The default component is ready to compose." size=13.0 @text-fg
      DemoTab.code
        row
          with
            gap=4.0
            p=4.0
            @bg-accent
            @rounded-lg
          button "Preview" #show-preview p=8.0 @ghost_action -> preview
          button "Code" p=8.0 @secondary_action -> code
        box
          with
            w=fill
            h=92.0
            p=16.0
            bg=fg
            r=9.0
          text "button \"Save\" @primary_action -> save" size=13.0 @text-white

component PaginationDemo(page:i64, max_page:i64=5)
  emits
    previous
    next
  row gap=6.0 align=center
    button "Previous" #previous -> emit(previous)
      with
        p=8.0
        disabled=(page <= 1)
        @secondary_action
    box
      with
        w=36.0
        h=36.0
        align-x=center
        align-y=center
        bg=primary
        r=8.0
      text page
        with
          size=13.0
          @font-bold
          @text-white
    text "of" size=12.0 @text-muted
    text max_page size=12.0 @text-muted
    button "Next" #next -> emit(next)
      with
        p=8.0
        disabled=(page >= max_page)
        @secondary_action

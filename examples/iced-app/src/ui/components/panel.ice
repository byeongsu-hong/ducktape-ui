component Panel(title:str)
  col
    with
      gap=12.0
      p=16.0
      @w-full
      @bg-surface
      @border
      @border-border
      @rounded-lg
    text title
      with
        size=18.0
        @font-bold
        @text-fg
    slot

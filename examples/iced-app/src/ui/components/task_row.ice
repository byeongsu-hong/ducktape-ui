component TaskRow(task:Task, loading:bool)
  emits
    toggle(i64, bool)
  row
    with
      p=16.0
      align=center
      @w-full
      @bg-surface
      @border
      @border-border
      @rounded-lg
    checkbox task.title -> emit(toggle, task.id, _)
      with
        checked=task.done
        disabled=loading
        style=success
      active checked bg=linear(1.57, primary@0.0, surface@1.0) icon=fg text=fg border=primary border-w=1.0 r=4.0 r-tl=3.0 r-tr=4.0 r-br=5.0 r-bl=6.0
      active unchecked bg=surface icon=primary text=fg border=border
      hovered checked bg=primary icon=fg text=fg border=fg
      hovered unchecked bg=bg icon=primary text=fg border=primary
      disabled checked bg=surface icon=muted text=muted border=border
      disabled unchecked bg=bg icon=muted text=muted border=border

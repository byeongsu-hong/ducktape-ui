component Dialog(width:f64=480.0)
  box w=width h=shrink max-w=720.0 p=24.0 border-w=1.0 r=10.0 @bg-surface border-border
    col gap=16.0 @w-full
      slot Header
      slot Body
      if provided(Actions)
        slot Actions?

component Dialog.Header()
  row align=center @w-full
    slot

component Dialog.Body()
  box w=fill
    slot

component Dialog.Actions()
  row gap=8.0 @w-full
    slot

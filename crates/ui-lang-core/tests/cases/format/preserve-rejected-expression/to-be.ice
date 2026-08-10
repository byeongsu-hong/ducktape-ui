app Demo
on pressed
view
  col
    button "Save" disabled=a || b w=200.0 h=40.0 -> pressed
    button "Cancel" -> pressed
      with
        disabled=x || y
        w=120.0
        h=32.0

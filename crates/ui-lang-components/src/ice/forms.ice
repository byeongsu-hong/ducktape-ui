// Form owns scrolling; its content keeps a readable width on large windows.
component Form(max_width:f64=640.0, padding:f64=24.0)
  scroll #root w=fill h=fill
    box
      with
        w=fill
        align-x=center
        bg=bg
      box #content
        with
          w=fill
          max-w=max_width
          p=padding
        slot

component FormSection(title:str, description:str="", padding:f64=20.0, radius:f64=11.0)
  box #root
    with
      w=fill
      p=padding
      r=radius
      bg=surface
      border=border
      border-w=1.0
    col w=fill gap=20.0
      col w=fill gap=4.0
        text title #title
          with
            heading=2
            w=fill
            wrap=word-or-glyph
            @section_title
        if !empty(description)
          text description #description
            with
              w=fill
              wrap=word-or-glyph
              @caption
      slot

// The slot-based Field remains available for custom controls. This convenience
// component owns a native input, so changing its geometry preserves behavior.
component TextField(label:str, bind value:str, description:str="", error:str="", placeholder:str="", disabled:bool=false, secure:bool=false, padding:f64=11.0, radius:f64=10.0)
  Field #field
    with
      label=label
      description=description
      error=error
    input "" #input <-> value
      with
        hint=placeholder
        label=label
        description=description
        disabled=disabled
        secure=secure
        w=fill
        p=padding
        @control
      active r=radius
      focused border=ring border-w=2.0 r=radius
      focused-hovered border=ring border-w=2.0 r=radius

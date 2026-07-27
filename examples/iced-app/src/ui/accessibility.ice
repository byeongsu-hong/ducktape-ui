app Accessibility

use "themes/slate.ice"

state
  name = ""
  secret = ""
  accepted = false

on toggle(value)
  accepted = value

on submit

view
  col gap=12.0 p=16.0
    text "Accessible form"
    input "Name" #name label="Full name" description="Name used on your profile" hint="Ada" <-> name
    input "Password" #password <-> secret
      with
        label="Account password"
        description="Password text is never exported"
        secure=true
    checkbox "Accept terms" #terms -> toggle _
      with
        label="Terms consent"
        description="Required before submission"
        checked=accepted
    button "Submit" #submit -> submit
      with
        description="Save the accessible form"
        disabled=empty(trim(name))
    button #help label="Open help" description="Show keyboard and screen-reader help" -> submit
      text "?"
    image "assets/demo.ppm"
      with
        label="Ice accessibility example"
        description="A decorative sample promoted into the accessibility tree"

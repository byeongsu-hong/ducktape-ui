app HandlerComponentRetention

use "themes/monochrome.ice"

component Ephemeral()
  on press
  button "Press" -> press

view
  Ephemeral #action

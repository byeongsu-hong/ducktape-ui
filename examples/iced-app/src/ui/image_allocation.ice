app ImageAllocation

use "themes/slate.ice"

state
  handle:image = rgba(1, 1, bytes(ff 00 ff ff))
  allocation:image-allocation? = none
  retained:image-memory? = none
  recovered:image-allocation? = none
  failure:image-error? = none
  width = 0
  height = 0
  error_kind = ""
  error_message = ""

on allocate
  task image allocate handle -> ready _ | failed _

on ready(value)
  handle = value.handle
  width = value.size.width
  height = value.size.height
  retained = some(image.downgrade(value))
  recovered = image.upgrade(image.downgrade(value))
  allocation = some(value)

on failed(error)
  error_kind = error.kind
  error_message = error.message
  failure = some(error)

on allocate_flow
  flow
    from task image allocate handle
    map value -> value.size.width
    map-err error -> error.message
    done -> width_ready _
    error -> flow_failed _

on width_ready(value)
  width = value

on flow_failed(message)
  error_message = message

view
  col gap=8.0 p=16.0
    image handle w=64.0 h=64.0
    button "Allocate" -> allocate
    button "Allocate flow" -> allocate_flow
    text width
    text height
    text error_kind
    text error_message

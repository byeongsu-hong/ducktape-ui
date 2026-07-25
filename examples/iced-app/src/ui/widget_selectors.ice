app WidgetSelectors

use "extern/widget_selectors.ice"

use "themes/slate.ice"

state
  value = ""
  found:widget-target? = none
  found_all:[widget-target] = []
  kinds:[str] = []
  found_kind:str? = none
  found_x:f64? = none

on find_id
  task widget find id #root/field -> found_one _

on find_text
  task widget find text "Search" -> found_one _

on find_point
  task widget find point 12.0 24.0 -> found_one _

on find_focused
  task widget find focused -> found_one _

on find_all_text
  task widget find-all text "Search" -> found_many _

on find_custom
  task widget find-all by_kind("text") -> found_kinds _

on found_one(value)
  found_kind = value.kind
  found_x = value.x
  found = value

on found_many(value)
  found_all = value

on found_kinds(value)
  kinds = value

view
  col #root
    input "Search" #field <-> value
    text "Search"

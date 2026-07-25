app NativeWindowScreenshot

use "extern/window_screenshot.ice"

use "themes/slate.ice"

state
  sample:window-screenshot = screenshot_sample()
  returned:window-screenshot = screenshot_sample()
  rebuilt:window-screenshot = screenshot_sample()
  cropped:window-screenshot? = none
  rgba:bytes = bytes(00)
  size:size-u32 = screenshot_size()
  scale_factor = 0.0
  debug_text = ""
  borrowed_bytes:bytes = bytes(00)
  owned_bytes:bytes = bytes(00)
  zero_error:str? = none
  outside_error:str? = none
  valid_error:str? = none
  zero_message:str? = none
  outside_message:str? = none

on inspect
  returned = screenshot_round_trip(sample)
  rebuilt = screenshot.new(sample.rgba, sample.size, sample.scale_factor)
  cropped = screenshot.crop(sample, screenshot_crop_region())
  rgba = returned.rgba
  size = returned.size
  scale_factor = returned.scale_factor
  debug_text = returned.debug
  borrowed_bytes = screenshot.as_bytes(returned)
  owned_bytes = screenshot.into_bytes(returned)
  zero_error = screenshot.crop_error(sample, screenshot_zero_region())
  outside_error = screenshot.crop_error(sample, screenshot_outside_region())
  valid_error = screenshot.crop_error(sample, screenshot_crop_region())
  zero_message = screenshot.crop_error_message(sample, screenshot_zero_region())
  outside_message = screenshot.crop_error_message(sample, screenshot_outside_region())

on capture_native
  task window screenshot -> native_captured _

on native_captured(value)
  returned = value

view
  col gap=8.0 p=16.0
    button "Inspect" -> inspect
    button "Capture native" -> capture_native
    text debug_text
    text scale_factor

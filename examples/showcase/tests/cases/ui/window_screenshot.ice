app NativeWindowScreenshot

use "extern/window_screenshot.ice"

use "themes/slate.ice"

state
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
  let sample = screenshot_sample()
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

test inspect_window_screenshot
  dispatch inspect
  expect rebuilt.size == screenshot_size()
  expect rebuilt.scale_factor == 2.0
  expect rgba == bytes(00 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e 0f 10 11 12 13 14 15 16 17)
  expect size == screenshot_size()
  expect borrowed_bytes == rgba
  expect owned_bytes == rgba
  expect zero_error == some("zero")
  expect outside_error == some("out-of-bounds")
  expect valid_error == none
  expect zero_message == some("The cropped region is not visible.")
  expect outside_message == some("The cropped region is out of bounds.")

view
  col gap=8.0 p=16.0
    button "Inspect" -> inspect
    button "Capture native" -> capture_native
    text debug_text
    text scale_factor
    match cropped
      some(region)
        text region.debug
      none
        text "No cropped region"

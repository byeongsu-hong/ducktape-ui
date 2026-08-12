app NativeContentFit

use "extern/content_fit.ice"

use "themes/slate.ice"

state
  pixel:image = rgba(1, 1, bytes(ff 00 ff ff))
  default_fit:content-fit = fit.default()
  contain_fit:content-fit = fit.default()
  cover_fit:content-fit = fit.default()
  fill_fit:content-fit = fit.default()
  none_fit:content-fit = fit.default()
  scale_down_fit:content-fit = fit.default()
  round_trip:content-fit = fit.default()
  applied_size:size = size.zero()
  kind = ""
  display = ""
  equal = false

on inspect
  default_fit = fit.default()
  contain_fit = fit.contain()
  cover_fit = fit.cover()
  fill_fit = fit.fill()
  none_fit = fit.none()
  scale_down_fit = fit.scale_down()
  round_trip = content_fit_round_trip(cover_fit)
  applied_size = fit.apply(contain_fit, size(100.0, 50.0), size(80.0, 80.0))
  kind = scale_down_fit.kind
  display = scale_down_fit.display
  equal = default_fit == contain_fit

test inspect_content_fit
  dispatch inspect
  expect none_fit == fit.none()
  expect equal

view
  col gap=8.0 p=16.0
    button "Inspect" -> inspect
    image pixel
      with
        w=48.0
        h=48.0
        fit=round_trip
    viewer pixel
      with
        w=48.0
        h=48.0
        fit=scale_down_fit
    svg "<svg/>" memory
      with
        w=48.0
        h=48.0
        fit=fill_fit
    text kind
    text display
    text applied_size.width

test packaged_consumer_contract
  viewport 320 200
  target root = #root
  target message = #root/message
  expect root.width > 0.0
  expect message.visible
  expect text "Hello from packaged crates" within message
  capture packaged_consumer

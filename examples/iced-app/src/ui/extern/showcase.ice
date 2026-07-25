extern crate::backend
  Task(id:i64, title:str, done:bool)
  AppError(message:str)
  EditorCommand(save:bool)
  SliderNumber()
  list_tasks() -> [Task] ! AppError
  create_task(title:str) -> [Task] ! AppError
  set_task_done(id:i64, done:bool) -> [Task] ! AppError
  sync slider_number(value:f64) -> SliderNumber
  component native_help(active:bool) -> bool
  component borrowed_help(label:&str, active:&bool) -> bool
  markdown-viewer docs_viewer(prefix:str) -> str
  editor-binding editor_keys(readonly:bool) -> EditorCommand
  editor-highlighter editor_highlight(token:str)
  editor-style editor_surface(readonly:bool)
  text-style summary_text(busy:bool)
  slider-style volume_slider(busy:bool)
  progress-style loading_progress(active:bool)
  button-style action_button(busy:bool)
  checkbox-style task_checkbox(busy:bool)
  toggler-style notification_toggler(busy:bool)
  radio-style view_radio(busy:bool)
  box-style summary_container(busy:bool)
  svg-style status_svg(active:bool)
  input-style form_input(disabled:bool)
  scroll-style task_scroll(active:bool)
  pick-list-style view_picker(active:bool)
  menu-style view_menu(active:bool)
  panes-style workspace_panes(active:bool)
  task copy_text(text:str) -> unit ! AppError
  subscription app_events() -> bool

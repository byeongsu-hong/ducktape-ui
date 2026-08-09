daemon Demo
    tray
        icon-rgba "assets/alarm.rgba" 2 2 when count > 3
        icon-rgba "assets/tray.rgba" 2 2
        label describe(count)
        menu
            describe(count)
            separator
            "Quit" -> quit
extern crate::backend
    pure describe(value:i64) -> str
theme contract AppTheme
    bg
    fg
    primary
    danger
palette app for AppTheme
    bg #000000
    fg #ffffff
    primary #333333
    danger #ff0000
state
    count = 1
on quit
    exit
view
    text count
test chooses_a_row
    tray choose "Quit"
    expect tray command "Quit"
    expect no tray command "1"

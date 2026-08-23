app Extras
extern crate::backend
    Message(seq:i64, body:str)
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
    messages:[Message] = []
    unit = "pt"
view
    col
        for message in messages
            lazy message, unit as cached
                text cached.body

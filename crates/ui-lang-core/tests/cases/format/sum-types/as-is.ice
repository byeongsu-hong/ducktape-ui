app Demo
enum RequestState
    idle
    ready([str])
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
    request:RequestState = RequestState.ready(["one"])
view
    col
        match request
            RequestState.idle
                text "idle"
            RequestState.ready(items)
                text len(items)

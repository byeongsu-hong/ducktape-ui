app Music
  title "Music"
  theme app_theme
  bg app_background
  fg app_text
  id "dev.ducktape.ice.music"
  font "../../../showcase/assets/fonts/Geist-Regular.ttf"
  font "../../../showcase/assets/fonts/Geist-Bold.ttf"
  text-size 14
  antialiasing true
  window
    size 1180 760
    min-size 980 640
    position centered
    decorations false
    transparent true
    platform macos
      title-hidden true
      titlebar-transparent true
      fullsize-content-view true

use "extern/mock_api.ice"
use "theme.ice"
use "state.ice"
use "components/common.ice"
use "components/icons.ice"
use "components/sidebar.ice"
use "components/library.ice"
use "components/player.ice"
use "handlers/app.ice"
use "tests/app.ice"
use "tests/components.ice"
use "../../../../crates/ui/src/ice/recipes.ice"
use "../../../../crates/ui/src/ice/components.ice"

font geist family="Geist" default=true

view
  overlay when=lyrics_open dismiss=lyrics backdrop=transparent p=12.0 align-x=end align-y=center
    content
      overlay when=queue_open dismiss=queue backdrop=black/18 p=12.0 align-x=end align-y=center
        content
          box #app w=fill h=fill p=10.0 clip=true bg=linear(1.57, frame_start@0.0, frame_end@1.0) border=white/78 border-w=1.0 r=26.0
            flex #shell w=fill h=fill dir=row gap=10.0
              Sidebar query<->query section=section signed_in=signed_in profile_name=profile_name loading=loading current_title=current_title current_artist=current_artist current_cover=current_cover #sidebar
                events
                  close_window -> close_window
                  minimize_window -> minimize_window
                  toggle_maximize_window -> toggle_maximize_window
                  drag_window -> drag_window
                  search -> search
                  navigate -> navigate _
                  restart_current -> restart_current
                  sign_in -> sign_in
                  sign_out -> sign_out
              box #content flex=1.0,1.0,0.0 h=fill
                col w=fill h=fill
                  LibraryContent section=section query=query loading=loading error=error top_picks=top_picks recently_played=recently_played search_results=search_results current_title=current_title current_artist=current_artist current_cover=current_cover #library
                    events
                      restart_current -> restart_current
                      queue -> queue
                      play -> play _ _ _
                  box #dock w=fill px=16.0 pb=16.0
                    PlayerBar title=current_title artist=current_artist cover=current_cover active=playing playhead=position loudness=volume lyrics_active=lyrics_open queue_active=queue_open #player
                      events
                        shuffle -> shuffle
                        previous -> previous
                        toggle_playback -> toggle_playback
                        next -> next
                        seek -> seek _
                        toggle_mute -> toggle_mute
                        volume_changed -> volume_changed _
                        lyrics -> lyrics
                        queue -> queue
        layer
          QueuePanel albums=recently_played current_title=current_title current_artist=current_artist current_cover=current_cover #queue-panel
            events
              queue -> queue
              play -> play _ _ _
    layer
      LyricsPanel title=current_title artist=current_artist lines=lyrics_for(current_title, position) #lyrics-panel
        events
          lyrics -> lyrics
          seek -> seek _

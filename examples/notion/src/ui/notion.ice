app Notion
  title "Notion"
  id "dev.ducktape.ice.notion"
  font "../../../showcase/assets/fonts/Geist.ttf"
  text-size 14
  antialiasing true
  window
    size 1280 800
    min-size 860 600
    position centered

font geist family="Geist" default=true

extern crate::helpers
  sync page_matches(query:str, title:str) -> bool

extern crate::editor
  BlockEditorState()
  BlockEditorEvent()
  sync block_editor_state(template:str) -> BlockEditorState
  sync block_editor_apply(state:BlockEditorState, event:BlockEditorEvent) -> BlockEditorState
  sync block_editor_pending_focus(state:BlockEditorState) -> i64
  sync block_editor_clear_focus(state:BlockEditorState) -> BlockEditorState
  task block_editor_focus(block:i64) -> bool
  component block_editor(state:&BlockEditorState) -> BlockEditorEvent

theme
  bg         #ffffff
  surface    #ffffff
  sidebar    #f7f7f5
  fg         #37352f
  muted      #787774
  faint      #9b9a97
  hover      #efefed
  selected   #e8e7e4
  border     #e9e9e7
  primary    #2f80ed
  danger     #eb5757
  blue_soft  #eaf3fb

state
  selected_page = "home"
  sidebar_open = true
  search_open = false
  search_query = ""
  share_open = false
  favorite = true
  invite_email = ""
  invite_sent = false
  pending_focus = 0
  home_title = "Building a home for your work"
  roadmap_title = "Product roadmap"
  launch_title = "Launch plan"
  meeting_title = "Weekly meeting"
  untitled_title = "Untitled"
  home_document:BlockEditorState = block_editor_state("home")
  roadmap_document:BlockEditorState = block_editor_state("roadmap")
  launch_document:BlockEditorState = block_editor_state("launch")
  meeting_document:BlockEditorState = block_editor_state("meeting")
  untitled_document:BlockEditorState = block_editor_state("untitled")

on navigate(page)
  selected_page = page
  search_open = false

on toggle_sidebar
  sidebar_open = !sidebar_open

on open_search
  search_open = true
  share_open = false

on close_search
  search_open = false

on toggle_favorite
  favorite = !favorite

on open_share
  share_open = true
  search_open = false
  invite_sent = false

on close_share
  share_open = false

on send_invite
  return if empty(trim(invite_email))
  invite_sent = true

on home_editor_changed(event)
  home_document = block_editor_apply(home_document, event)
  pending_focus = block_editor_pending_focus(home_document)
  home_document = block_editor_clear_focus(home_document)
  return if pending_focus <= 0
  task block_editor_focus(pending_focus) -> editor_focused _

on roadmap_editor_changed(event)
  roadmap_document = block_editor_apply(roadmap_document, event)
  pending_focus = block_editor_pending_focus(roadmap_document)
  roadmap_document = block_editor_clear_focus(roadmap_document)
  return if pending_focus <= 0
  task block_editor_focus(pending_focus) -> editor_focused _

on launch_editor_changed(event)
  launch_document = block_editor_apply(launch_document, event)
  pending_focus = block_editor_pending_focus(launch_document)
  launch_document = block_editor_clear_focus(launch_document)
  return if pending_focus <= 0
  task block_editor_focus(pending_focus) -> editor_focused _

on meeting_editor_changed(event)
  meeting_document = block_editor_apply(meeting_document, event)
  pending_focus = block_editor_pending_focus(meeting_document)
  meeting_document = block_editor_clear_focus(meeting_document)
  return if pending_focus <= 0
  task block_editor_focus(pending_focus) -> editor_focused _

on untitled_editor_changed(event)
  untitled_document = block_editor_apply(untitled_document, event)
  pending_focus = block_editor_pending_focus(untitled_document)
  untitled_document = block_editor_clear_focus(untitled_document)
  return if pending_focus <= 0
  task block_editor_focus(pending_focus) -> editor_focused _

on editor_focused(_focused)
  pending_focus = 0

component SidebarAction(icon:str, label:str)
  button label=label w=fill p=7.0 -> noop
    row w=fill gap=10.0 align=center
      text icon w=18.0 size=16.0 align-x=center @text-muted
      text label w=fill size=13.0 @text-muted
    active bg=transparent text=muted r=5.0
    hovered bg=hover text=fg
    pressed bg=selected text=fg

component PageItem(icon:str, title:str, page:str, selected:bool)
  col w=fill
    if selected
      button label=title w=fill p=6.0 -> navigate(trim(page))
        row w=fill gap=8.0 align=center
          text icon w=18.0 size=15.0 align-x=center @text-fg
          text title w=fill size=13.0 wrap=none @text-fg
        active bg=selected text=fg r=5.0
        hovered bg=selected
        pressed bg=hover
    if !selected
      button label=title w=fill p=6.0 -> navigate(trim(page))
        row w=fill gap=8.0 align=center
          text icon w=18.0 size=15.0 align-x=center @text-muted
          text title w=fill size=13.0 wrap=none @text-muted
        active bg=transparent text=muted r=5.0
        hovered bg=hover text=fg
        pressed bg=selected text=fg

component Sidebar(selected_page:str, home_title:str, roadmap_title:str, launch_title:str, meeting_title:str, untitled_title:str)
  box w=244.0 h=fill bg=sidebar border=border border-w=1.0
    col w=fill h=fill p=8.0 gap=2.0
      row w=fill h=38.0 p=5.0 gap=8.0 align=center
        box w=24.0 h=24.0 align-x=center align-y=center bg=fg r=5.0
          text "E" size=12.0 @text-white font-bold
        text "Eddy's Notion" w=fill size=14.0 @text-fg font-bold
        button label="Collapse sidebar" p=4.0 style=text -> toggle_sidebar
          text "«" size=16.0 @text-muted
      button label="Search" w=fill p=7.0 -> open_search
        row w=fill gap=10.0 align=center
          text "⌕" w=18.0 size=17.0 align-x=center @text-muted
          text "Search" w=fill size=13.0 @text-muted
          text "⌘ K" size=10.0 @text-faint
        active bg=transparent text=muted r=5.0
        hovered bg=hover text=fg
        pressed bg=selected text=fg
      SidebarAction icon="◫" label="Home"
      SidebarAction icon="◷" label="Updates"
      SidebarAction icon="▱" label="Inbox"
      text "Favorites" size=11.0 @text-faint
      PageItem icon="◆" title=home_title page="home" selected=(selected_page == "home")
      text "Private" size=11.0 @text-faint
      PageItem icon="▦" title=roadmap_title page="roadmap" selected=(selected_page == "roadmap")
      PageItem icon="✓" title=launch_title page="launch" selected=(selected_page == "launch")
      PageItem icon="▤" title=meeting_title page="meeting" selected=(selected_page == "meeting")
      PageItem icon="□" title=untitled_title page="untitled" selected=(selected_page == "untitled")
      button label="New page" w=fill p=7.0 -> navigate("untitled")
        row w=fill gap=10.0 align=center
          text "+" w=18.0 size=17.0 align-x=center @text-muted
          text "New page" size=13.0 @text-muted
        active bg=transparent text=muted r=5.0
        hovered bg=hover text=fg
        pressed bg=selected text=fg
      space w=fill h=fill
      row w=fill p=7.0 gap=10.0 align=center
        text "?" w=18.0 size=13.0 align-x=center @text-muted
        text "Help & support" w=fill size=12.0 @text-muted

component MiniSidebar()
  box w=44.0 h=fill bg=sidebar border=border border-w=1.0 align-x=center
    col w=fill h=fill p=7.0 align=center
      button label="Expand sidebar" p=6.0 style=text -> toggle_sidebar
        text "»" size=17.0 @text-muted
      button label="Search" p=6.0 style=text -> open_search
        text "⌕" size=17.0 @text-muted
      space w=fill h=fill
      box w=26.0 h=26.0 align-x=center align-y=center bg=fg r=5.0
        text "E" size=12.0 @text-white font-bold

component Topbar(favorite:bool)
  row w=fill h=46.0 px=12.0 gap=5.0 align=center
    text "Private" size=12.0 @text-muted
    text "/" size=12.0 @text-faint
    text "Workspace" w=fill size=12.0 @text-fg
    if favorite
      button label="Remove from favorites" p=6.0 style=text -> toggle_favorite
        text "★" size=16.0 @text-fg
    if !favorite
      button label="Add to favorites" p=6.0 style=text -> toggle_favorite
        text "☆" size=16.0 @text-muted
    button label="Share" p=6.0 -> open_share
      text "Share" size=12.0 @text-primary
      active bg=transparent text=primary r=5.0
      hovered bg=blue_soft
      pressed bg=selected
    button label="More actions" p=6.0 style=text -> noop
      text "•••" size=12.0 @text-muted

component Document(title:str, state:BlockEditorState, icon:str) -> BlockEditorEvent
  box w=fill h=fill px=28.0 align-x=center
    col w=fill h=fill max-w=920.0 pt=26.0
      text icon size=42.0 @text-fg
      box w=fill px=30.0
        input "" label="Page title" <-> title hint="Untitled" w=fill p=0.0 text-size=36.0 font=geist
          active bg=transparent border=transparent value=fg placeholder=faint selection=primary border-w=0.0 r=0.0
          hovered bg=transparent border=transparent value=fg placeholder=faint border-w=0.0
          focused bg=transparent border=transparent value=fg placeholder=faint selection=primary border-w=0.0
      extern block_editor(state) -> emit _

component SearchDialog(search_query:str, selected_page:str, home_title:str, roadmap_title:str, launch_title:str, meeting_title:str)
  box w=520.0 p=12.0 bg=surface border=border border-w=1.0 r=10.0 shadow=black/20 shadow-y=8.0 shadow-blur=24.0
    col w=fill gap=7.0
      row w=fill gap=8.0 align=center
        text "⌕" size=18.0 @text-muted
        input "" label="Search pages" <-> search_query hint="Search Eddy's Notion" w=fill p=8.0
          active bg=transparent border=transparent value=fg placeholder=faint selection=primary border-w=0.0
          focused bg=transparent border=transparent value=fg placeholder=faint selection=primary border-w=0.0
        button label="Close search" p=5.0 style=text -> close_search
          text "esc" size=10.0 @text-faint
      box w=fill h=1.0 bg=border
        text ""
      text "JUMP TO" size=10.0 @text-faint font-bold
      if page_matches(search_query, home_title)
        PageItem icon="◆" title=home_title page="home" selected=(selected_page == "home")
      if page_matches(search_query, roadmap_title)
        PageItem icon="▦" title=roadmap_title page="roadmap" selected=(selected_page == "roadmap")
      if page_matches(search_query, launch_title)
        PageItem icon="✓" title=launch_title page="launch" selected=(selected_page == "launch")
      if page_matches(search_query, meeting_title)
        PageItem icon="▤" title=meeting_title page="meeting" selected=(selected_page == "meeting")
      if !page_matches(search_query, home_title) && !page_matches(search_query, roadmap_title) && !page_matches(search_query, launch_title) && !page_matches(search_query, meeting_title)
        text "No pages found" size=13.0 @text-muted

component ShareDialog(invite_email:str, invite_sent:bool)
  box w=470.0 p=20.0 bg=surface border=border border-w=1.0 r=10.0 shadow=black/20 shadow-y=8.0 shadow-blur=24.0
    col w=fill gap=14.0
      row w=fill align=center
        text "Share this page" w=fill size=17.0 @text-fg font-bold
        button label="Close share dialog" p=5.0 style=text -> close_share
          text "×" size=18.0 @text-muted
      text "Invite someone to collaborate on this document." size=13.0 @text-muted
      row w=fill gap=8.0
        input "" label="Email address" <-> invite_email hint="Email or name" w=fill p=9.0
          active bg=surface border=border value=fg placeholder=faint selection=primary border-w=1.0 r=6.0
          focused border=primary border-w=1.0
        button "Invite" disabled=empty(trim(invite_email)) p=9.0 -> send_invite
          active bg=primary text=white r=6.0
          hovered bg=primary/85
          disabled bg=hover text=faint
      if invite_sent
        box w=fill p=10.0 bg=blue_soft r=6.0
          text "Invitation ready to send" size=12.0 @text-primary
      box w=fill h=1.0 bg=border
        text ""
      row w=fill gap=10.0 align=center
        box w=32.0 h=32.0 align-x=center align-y=center bg=hover r=16.0
          text "↗" size=14.0 @text-muted
        col w=fill gap=2.0
          text "Publish" size=13.0 @text-fg font-bold
          text "Anyone with the link can view" size=11.0 @text-muted
        text "Off" size=12.0 @text-faint

on noop

view
  overlay when=search_open dismiss=close_search backdrop=black/18 p=24.0 align-x=center align-y=start
    content
      overlay when=share_open dismiss=close_share backdrop=black/18 p=24.0 align-x=center align-y=center
        content
          box w=fill h=fill bg=bg
            row w=fill h=fill
              if sidebar_open
                Sidebar selected_page=selected_page home_title=home_title roadmap_title=roadmap_title launch_title=launch_title meeting_title=meeting_title untitled_title=untitled_title
              if !sidebar_open
                MiniSidebar
              col w=fill h=fill
                Topbar favorite=favorite
                match selected_page
                  "home"
                    Document title=home_title state=home_document icon="◆" -> home_editor_changed _
                  "roadmap"
                    Document title=roadmap_title state=roadmap_document icon="▦" -> roadmap_editor_changed _
                  "launch"
                    Document title=launch_title state=launch_document icon="✓" -> launch_editor_changed _
                  "meeting"
                    Document title=meeting_title state=meeting_document icon="▤" -> meeting_editor_changed _
                  "untitled"
                    Document title=untitled_title state=untitled_document icon="□" -> untitled_editor_changed _
        layer
          ShareDialog invite_email=invite_email invite_sent=invite_sent
    layer
      SearchDialog search_query=search_query selected_page=selected_page home_title=home_title roadmap_title=roadmap_title launch_title=launch_title meeting_title=meeting_title

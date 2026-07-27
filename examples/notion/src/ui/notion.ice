app Notion
  title "Notion"
  id "dev.ducktape.ice.notion"
  font "../../../showcase/assets/fonts/Geist-Regular.ttf"
  font "../../../showcase/assets/fonts/Geist-Bold.ttf"
  text-size 14
  antialiasing true
  window
    size 1280 800
    min-size 860 600
    position centered

font geist family="Geist" default=true

extern crate::helpers
  sync page_matches(query:str, title:str) -> bool
  sync page_link(page:str) -> str
  sync selected_access(access:str?) -> str

extern crate::editor
  BlockEditorState()
  BlockEditorEvent()
  sync block_editor_state(template:str) -> BlockEditorState
  sync block_editor_apply(state:BlockEditorState, event:BlockEditorEvent) -> BlockEditorState
  sync block_editor_pending_focus(state:BlockEditorState) -> i64
  sync block_editor_clear_focus(state:BlockEditorState) -> BlockEditorState
  sync block_editor_toggle_comments(state:BlockEditorState) -> BlockEditorState
  sync block_editor_comments_open(state:BlockEditorState) -> bool
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
  home_favorite = false
  roadmap_favorite = false
  launch_favorite = false
  meeting_favorite = false
  untitled_favorite = false
  invite_email = ""
  invite_access_options = ["Can edit", "Can comment", "Can view"]
  invite_access_choice:str? = none
  invited_email = ""
  invited_access = ""
  link_copied = false
  pending_focus = 0
  home_title = "Product strategy"
  roadmap_title = "Product roadmap"
  launch_title = "Launch plan"
  meeting_title = "Weekly meeting"
  untitled_title = "Untitled"
  home_document:BlockEditorState = block_editor_state("home")
  roadmap_document:BlockEditorState = block_editor_state("roadmap")
  launch_document:BlockEditorState = block_editor_state("launch")
  meeting_document:BlockEditorState = block_editor_state("meeting")
  untitled_document:BlockEditorState = block_editor_state("untitled")

preset test
  state
    selected_page = "home"
    sidebar_open = true
    search_open = false
    search_query = ""
    share_open = false
    home_favorite = false
    roadmap_favorite = false
    launch_favorite = false
    meeting_favorite = false
    untitled_favorite = false
    invite_email = ""
    invite_access_choice = none
    invited_email = ""
    invited_access = ""
    link_copied = false
    pending_focus = 0
    home_title = "Product strategy"
    roadmap_title = "Product roadmap"
    launch_title = "Launch plan"
    meeting_title = "Weekly meeting"
    untitled_title = "Untitled"
    home_document = block_editor_state("home")
    roadmap_document = block_editor_state("roadmap")
    launch_document = block_editor_state("launch")
    meeting_document = block_editor_state("meeting")
    untitled_document = block_editor_state("untitled")

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

on toggle_home_favorite
  home_favorite = !home_favorite

on toggle_roadmap_favorite
  roadmap_favorite = !roadmap_favorite

on toggle_launch_favorite
  launch_favorite = !launch_favorite

on toggle_meeting_favorite
  meeting_favorite = !meeting_favorite

on toggle_untitled_favorite
  untitled_favorite = !untitled_favorite

on home_comments_toggled
  home_document = block_editor_toggle_comments(home_document)

on roadmap_comments_toggled
  roadmap_document = block_editor_toggle_comments(roadmap_document)

on launch_comments_toggled
  launch_document = block_editor_toggle_comments(launch_document)

on meeting_comments_toggled
  meeting_document = block_editor_toggle_comments(meeting_document)

on untitled_comments_toggled
  untitled_document = block_editor_toggle_comments(untitled_document)

on open_share
  share_open = true
  search_open = false
  link_copied = false
  invite_access_choice = none

on close_share
  share_open = false

on new_page
  selected_page = "untitled"
  untitled_title = "Untitled"
  untitled_document = block_editor_state("untitled")

on send_invite
  return if empty(trim(invite_email))
  invited_email = trim(invite_email)
  invited_access = selected_access(invite_access_choice)
  invite_email = ""

on invite_access_changed(next)
  invite_access_choice = some(next)

on copy_link(link)
  link_copied = true
  task clipboard write link

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

component PageItem(icon:str, title:str, page:str, selected:bool)
  col #root w=fill
    if selected
      button #selected-button label=title w=fill p=6.0 -> navigate(trim(page))
        row w=fill gap=8.0 align=center
          text icon w=18.0 size=15.0 align-x=center @text-fg
          text title w=fill size=13.0 wrap=none @text-fg
        active bg=selected text=fg r=5.0
        hovered bg=selected
        pressed bg=hover
    if !selected
      button #button label=title w=fill p=6.0 -> navigate(trim(page))
        row w=fill gap=8.0 align=center
          text icon w=18.0 size=15.0 align-x=center @text-muted
          text title w=fill size=13.0 wrap=none @text-muted
        active bg=transparent text=muted r=5.0
        hovered bg=hover text=fg
        pressed bg=selected text=fg

component Sidebar(selected_page:str, home_favorite:bool, roadmap_favorite:bool, launch_favorite:bool, meeting_favorite:bool, untitled_favorite:bool, home_title:str, roadmap_title:str, launch_title:str, meeting_title:str, untitled_title:str)
  box #root w=244.0 h=fill bg=sidebar border=border border-w=1.0
    col w=fill h=fill p=8.0 gap=2.0
      row #workspace w=fill h=38.0 p=5.0 gap=8.0 align=center
        box w=24.0 h=24.0 align-x=center align-y=center bg=fg r=5.0
          text "E" size=12.0 @text-white font-bold
        text "Eddy's Notion" w=fill size=14.0 @text-fg font-bold
        button #collapse label="Collapse sidebar" p=4.0 style=text -> toggle_sidebar
          text "«" size=16.0 @text-muted
      button #search label="Search" w=fill p=7.0 -> open_search
        row w=fill gap=10.0 align=center
          text "⌕" w=18.0 size=17.0 align-x=center @text-muted
          text "Search" w=fill size=13.0 @text-muted
        active bg=transparent text=muted r=5.0
        hovered bg=hover text=fg
        pressed bg=selected text=fg
      if home_favorite || roadmap_favorite || launch_favorite || meeting_favorite || untitled_favorite
        col #favorites w=fill gap=2.0
          text "Favorites" size=11.0 @text-faint
          if home_favorite
            PageItem icon="◆" title=home_title page="home" selected=(selected_page == "home") #favorite-home
          if roadmap_favorite
            PageItem icon="▦" title=roadmap_title page="roadmap" selected=(selected_page == "roadmap") #favorite-roadmap
          if launch_favorite
            PageItem icon="✓" title=launch_title page="launch" selected=(selected_page == "launch") #favorite-launch
          if meeting_favorite
            PageItem icon="▤" title=meeting_title page="meeting" selected=(selected_page == "meeting") #favorite-meeting
          if untitled_favorite
            PageItem icon="□" title=untitled_title page="untitled" selected=(selected_page == "untitled") #favorite-untitled
      text "Private" size=11.0 @text-faint
      PageItem icon="◆" title=home_title page="home" selected=(selected_page == "home") #private-home
      PageItem icon="▦" title=roadmap_title page="roadmap" selected=(selected_page == "roadmap") #roadmap
      PageItem icon="✓" title=launch_title page="launch" selected=(selected_page == "launch") #launch
      PageItem icon="▤" title=meeting_title page="meeting" selected=(selected_page == "meeting") #meeting
      PageItem icon="□" title=untitled_title page="untitled" selected=(selected_page == "untitled") #untitled
      button #new-page label="New page" w=fill p=7.0 -> new_page
        row w=fill gap=10.0 align=center
          text "+" w=18.0 size=17.0 align-x=center @text-muted
          text "New page" size=13.0 @text-muted
        active bg=transparent text=muted r=5.0
        hovered bg=hover text=fg
        pressed bg=selected text=fg
      space w=fill h=fill

component MiniSidebar()
  box #root w=44.0 h=fill bg=sidebar border=border border-w=1.0 align-x=center
    col w=fill h=fill p=7.0 align=center
      button #expand label="Expand sidebar" p=6.0 style=text -> toggle_sidebar
        text "»" size=17.0 @text-muted
      button #search label="Search" p=6.0 style=text -> open_search
        text "⌕" size=17.0 @text-muted
      space w=fill h=fill
      box w=26.0 h=26.0 align-x=center align-y=center bg=fg r=5.0
        text "E" size=12.0 @text-white font-bold

component Topbar(current_title:str, current_icon:str)
  row #root w=fill h=46.0 px=12.0 gap=5.0 align=center
    text current_icon size=13.0 @text-muted
    text current_title w=fill size=12.0 @text-fg
    slot comments
    slot favorite_action
    button #share label="Share" p=6.0 -> open_share
      text "Share" size=12.0 @text-primary
      active bg=transparent text=primary r=5.0
      hovered bg=blue_soft
      pressed bg=selected

component Document(title:str, state:BlockEditorState, icon:str) -> BlockEditorEvent
  box #root w=fill h=fill px=28.0 align-x=center
    col w=fill h=fill max-w=920.0 pt=26.0
      text icon #icon size=42.0 @text-fg
      box w=fill px=30.0
        input "" #title label="Page title" <-> title hint="Untitled" w=fill p=0.0 text-size=36.0 font=geist
          active bg=transparent border=transparent value=fg placeholder=faint selection=primary border-w=0.0 r=0.0
          hovered bg=transparent border=transparent value=fg placeholder=faint border-w=0.0
          focused bg=transparent border=transparent value=fg placeholder=faint selection=primary border-w=0.0
      extern block_editor(state) #editor -> emit _

component SearchDialog(search_query:str, selected_page:str, home_title:str, roadmap_title:str, launch_title:str, meeting_title:str, untitled_title:str)
  box #root w=520.0 p=12.0 bg=surface border=border border-w=1.0 r=10.0 shadow=black/20 shadow-y=8.0 shadow-blur=24.0
    col w=fill gap=7.0
      row w=fill gap=8.0 align=center
        text "⌕" size=18.0 @text-muted
        input "" #query label="Search pages" <-> search_query hint="Search Eddy's Notion" w=fill p=8.0
          active bg=transparent border=transparent value=fg placeholder=faint selection=primary border-w=0.0
          focused bg=transparent border=transparent value=fg placeholder=faint selection=primary border-w=0.0
        button #close label="Close search" p=5.0 style=text -> close_search
          text "esc" size=10.0 @text-faint
      box w=fill h=1.0 bg=border
        text ""
      if empty(trim(search_query))
        text "RECENT" size=10.0 @text-faint font-bold
      if !empty(trim(search_query))
        text "BEST MATCHES" size=10.0 @text-faint font-bold
      if page_matches(search_query, home_title)
        PageItem icon="◆" title=home_title page="home" selected=(selected_page == "home") #home-result
      if page_matches(search_query, roadmap_title)
        PageItem icon="▦" title=roadmap_title page="roadmap" selected=(selected_page == "roadmap") #roadmap-result
      if page_matches(search_query, launch_title)
        PageItem icon="✓" title=launch_title page="launch" selected=(selected_page == "launch") #launch-result
      if page_matches(search_query, meeting_title)
        PageItem icon="▤" title=meeting_title page="meeting" selected=(selected_page == "meeting") #meeting-result
      if page_matches(search_query, untitled_title)
        PageItem icon="□" title=untitled_title page="untitled" selected=(selected_page == "untitled") #untitled-result
      if !page_matches(search_query, home_title) && !page_matches(search_query, roadmap_title) && !page_matches(search_query, launch_title) && !page_matches(search_query, meeting_title) && !page_matches(search_query, untitled_title)
        text "No pages found" size=13.0 @text-muted

component ShareDialog(invite_email:str, invite_access_options:[str], invite_access_choice:str?, invited_email:str, invited_access:str, link_copied:bool, page_link:str)
  box #root w=470.0 p=18.0 bg=surface border=border border-w=1.0 r=10.0 shadow=black/20 shadow-y=8.0 shadow-blur=24.0
    col w=fill gap=14.0
      row w=fill align=center
        text "Share this page" w=fill size=17.0 @text-fg font-bold
        button #close label="Close share dialog" p=5.0 style=text -> close_share
          text "×" size=18.0 @text-muted
      row w=fill gap=8.0
        input "" #email label="Email address" <-> invite_email hint="Email or name" w=fill p=9.0
          active bg=surface border=border value=fg placeholder=faint selection=primary border-w=1.0 r=6.0
          focused border=primary border-w=1.0
        pick invite_access_options invite_access_choice #access hint="Can edit" w=112.0 p=9.0 -> invite_access_changed _
        button "Invite" #invite disabled=empty(trim(invite_email)) p=9.0 -> send_invite
          active bg=primary text=white r=6.0
          hovered bg=primary/85
          disabled bg=hover text=faint
      if !empty(invited_email)
        row #invited w=fill gap=10.0 align=center
          box w=30.0 h=30.0 align-x=center align-y=center bg=fg r=15.0
            text "E" size=11.0 @text-white font-bold
          col w=fill gap=1.0
            text invited_email size=12.0 @text-fg
            text "Invited" size=10.0 @text-muted
          text invited_access size=11.0 @text-muted
      box w=fill h=1.0 bg=border
        text ""
      row #general-access w=fill gap=10.0 align=center
        box w=32.0 h=32.0 align-x=center align-y=center bg=hover r=16.0
          text "●" size=9.0 @text-muted
        col w=fill gap=2.0
          text "General access" size=13.0 @text-fg font-bold
          text "Only people invited" size=11.0 @text-muted
        if link_copied
          button "Copied" #copied-link p=7.0 -> copy_link(page_link)
            active bg=hover text=muted r=5.0
        if !link_copied
          button "Copy link" #copy-link p=7.0 -> copy_link(page_link)
            active bg=surface text=fg border=border border-w=1.0 r=5.0
            hovered bg=hover

test page_item_component
  preset test
  viewport 300 90
  mount
    PageItem icon="▦" title=roadmap_title page="roadmap" selected=false #item
  target root = #item/root
  target button = #item/root/button
  expect root.width ~= 300.0
  expect button.kind == "button"
  expect text "Product roadmap" within root
  click button
  expect selected_page == "roadmap"

test sidebar_component
  preset test
  viewport 300 640
  mount
    Sidebar selected_page=selected_page home_favorite=true roadmap_favorite=false launch_favorite=false meeting_favorite=false untitled_favorite=false home_title=home_title roadmap_title=roadmap_title launch_title=launch_title meeting_title=meeting_title untitled_title=untitled_title #sidebar
  target root = #sidebar/root
  target collapse = #sidebar/root/workspace/collapse
  target search = #sidebar/root/search
  target launch = #sidebar/root/launch/root/button
  expect root.width ~= 244.0
  expect text "Favorites" within root
  expect text "Private" within root
  expect no text "Updates" within root
  click launch
  expect selected_page == "launch"
  click search
  expect search_open
  click collapse
  expect !sidebar_open

test mini_sidebar_component
  preset test
  viewport 100 320
  mount
    MiniSidebar #mini
  target root = #mini/root
  target expand = #mini/root/expand
  target search = #mini/root/search
  expect root.width ~= 44.0
  click search
  expect search_open
  click expand
  expect !sidebar_open

test topbar_component
  preset test
  viewport 640 80
  mount
    Topbar current_title=home_title current_icon="◆" #topbar
      comments:
        button #comments label="Open comments" p=6.0 style=text -> home_comments_toggled
          text "☵" size=15.0 @text-muted
      favorite_action:
        row
          if home_favorite
            button #remove-favorite label="Remove from favorites" p=6.0 style=text -> toggle_home_favorite
              text "★" size=16.0 @text-fg
          if !home_favorite
            button #favorite label="Add to favorites" p=6.0 style=text -> toggle_home_favorite
              text "☆" size=16.0 @text-muted
  target root = #topbar/root
  target comments = #topbar/root/comments
  target favorite_button = #topbar/root/favorite
  target share = #topbar/root/share
  expect root.height ~= 46.0
  expect text "Product strategy" within root
  expect !block_editor_comments_open(home_document)
  click comments
  expect block_editor_comments_open(home_document)
  click favorite_button
  expect home_favorite
  click share
  expect share_open

test document_component
  preset test
  viewport 960 650
  mount
    Document title=home_title state=home_document icon="◆" #document -> home_editor_changed _
  target root = #document/root
  target title = #document/root/title
  target editor = #document/root/editor
  expect root.width ~= 960.0
  expect title.value == "Product strategy"
  expect title.font.family == family.named("Geist")
  expect editor.visible
  expect text "Build a calmer place to work." within editor
  expect no text "BLOCKS" within editor
  expect no text "threads ·" within editor
  click title
  type " updated"
  expect title.value != "Product strategy"
  expect home_title == title.value

test search_dialog_component
  preset test
  viewport 600 520
  mount
    SearchDialog search_query=search_query selected_page=selected_page home_title=home_title roadmap_title=roadmap_title launch_title=launch_title meeting_title=meeting_title untitled_title=untitled_title #search-dialog
  target root = #search-dialog/root
  target query = #search-dialog/root/query
  target roadmap = #search-dialog/root/roadmap-result/root/button
  expect root.width ~= 520.0
  expect text "RECENT" within root
  expect text "Untitled" within root
  click query
  type "road"
  expect query.value == "road"
  expect text "BEST MATCHES" within root
  expect text "Product roadmap" within root
  expect no text "Launch plan" within root
  click roadmap
  expect selected_page == "roadmap"

test share_dialog_component
  preset test
  viewport 560 420
  mount
    ShareDialog invite_email=invite_email invite_access_options=invite_access_options invite_access_choice=invite_access_choice invited_email=invited_email invited_access=invited_access link_copied=link_copied page_link=page_link("home") #share-dialog
  target root = #share-dialog/root
  target email = #share-dialog/root/email
  target access = #share-dialog/root/access
  target invite = #share-dialog/root/invite
  target invited = #share-dialog/root/invited
  target copy_link = #share-dialog/root/general-access/copy-link
  expect root.width ~= 470.0
  expect text "Only people invited" within root
  expect missing invited
  click email
  type "collaborator@example.com"
  dispatch invite_access_changed("Can view")
  expect access.visible
  expect selected_access(invite_access_choice) == "Can view"
  expect text "Can view" within root
  click invite
  expect invite_email == ""
  expect invited_email == "collaborator@example.com"
  expect invited_access == "Can view"
  expect exists invited
  expect text "Can view" within invited
  click copy_link
  expect link_copied
  expect text "Copied" within root

test app_flow
  preset test
  viewport 1280 800
  target app = #app
  target sidebar = #app/shell/sidebar/root
  target search = #app/shell/sidebar/root/search
  target launch = #app/shell/sidebar/root/launch/root/button
  target page_title = #app/shell/home-page/home-document/root/title
  target comments = #app/shell/home-page/home-topbar/root/comments
  target favorite_button = #app/shell/home-page/home-topbar/root/favorite
  target share = #app/shell/home-page/home-topbar/root/share
  target favorites = #app/shell/sidebar/root/favorites
  target new_page = #app/shell/sidebar/root/new-page
  target search_dialog = #search-dialog/root
  target search_query_input = #search-dialog/root/query
  target share_dialog = #share-dialog/root
  expect app.width ~= 1280.0
  expect sidebar.width ~= 244.0
  expect page_title.value == "Product strategy"
  expect missing search_dialog
  expect missing favorites
  click favorite_button
  expect home_favorite
  expect exists favorites
  click search
  expect exists search_dialog
  expect search_query == ""
  expect search_query_input.value == ""
  dispatch close_search
  click comments
  expect block_editor_comments_open(home_document)
  expect text "Comments"
  click share
  expect exists share_dialog
  dispatch close_share
  click launch
  expect selected_page == "launch"
  expect text "Finalize announcement"
  click new_page
  expect selected_page == "untitled"
  expect untitled_title == "Untitled"
  expect text "Type '/' for commands"

test minimum_window_layout
  preset test
  viewport 860 600
  target app = #app
  target shell = #app/shell
  target sidebar = #app/shell/sidebar/root
  target mini_sidebar = #app/shell/mini-sidebar/root
  target page = #app/shell/home-page
  target topbar = #app/shell/home-page/home-topbar/root
  target document = #app/shell/home-page/home-document/root
  target title = #app/shell/home-page/home-document/root/title
  target editor = #app/shell/home-page/home-document/root/editor
  target search_dialog = #search-dialog/root
  target share_dialog = #share-dialog/root
  expect app.width ~= 860.0
  expect app.height ~= 600.0
  expect shell.width ~= app.width
  expect shell.height ~= app.height
  expect sidebar.x ~= app.x
  expect sidebar.width ~= 244.0
  expect sidebar.height ~= app.height
  expect missing mini_sidebar
  expect page.x ~= sidebar.right
  expect page.right ~= app.right
  expect page.height ~= app.height
  expect topbar.y ~= page.y
  expect topbar.width ~= page.width
  expect topbar.height ~= 46.0
  expect document.y ~= topbar.bottom
  expect document.bottom ~= page.bottom
  expect document.width ~= page.width
  expect title.x > document.x
  expect editor.y >= title.bottom
  dispatch toggle_sidebar
  expect missing sidebar
  expect exists mini_sidebar
  expect mini_sidebar.x ~= app.x
  expect mini_sidebar.width ~= 44.0
  expect page.x ~= mini_sidebar.right
  expect page.right ~= app.right
  dispatch open_search
  expect search_dialog.x ~= 170.0
  expect search_dialog.y ~= 24.0
  expect search_dialog.right ~= app.right - 170.0
  dispatch open_share
  expect missing search_dialog
  expect share_dialog.x ~= 195.0
  expect share_dialog.right ~= app.right - 195.0
  expect share_dialog.y >= app.y + 24.0
  expect share_dialog.bottom <= app.bottom - 24.0

view
  overlay when=search_open dismiss=close_search backdrop=black/18 p=24.0 align-x=center align-y=start
    content
      overlay when=share_open dismiss=close_share backdrop=black/18 p=24.0 align-x=center align-y=center
        content
          box #app w=fill h=fill bg=bg
            row #shell w=fill h=fill
              if sidebar_open
                Sidebar selected_page=selected_page home_favorite=home_favorite roadmap_favorite=roadmap_favorite launch_favorite=launch_favorite meeting_favorite=meeting_favorite untitled_favorite=untitled_favorite home_title=home_title roadmap_title=roadmap_title launch_title=launch_title meeting_title=meeting_title untitled_title=untitled_title #sidebar
              if !sidebar_open
                MiniSidebar #mini-sidebar
              match selected_page
                "home"
                  col #home-page w=fill h=fill
                    Topbar current_title=home_title current_icon="◆" #home-topbar
                      comments:
                        button #comments label="Open comments" p=6.0 style=text -> home_comments_toggled
                          text "☵" size=15.0 @text-muted
                      favorite_action:
                        row
                          if home_favorite
                            button #remove-favorite label="Remove from favorites" p=6.0 style=text -> toggle_home_favorite
                              text "★" size=16.0 @text-fg
                          if !home_favorite
                            button #favorite label="Add to favorites" p=6.0 style=text -> toggle_home_favorite
                              text "☆" size=16.0 @text-muted
                    Document title=home_title state=home_document icon="◆" #home-document -> home_editor_changed _
                "roadmap"
                  col #roadmap-page w=fill h=fill
                    Topbar current_title=roadmap_title current_icon="▦" #roadmap-topbar
                      comments:
                        button #comments label="Open comments" p=6.0 style=text -> roadmap_comments_toggled
                          text "☵" size=15.0 @text-muted
                      favorite_action:
                        row
                          if roadmap_favorite
                            button #remove-favorite label="Remove from favorites" p=6.0 style=text -> toggle_roadmap_favorite
                              text "★" size=16.0 @text-fg
                          if !roadmap_favorite
                            button #favorite label="Add to favorites" p=6.0 style=text -> toggle_roadmap_favorite
                              text "☆" size=16.0 @text-muted
                    Document title=roadmap_title state=roadmap_document icon="▦" #roadmap-document -> roadmap_editor_changed _
                "launch"
                  col #launch-page w=fill h=fill
                    Topbar current_title=launch_title current_icon="✓" #launch-topbar
                      comments:
                        button #comments label="Open comments" p=6.0 style=text -> launch_comments_toggled
                          text "☵" size=15.0 @text-muted
                      favorite_action:
                        row
                          if launch_favorite
                            button #remove-favorite label="Remove from favorites" p=6.0 style=text -> toggle_launch_favorite
                              text "★" size=16.0 @text-fg
                          if !launch_favorite
                            button #favorite label="Add to favorites" p=6.0 style=text -> toggle_launch_favorite
                              text "☆" size=16.0 @text-muted
                    Document title=launch_title state=launch_document icon="✓" #launch-document -> launch_editor_changed _
                "meeting"
                  col #meeting-page w=fill h=fill
                    Topbar current_title=meeting_title current_icon="▤" #meeting-topbar
                      comments:
                        button #comments label="Open comments" p=6.0 style=text -> meeting_comments_toggled
                          text "☵" size=15.0 @text-muted
                      favorite_action:
                        row
                          if meeting_favorite
                            button #remove-favorite label="Remove from favorites" p=6.0 style=text -> toggle_meeting_favorite
                              text "★" size=16.0 @text-fg
                          if !meeting_favorite
                            button #favorite label="Add to favorites" p=6.0 style=text -> toggle_meeting_favorite
                              text "☆" size=16.0 @text-muted
                    Document title=meeting_title state=meeting_document icon="▤" #meeting-document -> meeting_editor_changed _
                "untitled"
                  col #untitled-page w=fill h=fill
                    Topbar current_title=untitled_title current_icon="□" #untitled-topbar
                      comments:
                        button #comments label="Open comments" p=6.0 style=text -> untitled_comments_toggled
                          text "☵" size=15.0 @text-muted
                      favorite_action:
                        row
                          if untitled_favorite
                            button #remove-favorite label="Remove from favorites" p=6.0 style=text -> toggle_untitled_favorite
                              text "★" size=16.0 @text-fg
                          if !untitled_favorite
                            button #favorite label="Add to favorites" p=6.0 style=text -> toggle_untitled_favorite
                              text "☆" size=16.0 @text-muted
                    Document title=untitled_title state=untitled_document icon="□" #untitled-document -> untitled_editor_changed _
        layer
          ShareDialog invite_email=invite_email invite_access_options=invite_access_options invite_access_choice=invite_access_choice invited_email=invited_email invited_access=invited_access link_copied=link_copied page_link=page_link(selected_page) #share-dialog
    layer
      SearchDialog search_query=search_query selected_page=selected_page home_title=home_title roadmap_title=roadmap_title launch_title=launch_title meeting_title=meeting_title untitled_title=untitled_title #search-dialog

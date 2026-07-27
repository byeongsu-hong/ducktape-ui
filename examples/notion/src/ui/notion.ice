app Notion
  title "Notion"
  id "dev.ducktape.ice.notion"
  font "../../assets/fonts/Inter-Regular.ttf"
  font "../../assets/fonts/Inter-Bold.ttf"
  text-size 14
  antialiasing true
  window
    size 1280 800
    min-size 860 600
    position centered

font inter family="Inter" default=true
font inter_bold family="Inter" weight=bold

use "styles.ice" as notion

extern crate::helpers
  sync page_matches(query:str, title:str) -> bool
  sync page_link(page:str) -> str
  sync selected_access(access:str?) -> str

extern crate::editor
  BlockEditorState()
  BlockEditorEvent()
  sync block_editor_state(template:str) -> BlockEditorState
  sync block_editor_apply(state:BlockEditorState, event:BlockEditorEvent) -> BlockEditorState
  sync block_editor_should_focus(state:BlockEditorState) -> bool
  sync block_editor_should_focus_search(state:BlockEditorState) -> bool
  sync block_editor_clear_focus(state:BlockEditorState) -> BlockEditorState
  sync block_editor_toggle_comments(state:BlockEditorState) -> BlockEditorState
  sync block_editor_comments_open(state:BlockEditorState) -> bool
  task block_editor_focus(search:bool) -> bool
  component block_editor(state:&BlockEditorState) -> BlockEditorEvent

theme contract AppTheme
  bg
  surface
  sidebar
  fg
  muted
  faint
  hover
  selected
  border
  primary
  danger
  blue_soft
palette app for AppTheme
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

enum ModalState
  closed
  search
  share

state
  selected_page = "home"
  sidebar_open = true
  modal:ModalState = ModalState.closed
  search_query = ""
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
  pending_editor_focus = false
  pending_editor_search_focus = false
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

derived
  normalized_invite_email = trim(invite_email)
  invite_ready = !empty(normalized_invite_email)
  has_search_query = !empty(trim(search_query))

preset test
  state
    selected_page = "home"
    sidebar_open = true
    modal = ModalState.closed
    search_query = ""
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
    pending_editor_focus = false
    pending_editor_search_focus = false
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
  modal = ModalState.closed

on toggle_sidebar
  sidebar_open = !sidebar_open

on open_search
  modal = ModalState.search

on close_search
  modal = ModalState.closed

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
  modal = ModalState.share
  link_copied = false
  invite_access_choice = none

on close_share
  modal = ModalState.closed

on new_page
  selected_page = "untitled"
  untitled_title = "Untitled"
  untitled_document = block_editor_state("untitled")

on send_invite
  let email = normalized_invite_email
  return if !invite_ready
  invited_email = email
  invited_access = selected_access(invite_access_choice)
  invite_email = ""

on invite_access_changed(next)
  invite_access_choice = some(next)

on copy_link(link)
  link_copied = true
  task clipboard write link

on home_editor_changed(event)
  let next = block_editor_apply(home_document, event)
  let focus_editor = block_editor_should_focus(next)
  let focus_search = block_editor_should_focus_search(next)
  pending_editor_focus = focus_editor
  pending_editor_search_focus = focus_search
  home_document = block_editor_clear_focus(next)
  return if !focus_editor
  task block_editor_focus(focus_search) -> editor_focused _

on roadmap_editor_changed(event)
  let next = block_editor_apply(roadmap_document, event)
  let focus_editor = block_editor_should_focus(next)
  let focus_search = block_editor_should_focus_search(next)
  pending_editor_focus = focus_editor
  pending_editor_search_focus = focus_search
  roadmap_document = block_editor_clear_focus(next)
  return if !focus_editor
  task block_editor_focus(focus_search) -> editor_focused _

on launch_editor_changed(event)
  let next = block_editor_apply(launch_document, event)
  let focus_editor = block_editor_should_focus(next)
  let focus_search = block_editor_should_focus_search(next)
  pending_editor_focus = focus_editor
  pending_editor_search_focus = focus_search
  launch_document = block_editor_clear_focus(next)
  return if !focus_editor
  task block_editor_focus(focus_search) -> editor_focused _

on meeting_editor_changed(event)
  let next = block_editor_apply(meeting_document, event)
  let focus_editor = block_editor_should_focus(next)
  let focus_search = block_editor_should_focus_search(next)
  pending_editor_focus = focus_editor
  pending_editor_search_focus = focus_search
  meeting_document = block_editor_clear_focus(next)
  return if !focus_editor
  task block_editor_focus(focus_search) -> editor_focused _

on untitled_editor_changed(event)
  let next = block_editor_apply(untitled_document, event)
  let focus_editor = block_editor_should_focus(next)
  let focus_search = block_editor_should_focus_search(next)
  pending_editor_focus = focus_editor
  pending_editor_search_focus = focus_search
  untitled_document = block_editor_clear_focus(next)
  return if !focus_editor
  task block_editor_focus(focus_search) -> editor_focused _

on editor_focused(_focused)
  pending_editor_focus = false
  pending_editor_search_focus = false

component PageItem(icon:str, title:str, page:str, selected:bool)
  emits
    navigate(str)
  col #root w=fill
    if selected
      button #selected-button label=title w=fill p=6.0 -> emit(navigate, trim(page))
        row w=fill gap=8.0 align=center
          text icon w=18.0 size=15.0 align-x=center @text-fg
          text title w=fill size=13.0 wrap=none @text-fg
        active bg=selected text=fg r=5.0
        hovered bg=selected
        pressed bg=hover
    if !selected
      button #button label=title w=fill p=6.0 -> emit(navigate, trim(page))
        row w=fill gap=8.0 align=center
          text icon w=18.0 size=15.0 align-x=center @text-muted
          text title w=fill size=13.0 wrap=none @text-muted
        active bg=transparent text=muted r=5.0
        hovered bg=hover text=fg
        pressed bg=selected text=fg

component Sidebar(selected_page:str, home_favorite:bool, roadmap_favorite:bool, launch_favorite:bool, meeting_favorite:bool, untitled_favorite:bool, home_title:str, roadmap_title:str, launch_title:str, meeting_title:str, untitled_title:str)
  emits
    toggle_sidebar
    open_search
    navigate(str)
    new_page
  box #root w=244.0 h=fill bg=sidebar border=border border-w=1.0
    col w=fill h=fill p=8.0 gap=2.0
      row #workspace w=fill h=38.0 p=5.0 gap=8.0 align=center
        box w=24.0 h=24.0 align-x=center align-y=center bg=fg r=5.0
          text "E" size=12.0 @text-white font-bold
        text "Eddy's Notion" w=fill size=14.0 @text-fg font-bold
        button #collapse label="Collapse sidebar" p=4.0 style=text -> emit toggle_sidebar
          text "«" size=16.0 @text-muted
      button #search label="Search" w=fill p=7.0 -> emit open_search
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
            PageItem #favorite-home
              with
                icon="◆"
                title=home_title
                page="home"
                selected=(selected_page == "home")
              events
                navigate -> emit navigate _
          if roadmap_favorite
            PageItem #favorite-roadmap
              with
                icon="▦"
                title=roadmap_title
                page="roadmap"
                selected=(selected_page == "roadmap")
              events
                navigate -> emit navigate _
          if launch_favorite
            PageItem #favorite-launch
              with
                icon="✓"
                title=launch_title
                page="launch"
                selected=(selected_page == "launch")
              events
                navigate -> emit navigate _
          if meeting_favorite
            PageItem #favorite-meeting
              with
                icon="▤"
                title=meeting_title
                page="meeting"
                selected=(selected_page == "meeting")
              events
                navigate -> emit navigate _
          if untitled_favorite
            PageItem #favorite-untitled
              with
                icon="□"
                title=untitled_title
                page="untitled"
                selected=(selected_page == "untitled")
              events
                navigate -> emit navigate _
      text "Private" size=11.0 @text-faint
      PageItem #private-home
        with
          icon="◆"
          title=home_title
          page="home"
          selected=(selected_page == "home")
        events
          navigate -> emit navigate _
      PageItem #roadmap
        with
          icon="▦"
          title=roadmap_title
          page="roadmap"
          selected=(selected_page == "roadmap")
        events
          navigate -> emit navigate _
      PageItem #launch
        with
          icon="✓"
          title=launch_title
          page="launch"
          selected=(selected_page == "launch")
        events
          navigate -> emit navigate _
      PageItem #meeting
        with
          icon="▤"
          title=meeting_title
          page="meeting"
          selected=(selected_page == "meeting")
        events
          navigate -> emit navigate _
      PageItem #untitled
        with
          icon="□"
          title=untitled_title
          page="untitled"
          selected=(selected_page == "untitled")
        events
          navigate -> emit navigate _
      button #new-page label="New page" w=fill p=7.0 -> emit new_page
        row w=fill gap=10.0 align=center
          text "+" w=18.0 size=17.0 align-x=center @text-muted
          text "New page" size=13.0 @text-muted
        active bg=transparent text=muted r=5.0
        hovered bg=hover text=fg
        pressed bg=selected text=fg
      space w=fill h=fill

component MiniSidebar()
  emits
    toggle_sidebar
    open_search
  box #root w=44.0 h=fill bg=sidebar border=border border-w=1.0 align-x=center
    col w=fill h=fill p=7.0 align=center
      button #expand -> emit toggle_sidebar
        with
          label="Expand sidebar"
          style=text
          @notion::toolbar_action
        text "»" size=17.0 @text-muted
      button #search label="Search" style=text @notion::toolbar_action -> emit open_search
        text "⌕" size=17.0 @text-muted
      space w=fill h=fill
      box w=26.0 h=26.0 align-x=center align-y=center bg=fg r=5.0
        text "E" size=12.0 @text-white font-bold

component Topbar(current_title:str, current_icon:str)
  emits
    open_share
  row #root w=fill h=46.0 px=12.0 gap=5.0 align=center
    text current_icon size=13.0 @text-muted
    text current_title w=fill size=12.0 @text-fg
    if provided(comments)
      slot comments?
    if provided(favorite_action)
      slot favorite_action?
    button #share label="Share" @notion::toolbar_action -> emit open_share
      text "Share" size=12.0 @text-primary
      active bg=transparent text=primary r=5.0
      hovered bg=blue_soft
      pressed bg=selected

component Document(bind title:str, state:BlockEditorState, icon:str="□") -> BlockEditorEvent
  box #root w=fill h=fill px=28.0 align-x=center
    col w=fill h=fill max-w=920.0 pt=26.0
      text icon #icon size=42.0 @text-fg
      box w=fill px=30.0
        input "" #title <-> title
          with
            label="Page title"
            hint="Untitled"
            w=fill
            p=0.0
            text-size=36.0
            font=inter_bold
          active bg=transparent border=transparent value=fg placeholder=faint selection=primary border-w=0.0 r=0.0
          hovered bg=transparent border=transparent value=fg placeholder=faint border-w=0.0
          focused bg=transparent border=transparent value=fg placeholder=faint selection=primary border-w=0.0
      extern block_editor(state) #editor -> emit _

component SearchDialog(bind search_query:str, selected_page:str, home_title:str, roadmap_title:str, launch_title:str, meeting_title:str, untitled_title:str, has_query:bool)
  emits
    close_search
    navigate(str)
  box #root
    with
      w=520.0
      p=12.0
      bg=surface
      border=border
      border-w=1.0
      r=10.0
      shadow=black/20
      shadow-y=8.0
      shadow-blur=24.0
    col w=fill gap=7.0
      row w=fill gap=8.0 align=center
        text "⌕" size=18.0 @text-muted
        input "" #query <-> search_query
          with
            label="Search pages"
            hint="Search Eddy's Notion"
            w=fill
            p=8.0
          active bg=transparent border=transparent value=fg placeholder=faint selection=primary border-w=0.0
          focused bg=transparent border=transparent value=fg placeholder=faint selection=primary border-w=0.0
        button #close label="Close search" style=text @notion::compact_action -> emit close_search
          text "esc" size=10.0 @text-faint
      box w=fill h=1.0 bg=border
        text ""
      if !has_query
        text "RECENT" size=10.0 @text-faint font-bold
      if has_query
        text "BEST MATCHES" size=10.0 @text-faint font-bold
      if page_matches(search_query, home_title)
        PageItem #home-result
          with
            icon="◆"
            title=home_title
            page="home"
            selected=(selected_page == "home")
          events
            navigate -> emit navigate _
      if page_matches(search_query, roadmap_title)
        PageItem #roadmap-result
          with
            icon="▦"
            title=roadmap_title
            page="roadmap"
            selected=(selected_page == "roadmap")
          events
            navigate -> emit navigate _
      if page_matches(search_query, launch_title)
        PageItem #launch-result
          with
            icon="✓"
            title=launch_title
            page="launch"
            selected=(selected_page == "launch")
          events
            navigate -> emit navigate _
      if page_matches(search_query, meeting_title)
        PageItem #meeting-result
          with
            icon="▤"
            title=meeting_title
            page="meeting"
            selected=(selected_page == "meeting")
          events
            navigate -> emit navigate _
      if page_matches(search_query, untitled_title)
        PageItem #untitled-result
          with
            icon="□"
            title=untitled_title
            page="untitled"
            selected=(selected_page == "untitled")
          events
            navigate -> emit navigate _
      if !page_matches(search_query, home_title) && !page_matches(search_query, roadmap_title) && !page_matches(search_query, launch_title) && !page_matches(search_query, meeting_title) && !page_matches(search_query, untitled_title)
        text "No pages found" size=13.0 @text-muted

component ShareDialog(bind invite_email:str, invite_access_options:[str], invite_access_choice:str?, invited_email:str, invited_access:str, link_copied:bool, page_link:str, invite_ready:bool)
  emits
    close_share
    invite_access_changed(str)
    send_invite
    copy_link(str)
  box #root
    with
      w=470.0
      p=18.0
      bg=surface
      border=border
      border-w=1.0
      r=10.0
      shadow=black/20
      shadow-y=8.0
      shadow-blur=24.0
    col w=fill gap=14.0
      row w=fill align=center
        text "Share this page" w=fill size=17.0 @text-fg font-bold
        button #close -> emit close_share
          with
            label="Close share dialog"
            style=text
            @notion::compact_action
          text "×" size=18.0 @text-muted
      row w=fill gap=8.0
        input "" #email label="Email address" <-> invite_email hint="Email or name" w=fill p=9.0
          active bg=surface border=border value=fg placeholder=faint selection=primary border-w=1.0 r=6.0
          focused border=primary border-w=1.0
        pick invite_access_options invite_access_choice #access -> emit invite_access_changed _
          with
            hint="Can edit"
            w=112.0
            p=9.0
        button "Invite" #invite disabled=!invite_ready p=9.0 -> emit send_invite
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
          button "Copied" #copied-link p=7.0 -> emit copy_link page_link
            active bg=hover text=muted r=5.0
        if !link_copied
          button "Copy link" #copy-link p=7.0 -> emit copy_link page_link
            active bg=surface text=fg border=border border-w=1.0 r=5.0
            hovered bg=hover

test page_item_component
  preset test
  viewport 300 90
  mount
    PageItem icon="▦" title=roadmap_title page="roadmap" selected=false #item
      events
        navigate -> navigate _
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
    Sidebar #sidebar
      with
        selected_page=selected_page
        home_favorite=true
        roadmap_favorite=false
        launch_favorite=false
        meeting_favorite=false
        untitled_favorite=false
        home_title=home_title
        roadmap_title=roadmap_title
        launch_title=launch_title
        meeting_title=meeting_title
        untitled_title=untitled_title
      events
        toggle_sidebar -> toggle_sidebar
        open_search -> open_search
        navigate -> navigate _
        new_page -> new_page
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
  expect modal == ModalState.search
  click collapse
  expect !sidebar_open

test mini_sidebar_component
  preset test
  viewport 100 320
  mount
    MiniSidebar #mini
      events
        toggle_sidebar -> toggle_sidebar
        open_search -> open_search
  target root = #mini/root
  target expand = #mini/root/expand
  target search = #mini/root/search
  expect root.width ~= 44.0
  click search
  expect modal == ModalState.search
  click expand
  expect !sidebar_open

test topbar_component
  preset test
  viewport 640 80
  mount
    Topbar current_title=home_title current_icon="◆" #topbar
      events
        open_share -> open_share
      comments:
        button #comments -> home_comments_toggled
          with
            label="Open comments"
            style=text
            @notion::toolbar_action
          text "☵" size=15.0 @text-muted
      favorite_action:
        row
          if home_favorite
            button #remove-favorite -> toggle_home_favorite
              with
                label="Remove from favorites"
                @notion::toolbar_action
                style=text
              text "★" size=16.0 @text-fg
          if !home_favorite
            button #favorite -> toggle_home_favorite
              with
                label="Add to favorites"
                style=text
                @notion::toolbar_action
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
  expect modal == ModalState.share

test document_component
  preset test
  viewport 960 650
  mount
    Document title<->home_title state=home_document #document -> home_editor_changed _
  target root = #document/root
  target title = #document/root/title
  target editor = #document/root/editor
  expect root.width ~= 960.0
  expect title.value == "Product strategy"
  expect title.font.family == family.named("Inter")
  expect title.font.weight == weight.bold()
  expect editor.visible
  expect text "Styles ▾" within editor
  expect no text "Preview" within editor
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
    SearchDialog #search-dialog
      with
        search_query<->search_query
        selected_page=selected_page
        home_title=home_title
        roadmap_title=roadmap_title
        launch_title=launch_title
        meeting_title=meeting_title
        untitled_title=untitled_title
        has_query=has_search_query
      events
        close_search -> close_search
        navigate -> navigate _
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
    ShareDialog #share-dialog
      with
        invite_email<->invite_email
        invite_access_options=invite_access_options
        invite_access_choice=invite_access_choice
        invited_email=invited_email
        invited_access=invited_access
        link_copied=link_copied
        page_link=page_link("home")
        invite_ready=invite_ready
      events
        close_share -> close_share
        invite_access_changed -> invite_access_changed _
        send_invite -> send_invite
        copy_link -> copy_link _
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
  expect text "Styles ▾"
  click new_page
  expect selected_page == "untitled"
  expect untitled_title == "Untitled"
  expect text "Styles ▾"

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
  overlay
    with
      when=(modal == ModalState.search)
      dismiss=close_search
      backdrop=black/18
      p=24.0
      align-x=center
      align-y=start
    content
      overlay
        with
          when=(modal == ModalState.share)
          dismiss=close_share
          backdrop=black/18
          p=24.0
          align-x=center
          align-y=center
        content
          box #app w=fill h=fill bg=bg
            row #shell w=fill h=fill
              if sidebar_open
                Sidebar #sidebar
                  with
                    selected_page=selected_page
                    home_favorite=home_favorite
                    roadmap_favorite=roadmap_favorite
                    launch_favorite=launch_favorite
                    meeting_favorite=meeting_favorite
                    untitled_favorite=untitled_favorite
                    home_title=home_title
                    roadmap_title=roadmap_title
                    launch_title=launch_title
                    meeting_title=meeting_title
                    untitled_title=untitled_title
                  events
                    toggle_sidebar -> toggle_sidebar
                    open_search -> open_search
                    navigate -> navigate _
                    new_page -> new_page
              if !sidebar_open
                MiniSidebar #mini-sidebar
                  events
                    toggle_sidebar -> toggle_sidebar
                    open_search -> open_search
              match selected_page
                "home"
                  col #home-page w=fill h=fill
                    Topbar current_title=home_title current_icon="◆" #home-topbar
                      events
                        open_share -> open_share
                      comments:
                        button #comments -> home_comments_toggled
                          with
                            label="Open comments"
                            @notion::toolbar_action
                            style=text
                          text "☵" size=15.0 @text-muted
                      favorite_action:
                        row
                          if home_favorite
                            button #remove-favorite -> toggle_home_favorite
                              with
                                label="Remove from favorites"
                                @notion::toolbar_action
                                style=text
                              text "★" size=16.0 @text-fg
                          if !home_favorite
                            button #favorite -> toggle_home_favorite
                              with
                                label="Add to favorites"
                                @notion::toolbar_action
                                style=text
                              text "☆" size=16.0 @text-muted
                    Document #home-document -> home_editor_changed _
                      with
                        title<->home_title
                        state=home_document
                        icon="◆"
                "roadmap"
                  col #roadmap-page w=fill h=fill
                    Topbar current_title=roadmap_title current_icon="▦" #roadmap-topbar
                      events
                        open_share -> open_share
                      comments:
                        button #comments -> roadmap_comments_toggled
                          with
                            label="Open comments"
                            @notion::toolbar_action
                            style=text
                          text "☵" size=15.0 @text-muted
                      favorite_action:
                        row
                          if roadmap_favorite
                            button #remove-favorite -> toggle_roadmap_favorite
                              with
                                label="Remove from favorites"
                                @notion::toolbar_action
                                style=text
                              text "★" size=16.0 @text-fg
                          if !roadmap_favorite
                            button #favorite -> toggle_roadmap_favorite
                              with
                                label="Add to favorites"
                                @notion::toolbar_action
                                style=text
                              text "☆" size=16.0 @text-muted
                    Document #roadmap-document -> roadmap_editor_changed _
                      with
                        title<->roadmap_title
                        state=roadmap_document
                        icon="▦"
                "launch"
                  col #launch-page w=fill h=fill
                    Topbar current_title=launch_title current_icon="✓" #launch-topbar
                      events
                        open_share -> open_share
                      comments:
                        button #comments -> launch_comments_toggled
                          with
                            label="Open comments"
                            @notion::toolbar_action
                            style=text
                          text "☵" size=15.0 @text-muted
                      favorite_action:
                        row
                          if launch_favorite
                            button #remove-favorite -> toggle_launch_favorite
                              with
                                label="Remove from favorites"
                                @notion::toolbar_action
                                style=text
                              text "★" size=16.0 @text-fg
                          if !launch_favorite
                            button #favorite -> toggle_launch_favorite
                              with
                                label="Add to favorites"
                                @notion::toolbar_action
                                style=text
                              text "☆" size=16.0 @text-muted
                    Document #launch-document -> launch_editor_changed _
                      with
                        title<->launch_title
                        state=launch_document
                        icon="✓"
                "meeting"
                  col #meeting-page w=fill h=fill
                    Topbar current_title=meeting_title current_icon="▤" #meeting-topbar
                      events
                        open_share -> open_share
                      comments:
                        button #comments -> meeting_comments_toggled
                          with
                            label="Open comments"
                            @notion::toolbar_action
                            style=text
                          text "☵" size=15.0 @text-muted
                      favorite_action:
                        row
                          if meeting_favorite
                            button #remove-favorite -> toggle_meeting_favorite
                              with
                                label="Remove from favorites"
                                @notion::toolbar_action
                                style=text
                              text "★" size=16.0 @text-fg
                          if !meeting_favorite
                            button #favorite -> toggle_meeting_favorite
                              with
                                label="Add to favorites"
                                @notion::toolbar_action
                                style=text
                              text "☆" size=16.0 @text-muted
                    Document #meeting-document -> meeting_editor_changed _
                      with
                        title<->meeting_title
                        state=meeting_document
                        icon="▤"
                "untitled"
                  col #untitled-page w=fill h=fill
                    Topbar current_title=untitled_title current_icon="□" #untitled-topbar
                      events
                        open_share -> open_share
                      comments:
                        button #comments -> untitled_comments_toggled
                          with
                            label="Open comments"
                            @notion::toolbar_action
                            style=text
                          text "☵" size=15.0 @text-muted
                      favorite_action:
                        row
                          if untitled_favorite
                            button #remove-favorite -> toggle_untitled_favorite
                              with
                                label="Remove from favorites"
                                @notion::toolbar_action
                                style=text
                              text "★" size=16.0 @text-fg
                          if !untitled_favorite
                            button #favorite -> toggle_untitled_favorite
                              with
                                label="Add to favorites"
                                @notion::toolbar_action
                                style=text
                              text "☆" size=16.0 @text-muted
                    Document #untitled-document -> untitled_editor_changed _
                      with
                        title<->untitled_title
                        state=untitled_document
        layer
          ShareDialog #share-dialog
            with
              invite_email<->invite_email
              invite_access_options=invite_access_options
              invite_access_choice=invite_access_choice
              invited_email=invited_email
              invited_access=invited_access
              link_copied=link_copied
              page_link=page_link(selected_page)
              invite_ready=invite_ready
            events
              close_share -> close_share
              invite_access_changed -> invite_access_changed _
              send_invite -> send_invite
              copy_link -> copy_link _
    layer
      SearchDialog #search-dialog
        with
          search_query<->search_query
          selected_page=selected_page
          home_title=home_title
          roadmap_title=roadmap_title
          launch_title=launch_title
          meeting_title=meeting_title
          untitled_title=untitled_title
          has_query=has_search_query
        events
          close_search -> close_search
          navigate -> navigate _

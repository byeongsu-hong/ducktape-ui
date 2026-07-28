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
  sync page_link(page:str) -> str
  sync selected_access(access:str?) -> str

extern crate::editor
  BlockEditorState()
  BlockEditorEvent()
  task block_editor_focus(search:bool) -> bool
  component block_editor(state:&BlockEditorState) -> BlockEditorEvent

extern crate::pages
  Page(id:str, icon:str, title:str, favorite:bool)
  PageStore(document:BlockEditorState)
  sync default_pages() -> PageStore
  sync selected_page_id(store:PageStore) -> str
  sync selected_page_title(store:PageStore) -> str
  sync selected_page_icon(store:PageStore) -> str
  sync selected_page_favorite(store:PageStore) -> bool
  sync visible_pages(store:PageStore) -> [Page]
  sync favorite_pages(store:PageStore) -> [Page]
  sync has_favorite_pages(store:PageStore) -> bool
  sync matching_pages(store:PageStore, query:str) -> [Page]
  sync has_matching_pages(store:PageStore, query:str) -> bool
  sync select_page(store:PageStore, id:str) -> PageStore
  sync reset_new_page(store:PageStore) -> PageStore
  sync rename_selected_page(store:PageStore, title:str) -> PageStore
  sync toggle_selected_favorite(store:PageStore) -> PageStore
  sync toggle_selected_comments(store:PageStore) -> PageStore
  sync apply_selected_editor_event(store:PageStore, event:BlockEditorEvent) -> PageStore
  sync selected_editor_should_focus(store:PageStore) -> bool
  sync selected_editor_should_focus_search(store:PageStore) -> bool
  sync clear_selected_editor_focus(store:PageStore) -> PageStore
  sync selected_comments_open(store:PageStore) -> bool

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
  pages:PageStore = default_pages()
  editing_title = "Product strategy"
  sidebar_open = true
  modal:ModalState = ModalState.closed
  search_query = ""
  invite_email = ""
  invite_access_options = ["Can edit", "Can comment", "Can view"]
  invite_access_choice:str? = none
  invited_email = ""
  invited_access = ""
  link_copied = false
  pending_editor_focus = false
  pending_editor_search_focus = false

derived
  normalized_invite_email = trim(invite_email)
  invite_ready = !empty(normalized_invite_email)
  has_search_query = !empty(trim(search_query))

preset test
  state
    pages = default_pages()
    editing_title = "Product strategy"
    sidebar_open = true
    modal = ModalState.closed
    search_query = ""
    invite_email = ""
    invite_access_choice = none
    invited_email = ""
    invited_access = ""
    link_copied = false
    pending_editor_focus = false
    pending_editor_search_focus = false

on navigate(page)
  pages = select_page(pages, page)
  editing_title = selected_page_title(pages)
  modal = ModalState.closed

on toggle_sidebar
  sidebar_open = !sidebar_open

on open_search
  modal = ModalState.search

on close_search
  modal = ModalState.closed

on toggle_favorite
  pages = toggle_selected_favorite(pages)

on comments_toggled
  pages = toggle_selected_comments(pages)

on open_share
  modal = ModalState.share
  link_copied = false
  invite_access_choice = none

on close_share
  modal = ModalState.closed

on new_page
  pages = reset_new_page(pages)
  editing_title = selected_page_title(pages)

on title_changed(title)
  pages = rename_selected_page(pages, title)
  editing_title = selected_page_title(pages)

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

on editor_changed(event)
  let next = apply_selected_editor_event(pages, event)
  let focus_editor = selected_editor_should_focus(next)
  let focus_search = selected_editor_should_focus_search(next)
  pending_editor_focus = focus_editor
  pending_editor_search_focus = focus_search
  pages = clear_selected_editor_focus(next)
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
      button #selected-button -> emit(navigate, trim(page))
        with
          label=title
          w=fill
          p=6.0
        row
          with
            w=fill
            gap=8.0
            align=center
          text icon
            with
              w=18.0
              size=15.0
              align-x=center
              @text-fg
          text title
            with
              w=fill
              size=13.0
              wrap=none
              @text-fg
        active bg=selected text=fg r=5.0
        hovered bg=selected
        pressed bg=hover
    if !selected
      button #button -> emit(navigate, trim(page))
        with
          label=title
          w=fill
          p=6.0
        row
          with
            w=fill
            gap=8.0
            align=center
          text icon
            with
              w=18.0
              size=15.0
              align-x=center
              @text-muted
          text title
            with
              w=fill
              size=13.0
              wrap=none
              @text-muted
        active bg=transparent text=muted r=5.0
        hovered bg=hover text=fg
        pressed bg=selected text=fg

component Sidebar(selected_page:str, pages:[Page], favorites:[Page], has_favorites:bool)
  emits
    toggle_sidebar
    open_search
    navigate(str)
    new_page
  box #root
    with
      w=244.0
      h=fill
      bg=sidebar
      border=border
      border-w=1.0
    col
      with
        w=fill
        h=fill
        p=8.0
        gap=2.0
      row #workspace
        with
          w=fill
          h=38.0
          p=5.0
          gap=8.0
          align=center
        box
          with
            w=24.0
            h=24.0
            align-x=center
            align-y=center
            bg=fg
            r=5.0
          text "E"
            with
              size=12.0
              @text-white
              @font-bold
        text "Eddy's Notion"
          with
            w=fill
            size=14.0
            @text-fg
            @font-bold
        button #collapse -> emit(toggle_sidebar)
          with
            label="Collapse sidebar"
            p=4.0
            style=text
          text "«" size=16.0 @text-muted
      button #search -> emit(open_search)
        with
          label="Search"
          w=fill
          p=7.0
        row
          with
            w=fill
            gap=10.0
            align=center
          text "⌕"
            with
              w=18.0
              size=17.0
              align-x=center
              @text-muted
          text "Search"
            with
              w=fill
              size=13.0
              @text-muted
        active bg=transparent text=muted r=5.0
        hovered bg=hover text=fg
        pressed bg=selected text=fg
      if has_favorites
        col #favorites w=fill gap=2.0
          text "Favorites" size=11.0 @text-faint
          for page in favorites
            PageItem #favorite(page.id)
              with
                icon=page.icon
                title=page.title
                page=page.id
                selected=(selected_page == page.id)
              forward
                navigate
      text "Private" size=11.0 @text-faint
      for page in pages
        PageItem #page(page.id)
          with
            icon=page.icon
            title=page.title
            page=page.id
            selected=(selected_page == page.id)
          forward
            navigate
      button #new-page -> emit(new_page)
        with
          label="New page"
          w=fill
          p=7.0
        row
          with
            w=fill
            gap=10.0
            align=center
          text "+"
            with
              w=18.0
              size=17.0
              align-x=center
              @text-muted
          text "New page" size=13.0 @text-muted
        active bg=transparent text=muted r=5.0
        hovered bg=hover text=fg
        pressed bg=selected text=fg
      space w=fill h=fill

component MiniSidebar()
  emits
    toggle_sidebar
    open_search
  box #root
    with
      w=44.0
      h=fill
      bg=sidebar
      border=border
      border-w=1.0
      align-x=center
    col
      with
        w=fill
        h=fill
        p=7.0
        align=center
      button #expand -> emit(toggle_sidebar)
        with
          label="Expand sidebar"
          style=text
          @notion::toolbar_action
        text "»" size=17.0 @text-muted
      button #search -> emit(open_search)
        with
          label="Search"
          style=text
          @notion::toolbar_action
        text "⌕" size=17.0 @text-muted
      space w=fill h=fill
      box
        with
          w=26.0
          h=26.0
          align-x=center
          align-y=center
          bg=fg
          r=5.0
        text "E"
          with
            size=12.0
            @text-white
            @font-bold

component Topbar(current_title:str, current_icon:str)
  emits
    open_share
  row #root
    with
      w=fill
      h=46.0
      px=12.0
      gap=5.0
      align=center
    text current_icon size=13.0 @text-muted
    text current_title
      with
        w=fill
        size=12.0
        @text-fg
    if provided(comments)
      slot comments?
    if provided(favorite_action)
      slot favorite_action?
    button #share label="Share" @notion::toolbar_action -> emit(open_share)
      text "Share" size=12.0 @text-primary
      active bg=transparent text=primary r=5.0
      hovered bg=blue_soft
      pressed bg=selected

component Document(bind title:str, state:BlockEditorState, icon:str="□") -> BlockEditorEvent
  emits
    title_changed(str)
  box #root
    with
      w=fill
      h=fill
      px=28.0
      align-x=center
    col
      with
        w=fill
        h=fill
        max-w=920.0
        pt=26.0
      text icon #icon size=42.0 @text-fg
      box w=fill px=30.0
        input "" #title <-> title
          with
            label="Page title"
            hint="Untitled"
            change=emit(title_changed, _)
            w=fill
            p=0.0
            text-size=36.0
            font=inter_bold
          active bg=transparent border=transparent value=fg placeholder=faint selection=primary border-w=0.0 r=0.0
          hovered bg=transparent border=transparent value=fg placeholder=faint border-w=0.0
          focused bg=transparent border=transparent value=fg placeholder=faint selection=primary border-w=0.0
      extern block_editor(state) #editor -> emit(_)

component SearchDialog(bind search_query:str, selected_page:str, results:[Page], has_query:bool, has_results:bool)
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
      row
        with
          w=fill
          gap=8.0
          align=center
        text "⌕" size=18.0 @text-muted
        input "" #query <-> search_query
          with
            label="Search pages"
            hint="Search Eddy's Notion"
            w=fill
            p=8.0
          active bg=transparent border=transparent value=fg placeholder=faint selection=primary border-w=0.0
          focused bg=transparent border=transparent value=fg placeholder=faint selection=primary border-w=0.0
        button #close -> emit(close_search)
          with
            label="Close search"
            style=text
            @notion::compact_action
          text "esc" size=10.0 @text-faint
      box
        with
          w=fill
          h=1.0
          bg=border
        text ""
      if !has_query
        text "RECENT"
          with
            size=10.0
            @text-faint
            @font-bold
      if has_query
        text "BEST MATCHES"
          with
            size=10.0
            @text-faint
            @font-bold
      for page in results
        PageItem #result(page.id)
          with
            icon=page.icon
            title=page.title
            page=page.id
            selected=(selected_page == page.id)
          forward
            navigate
      if !has_results
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
        text "Share this page"
          with
            w=fill
            size=17.0
            @text-fg
            @font-bold
        button #close -> emit(close_share)
          with
            label="Close share dialog"
            style=text
            @notion::compact_action
          text "×" size=18.0 @text-muted
      row w=fill gap=8.0
        input "" #email <-> invite_email
          with
            label="Email address"
            hint="Email or name"
            w=fill
            p=9.0
          active bg=surface border=border value=fg placeholder=faint selection=primary border-w=1.0 r=6.0
          focused border=primary border-w=1.0
        pick invite_access_options invite_access_choice #access -> emit(invite_access_changed, _)
          with
            hint="Can edit"
            w=112.0
            p=9.0
        button "Invite" #invite disabled=!invite_ready p=9.0 -> emit(send_invite)
          active bg=primary text=white r=6.0
          hovered bg=primary/85
          disabled bg=hover text=faint
      if !empty(invited_email)
        row #invited
          with
            w=fill
            gap=10.0
            align=center
          box
            with
              w=30.0
              h=30.0
              align-x=center
              align-y=center
              bg=fg
              r=15.0
            text "E"
              with
                size=11.0
                @text-white
                @font-bold
          col w=fill gap=1.0
            text invited_email size=12.0 @text-fg
            text "Invited" size=10.0 @text-muted
          text invited_access size=11.0 @text-muted
      box
        with
          w=fill
          h=1.0
          bg=border
        text ""
      row #general-access
        with
          w=fill
          gap=10.0
          align=center
        box
          with
            w=32.0
            h=32.0
            align-x=center
            align-y=center
            bg=hover
            r=16.0
          text "●" size=9.0 @text-muted
        col w=fill gap=2.0
          text "General access"
            with
              size=13.0
              @text-fg
              @font-bold
          text "Only people invited" size=11.0 @text-muted
        if link_copied
          button "Copied" #copied-link p=7.0 -> emit(copy_link, page_link)
            active bg=hover text=muted r=5.0
        if !link_copied
          button "Copy link" #copy-link p=7.0 -> emit(copy_link, page_link)
            active bg=surface text=fg border=border border-w=1.0 r=5.0
            hovered bg=hover

test page_item_component
  preset test
  timeout 5s
  viewport 300 90
  mount
    PageItem #item
      with
        icon="▦"
        title="Product roadmap"
        page="roadmap"
        selected=false
      events
        navigate -> navigate _
  target root = #item/root
  target button = #item/root/button
  expect root.width ~= 300.0
  expect button.kind == "button"
  expect text "Product roadmap" within root
  click button
  expect selected_page_id(pages) == "roadmap"

test sidebar_component
  preset test
  timeout 5s
  viewport 300 640
  mount
    Sidebar #sidebar
      with
        selected_page=selected_page_id(pages)
        pages=visible_pages(pages)
        favorites=favorite_pages(pages)
        has_favorites=has_favorite_pages(pages)
      events
        toggle_sidebar -> toggle_sidebar
        open_search -> open_search
        navigate -> navigate _
        new_page -> new_page
  target root = #sidebar/root
  target collapse = #sidebar/root/workspace/collapse
  target search = #sidebar/root/search
  target launch = #sidebar/root/page("launch")/root/button
  expect root.width ~= 244.0
  expect no text "Favorites" within root
  expect text "Private" within root
  expect no text "Updates" within root
  click launch
  expect selected_page_id(pages) == "launch"
  click search
  expect modal == ModalState.search
  click collapse
  expect !sidebar_open

test mini_sidebar_component
  preset test
  timeout 5s
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
  timeout 5s
  viewport 640 80
  mount
    Topbar current_title=selected_page_title(pages) current_icon=selected_page_icon(pages) #topbar
      events
        open_share -> open_share
      comments:
        button #comments -> comments_toggled
          with
            label="Open comments"
            style=text
            @notion::toolbar_action
          text "☵" size=15.0 @text-muted
      favorite_action:
        row
          if selected_page_favorite(pages)
            button #remove-favorite -> toggle_favorite
              with
                label="Remove from favorites"
                style=text
                @notion::toolbar_action
              text "★" size=16.0 @text-fg
          if !selected_page_favorite(pages)
            button #favorite -> toggle_favorite
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
  expect !selected_comments_open(pages)
  click comments
  expect selected_comments_open(pages)
  click favorite_button
  expect selected_page_favorite(pages)
  click share
  expect modal == ModalState.share

test document_component
  preset test
  timeout 5s
  viewport 960 650
  mount
    Document title<->editing_title state=pages.document #document -> editor_changed _
      events
        title_changed -> title_changed _
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
  expect selected_page_title(pages) == title.value

test search_dialog_component
  preset test
  timeout 5s
  viewport 600 520
  mount
    SearchDialog #search-dialog search_query<->search_query
      with
        selected_page=selected_page_id(pages)
        results=matching_pages(pages, search_query)
        has_query=has_search_query
        has_results=has_matching_pages(pages, search_query)
      events
        close_search -> close_search
        navigate -> navigate _
  target root = #search-dialog/root
  target query = #search-dialog/root/query
  target roadmap = #search-dialog/root/result("roadmap")/root/button
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
  expect selected_page_id(pages) == "roadmap"

test share_dialog_component
  preset test
  timeout 5s
  viewport 560 420
  mount
    ShareDialog #share-dialog invite_email<->invite_email
      with
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
  timeout 5s
  viewport 1280 800
  target app = #app
  target sidebar = #app/shell/sidebar/root
  target search = #app/shell/sidebar/root/search
  target launch = #app/shell/sidebar/root/page("launch")/root/button
  target page_title = #app/shell/current-page/document/root/title
  target comments = #app/shell/current-page/topbar/root/comments
  target favorite_button = #app/shell/current-page/topbar/root/favorite
  target share = #app/shell/current-page/topbar/root/share
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
  expect selected_page_favorite(pages)
  expect exists favorites
  click search
  expect exists search_dialog
  expect search_query == ""
  expect search_query_input.value == ""
  dispatch close_search
  click comments
  expect selected_comments_open(pages)
  expect text "Comments"
  click share
  expect exists share_dialog
  dispatch close_share
  click launch
  expect selected_page_id(pages) == "launch"
  expect text "Styles ▾"
  click new_page
  expect selected_page_id(pages) == "untitled"
  expect selected_page_title(pages) == "Untitled"
  expect text "Styles ▾"

test minimum_window_layout
  preset test
  timeout 5s
  viewport 860 600
  target app = #app
  target shell = #app/shell
  target sidebar = #app/shell/sidebar/root
  target mini_sidebar = #app/shell/mini-sidebar/root
  target page = #app/shell/current-page
  target topbar = #app/shell/current-page/topbar/root
  target document = #app/shell/current-page/document/root
  target title = #app/shell/current-page/document/root/title
  target editor = #app/shell/current-page/document/root/editor
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
          box #app
            with
              w=fill
              h=fill
              bg=bg
            row #shell w=fill h=fill
              if sidebar_open
                Sidebar #sidebar
                  with
                    selected_page=selected_page_id(pages)
                    pages=visible_pages(pages)
                    favorites=favorite_pages(pages)
                    has_favorites=has_favorite_pages(pages)
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
              col #current-page w=fill h=fill
                Topbar #topbar
                  with
                    current_title=selected_page_title(pages)
                    current_icon=selected_page_icon(pages)
                  events
                    open_share -> open_share
                  comments:
                    button #comments -> comments_toggled
                      with
                        label="Open comments"
                        style=text
                        @notion::toolbar_action
                      text "☵" size=15.0 @text-muted
                  favorite_action:
                    row
                      if selected_page_favorite(pages)
                        button #remove-favorite -> toggle_favorite
                          with
                            label="Remove from favorites"
                            style=text
                            @notion::toolbar_action
                          text "★" size=16.0 @text-fg
                      if !selected_page_favorite(pages)
                        button #favorite -> toggle_favorite
                          with
                            label="Add to favorites"
                            style=text
                            @notion::toolbar_action
                          text "☆" size=16.0 @text-muted
                Document #document title<->editing_title -> editor_changed _
                  with
                    state=pages.document
                    icon=selected_page_icon(pages)
                  events
                    title_changed -> title_changed _
        layer
          ShareDialog #share-dialog invite_email<->invite_email
            with
              invite_access_options=invite_access_options
              invite_access_choice=invite_access_choice
              invited_email=invited_email
              invited_access=invited_access
              link_copied=link_copied
              page_link=page_link(selected_page_id(pages))
              invite_ready=invite_ready
            events
              close_share -> close_share
              invite_access_changed -> invite_access_changed _
              send_invite -> send_invite
              copy_link -> copy_link _
    layer
      SearchDialog #search-dialog search_query<->search_query
        with
          selected_page=selected_page_id(pages)
          results=matching_pages(pages, search_query)
          has_query=has_search_query
          has_results=has_matching_pages(pages, search_query)
        events
          close_search -> close_search
          navigate -> navigate _

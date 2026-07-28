use crate::editor::{
    BlockEditorEvent, BlockEditorState, block_editor_apply, block_editor_clear_focus,
    block_editor_comments_open, block_editor_should_focus, block_editor_should_focus_search,
    block_editor_state, block_editor_toggle_comments,
};

#[derive(Clone, Debug)]
pub struct Page {
    pub id: String,
    pub icon: String,
    pub title: String,
    pub favorite: bool,
}

#[derive(Clone, Debug)]
struct PageEntry {
    page: Page,
    document: BlockEditorState,
}

#[derive(Clone, Debug)]
pub struct PageStore {
    pub document: BlockEditorState,
    selected: usize,
    entries: Vec<PageEntry>,
}

pub fn default_pages() -> PageStore {
    let entries = [
        ("home", "◆", "Product strategy"),
        ("roadmap", "▦", "Product roadmap"),
        ("launch", "✓", "Launch plan"),
        ("meeting", "▤", "Weekly meeting"),
        ("untitled", "□", "Untitled"),
    ]
    .into_iter()
    .map(|(id, icon, title)| PageEntry {
        page: Page {
            id: id.into(),
            icon: icon.into(),
            title: title.into(),
            favorite: false,
        },
        document: block_editor_state(id.into()),
    })
    .collect::<Vec<_>>();
    PageStore {
        document: entries[0].document.clone(),
        selected: 0,
        entries,
    }
}

pub fn selected_page_id(store: PageStore) -> String {
    store.entries[store.selected].page.id.clone()
}

pub fn selected_page_title(store: PageStore) -> String {
    store.entries[store.selected].page.title.clone()
}

pub fn selected_page_icon(store: PageStore) -> String {
    store.entries[store.selected].page.icon.clone()
}

pub fn selected_page_favorite(store: PageStore) -> bool {
    store.entries[store.selected].page.favorite
}

pub fn visible_pages(store: PageStore) -> Vec<Page> {
    store.entries.into_iter().map(|entry| entry.page).collect()
}

pub fn favorite_pages(store: PageStore) -> Vec<Page> {
    store
        .entries
        .into_iter()
        .filter(|entry| entry.page.favorite)
        .map(|entry| entry.page)
        .collect()
}

pub fn has_favorite_pages(store: PageStore) -> bool {
    store.entries.iter().any(|entry| entry.page.favorite)
}

pub fn matching_pages(store: PageStore, query: String) -> Vec<Page> {
    let query = query.trim().to_lowercase();
    store
        .entries
        .into_iter()
        .filter(|entry| entry.page.title.to_lowercase().contains(&query))
        .map(|entry| entry.page)
        .collect()
}

pub fn has_matching_pages(store: PageStore, query: String) -> bool {
    let query = query.trim().to_lowercase();
    store
        .entries
        .iter()
        .any(|entry| entry.page.title.to_lowercase().contains(&query))
}

pub fn select_page(mut store: PageStore, id: String) -> PageStore {
    if let Some(selected) = store.entries.iter().position(|entry| entry.page.id == id) {
        store.entries[store.selected].document = store.document;
        store.selected = selected;
        store.document = store.entries[selected].document.clone();
    }
    store
}

pub fn reset_new_page(mut store: PageStore) -> PageStore {
    if let Some(selected) = store
        .entries
        .iter()
        .position(|entry| entry.page.id == "untitled")
    {
        store.selected = selected;
        store.entries[selected].page.title = "Untitled".into();
        store.entries[selected].document = block_editor_state("untitled".into());
        store.document = store.entries[selected].document.clone();
    }
    store
}

pub fn rename_selected_page(mut store: PageStore, title: String) -> PageStore {
    store.entries[store.selected].page.title = title;
    store
}

pub fn toggle_selected_favorite(mut store: PageStore) -> PageStore {
    let page = &mut store.entries[store.selected].page;
    page.favorite = !page.favorite;
    store
}

pub fn toggle_selected_comments(mut store: PageStore) -> PageStore {
    store.document = block_editor_toggle_comments(store.document);
    store.entries[store.selected].document = store.document.clone();
    store
}

pub fn apply_selected_editor_event(mut store: PageStore, event: BlockEditorEvent) -> PageStore {
    store.document = block_editor_apply(store.document, event);
    store.entries[store.selected].document = store.document.clone();
    store
}

pub fn selected_editor_should_focus(store: PageStore) -> bool {
    block_editor_should_focus(store.document)
}

pub fn selected_editor_should_focus_search(store: PageStore) -> bool {
    block_editor_should_focus_search(store.document)
}

pub fn clear_selected_editor_focus(mut store: PageStore) -> PageStore {
    store.document = block_editor_clear_focus(store.document);
    store.entries[store.selected].document = store.document.clone();
    store
}

pub fn selected_comments_open(store: PageStore) -> bool {
    block_editor_comments_open(store.document)
}

impl PageStore {
    #[cfg(test)]
    pub fn selected_id(&self) -> &str {
        &self.entries[self.selected].page.id
    }

    #[cfg(test)]
    pub fn document(&self, id: &str) -> &BlockEditorState {
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.page.id == id)
            .expect("known page");
        if entry.page.id == self.entries[self.selected].page.id {
            &self.document
        } else {
            &entry.document
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_retains_each_pages_document() {
        let store = select_page(default_pages(), "launch".into());
        assert_eq!(store.selected_id(), "launch");
        assert!(
            store
                .document("launch")
                .markdown()
                .contains("Finalize announcement")
        );
        assert!(store.document("home").thread_count() > 0);
        assert_eq!(
            matching_pages(store.clone(), "ROAD".into())[0].id,
            "roadmap"
        );
        assert!(!has_matching_pages(store, "missing".into()));
    }
}

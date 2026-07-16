use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sort<Column> {
    pub column: Column,
    pub direction: SortDirection,
}

/// Headless sorting, filtering-copy, and pagination state for a composed table.
///
/// Row filtering and ordering stay caller-owned because each data set has a
/// different schema. Render the result with `table`, `input`, `checkbox`, and
/// `pagination` after applying this state to your rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataTableState<Column> {
    pub sort: Option<Sort<Column>>,
    pub query: String,
    pub page: usize,
    pub page_size: usize,
}

impl<Column> DataTableState<Column>
where
    Column: Clone + Eq,
{
    pub fn new(page_size: usize) -> Self {
        Self {
            sort: None,
            query: String::new(),
            page: 0,
            page_size: page_size.max(1),
        }
    }

    /// Cycles a column through ascending, descending, and unsorted.
    pub fn toggle_sort(&mut self, column: Column) {
        self.sort = match self.sort.as_ref() {
            Some(sort) if sort.column == column && sort.direction == SortDirection::Ascending => {
                Some(Sort {
                    column,
                    direction: SortDirection::Descending,
                })
            }
            Some(sort) if sort.column == column && sort.direction == SortDirection::Descending => {
                None
            }
            _ => Some(Sort {
                column,
                direction: SortDirection::Ascending,
            }),
        };
        self.page = 0;
    }

    pub fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into();
        self.page = 0;
    }

    pub fn set_page(&mut self, page: usize, filtered_rows: usize) {
        self.page = page.min(self.page_count(filtered_rows).saturating_sub(1));
    }

    pub fn page_count(&self, filtered_rows: usize) -> usize {
        filtered_rows.div_ceil(self.page_size.max(1))
    }

    pub fn visible_range(&self, filtered_rows: usize) -> Range<usize> {
        let page_size = self.page_size.max(1);
        let start = self.page.saturating_mul(page_size).min(filtered_rows);
        start..start.saturating_add(page_size).min(filtered_rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_cycle_and_filter_reset_pagination() {
        let mut state = DataTableState::new(10);
        state.page = 2;
        state.toggle_sort("name");
        assert_eq!(
            state.sort.as_ref().map(|sort| sort.direction),
            Some(SortDirection::Ascending)
        );
        assert_eq!(state.page, 0);
        state.toggle_sort("name");
        assert_eq!(
            state.sort.as_ref().map(|sort| sort.direction),
            Some(SortDirection::Descending)
        );
        state.toggle_sort("name");
        assert_eq!(state.sort, None);

        state.page = 3;
        state.set_query("duck");
        assert_eq!(state.page, 0);
    }

    #[test]
    fn pages_are_bounded_for_empty_and_partial_results() {
        let mut state = DataTableState::<()>::new(10);
        assert_eq!(state.page_count(0), 0);
        assert_eq!(state.page_count(21), 3);
        state.set_page(99, 21);
        assert_eq!(state.page, 2);
        assert_eq!(state.visible_range(21), 20..21);
        state.set_page(5, 0);
        assert_eq!(state.page, 0);
        assert_eq!(state.visible_range(0), 0..0);
    }
}

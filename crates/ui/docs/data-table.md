# Data table

Like shadcn/ui, ducktape-ui treats a data table as a composition recipe because every data set has different sorting, filtering, selection, and server-side pagination rules.

Enable the headless state plus its UI building blocks:

```toml
ducktape-ui = { git = "https://github.com/byeongsu-hong/ducktape-ui", features = ["data-table"] }
```

Own `DataTableState<Column>` in your application state. Apply `query` and `sort` to your domain rows, slice the result with `visible_range`, then render it with `table`. Compose `input`, `checkbox`, and `pagination` for the controls.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Column {
    Email,
    Amount,
}

let mut state = ducktape_ui::ui::data_table::DataTableState::new(10);
state.set_query("example.com");
state.toggle_sort(Column::Email);

let visible = state.visible_range(filtered_rows.len());
let page_rows = &filtered_rows[visible];
```

This module deliberately does not compare or filter arbitrary row types, prescribe row IDs, or store row selection. Those rules belong to the application, while the toolkit owns the reusable state transitions and visual primitives.

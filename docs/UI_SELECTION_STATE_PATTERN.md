# UI Selection State Pattern

Many relm4 UI components become inconsistent after the user selects a new item from a list because the old component state is only partially cleared. The fix is to treat a selection change as a full context switch, not as a partial field update.

## Recommended flow

```text
selection changed
→ clear old dependent state
→ enter loading/empty state
→ fetch/build new state
→ replace entire view state
```

## Core idea

Split component state into:

1. **Selection identity**
   - The current selected entity ID or key
2. **Derived detail state**
   - Empty, loading, loaded, or error state for the selected entity
3. **Child component state**
   - Explicitly reset when parent selection changes

This avoids keeping stale data from the previously selected item.

## Suggested structure

```rust
struct MyComponent {
    selected_id: Option<i64>,
    state: DetailState,
}

enum DetailState {
    Empty,
    Loading,
    Loaded(FormState),
    Error(String),
}
```

Keep data tied to the current selection inside `FormState`:

```rust
struct FormState {
    name: String,
    notes: String,
    selected_file_set_id: Option<i64>,
    validation_errors: Vec<String>,
    items: Vec<ListItem>,
}
```

Anything that should not survive a selection change belongs in this loaded state and must be replaced or cleared when the selection changes.

## Recommended methods

```rust
impl MyComponent {
    fn clear_view_state(&mut self) {
        self.state = DetailState::Empty;
        // clear child selections, validation, temp flags, etc.
    }

    fn start_loading(&mut self) {
        self.state = DetailState::Loading;
    }

    fn apply_loaded_data(&mut self, data: LoadedData) {
        self.state = DetailState::Loaded(FormState::from(data));
    }
}
```

The important part is replacing a whole derived state object instead of patching many fields one by one.

## relm4 message flow

```rust
Msg::ItemSelected(id) => {
    self.selected_id = Some(id);
    self.clear_view_state();
    self.start_loading();

    sender.oneshot_command(async move {
        CmdMsg::Loaded(service.load(id).await)
    });
}

CmdMsg::Loaded(Ok(data)) => {
    self.apply_loaded_data(data);
}

CmdMsg::Loaded(Err(e)) => {
    self.state = DetailState::Error(e.to_string());
}
```

## Why this helps

A common bug looks like this:

```rust
self.name = loaded.name;
self.notes = loaded.notes;
```

That often leaves old state behind in fields such as:

- selected child item
- validation errors
- temporary flags
- cached lists
- nested component state

Replacing one complete loaded state object is safer:

```rust
self.state = DetailState::Loaded(FormState::from(data));
```

## Parent-child components

When parent selection changes, child components should be reset explicitly:

```rust
fn clear_view_state(&mut self) {
    self.state = DetailState::Empty;
    self.file_list.emit(FileListMsg::Clear);
    self.details.emit(DetailsMsg::Clear);
}
```

Do not assume parent field updates automatically clear child-internal state.

## Reusable conventions

If multiple components follow this pattern, use consistent names:

- messages:
  - `SelectX { id }`
  - `Clear`
  - `LoadStarted`
  - `Loaded(Result<T, Error>)`
- methods:
  - `clear_view_state()`
  - `apply_loaded_data(...)`

## Practical rule

When selection changes, ask:

> If this field belonged to the previous selection, should it survive?

If the answer is no, it belongs in the loaded view state and must be replaced or cleared every time.

This pattern is especially useful for forms, detail panes, dialogs with list selection, and nested UI flows.

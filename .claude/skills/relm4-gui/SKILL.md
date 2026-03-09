---
name: relm4-gui
description: This skill should be used when implementing or modifying GTK4 UI components, dialogs, forms, or list views in the relm4-ui crate. Use when the user asks to "add a dialog", "create a component", "implement a form", "add a list view", "fix a UI bug", "create a widget", or discusses relm4/GTK4 implementation work.
version: 1.0.0
---

# Relm4 GUI Skill

You are implementing GTK4 UI in the **Emulation File Manager** using relm4 0.9.1.

## Primary Reference

**Always read `docs/patterns/gui.md` first.** It contains the canonical patterns for this project:
- Component skeleton (Input/Output/CommandOutput/Init types)
- Entry field update loop prevention (`update_with_view` pattern)
- Window close handling (`hide()` not `close()`)
- Async operations via `sender.oneshot_command`
- Component controllers with `OnceCell`
- Show/Hide dialog pattern
- `Arc<Mutex<Flags>>` shutdown coordination
- Error display with `show_error_dialog`

Do not deviate from those patterns. Do not duplicate them here.

## Vendor Examples

When you need a working example for a pattern not covered in `docs/patterns/gui.md`, look in `vendor/relm4/examples/`. Key examples:

| Need | Example file |
|---|---|
| Typed list view with filter/sort | `typed_list_view.rs` |
| Async typed list view | `typed_list_view_async.rs` |
| Dynamic widget lists (factory) | `factory.rs` |
| Factory with HashMap | `factory_hash_map.rs` |
| Multi-window | `multi_window.rs` |
| Async component | `simple_async.rs` |
| Worker (background task) | `worker.rs` |
| Tracker-based fine-grained updates | `tracker.rs` |
| Popover menus | `popover.rs` |
| Drag and drop | `drag_and_drop.rs` |

## Component Type Selection

| Use | When |
|---|---|
| `SimpleComponent` | No async commands needed, simple state |
| `Component` | Needs `CommandOutput` for async results |
| `FactoryComponent` | Each item in a dynamic list is itself a widget |
| `AsyncComponent` | Component `update()` itself must be async |

## TypedListView Pattern

For typed, filterable, sortable lists (preferred over raw `gtk::ListView`):

```rust
use relm4::typed_view::list::{RelmListItem, TypedListView};

// 1. Define the item type
#[derive(Debug)]
struct MyItem {
    id: i64,
    name: String,
}

// 2. Implement RelmListItem — defines how to render one row
impl RelmListItem for MyItem {
    type Root = gtk::Box;
    type Widgets = MyItemWidgets;

    fn setup(_item: &gtk::ListItem) -> (gtk::Box, MyItemWidgets) {
        relm4::view! {
            root = gtk::Box {
                set_spacing: 8,
                #[name = "name_label"]
                gtk::Label { set_hexpand: true, set_halign: gtk::Align::Start },
            }
        }
        (root, MyItemWidgets { name_label })
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        widgets.name_label.set_label(&self.name);
    }
}

struct MyItemWidgets { name_label: gtk::Label }

// 3. In the component model:
struct MyModel {
    list_view_wrapper: TypedListView<MyItem, gtk::SingleSelection>,
}

// 4. Initialize:
let mut list_view_wrapper: TypedListView<MyItem, gtk::SingleSelection> =
    TypedListView::new();  // or TypedListView::with_sorting()

// Optional: add filter
list_view_wrapper.add_filter(|item| item.name.contains("search"));

// 5. In view! macro — use #[local_ref]:
let my_list_view = &model.list_view_wrapper.view;
// Then in view!:
// #[local_ref]
// my_list_view -> gtk::ListView { set_vexpand: true }

// 6. Populate:
for item in items { self.list_view_wrapper.append(item); }
self.list_view_wrapper.clear();

// 7. Get selected:
if let Some(item) = self.list_view_wrapper.selection_model.selected_item() {
    // downcast and use
}
```

## FactoryVecDeque Pattern

For a list where each row is an interactive sub-component:

```rust
use relm4::factory::{DynamicIndex, FactoryComponent, FactorySender, FactoryVecDeque};

// 1. Define item component
#[relm4::factory]
impl FactoryComponent for MyRowModel {
    type Init = MyData;
    type Input = MyRowMsg;
    type Output = MyRowOutputMsg;  // bubbled up to parent
    type CommandOutput = ();
    type ParentWidget = gtk::Box;  // or gtk::ListBox

    view! { /* row widget tree */ }

    fn init_model(data: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self { /* ... */ }
    }
    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) { /* ... */ }
}

// 2. In parent model:
struct ParentModel {
    rows: FactoryVecDeque<MyRowModel>,
}

// 3. Initialize in parent's init():
let rows = FactoryVecDeque::builder()
    .launch(parent_widget_ref)
    .forward(sender.input_sender(), |msg| match msg {
        MyRowOutputMsg::Deleted(idx) => ParentMsg::DeleteRow(idx),
    });

// 4. Mutate via guard (commits changes on drop):
let mut guard = self.rows.guard();
guard.push_back(data);
guard.remove(index.current_index());
// guard drops here → UI updates
```

## #[local_ref] in view! Macro

Use `#[local_ref]` to embed a pre-built widget (e.g., from `TypedListView` or `FactoryVecDeque`) into the view tree:

```rust
// Before view_output!() or view!{} call:
let my_list = &model.list_view_wrapper.view;
let factory_box = model.rows.widget();

// Inside view! {}:
gtk::ScrolledWindow {
    set_vexpand: true,
    #[local_ref]
    my_list -> gtk::ListView {}
}
```

## Checklist Before Writing GUI Code

1. Read `docs/patterns/gui.md` for the component skeleton and critical gotchas
2. Choose component type: `SimpleComponent` / `Component` / `FactoryComponent`
3. For lists: use `TypedListView` (static items) or `FactoryVecDeque` (interactive rows)
4. Entry fields: use `update_with_view`, never `#[watch]` + `connect_changed` together
5. Async DB calls: use `sender.oneshot_command`, never `.await` in `update()`. Results go in a dedicated `CommandOutput` enum and are handled in `update_cmd` — never reuse `Input` as `CommandOutput` and never route `update_cmd` through `update()`
6. Dialogs: `root.hide()` / `root.present()`, never `root.close()`
7. UI stays thin — business logic belongs in `service` crate
8. GUI never imports from `database` crate directly

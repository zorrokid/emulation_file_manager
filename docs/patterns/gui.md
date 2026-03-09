# GUI Patterns

Reference guide for implementing GTK4 UI in the **Emulation File Manager** using relm4 0.9.1.

## Technology Stack

- **relm4 0.9.1**: Reactive GTK4 framework with elm-like architecture
- **GTK4**: Modern GNOME toolkit
- **async-std 1.13.2**: Async runtime for background operations
- **tracker 0.2.2**: Change tracking for model updates
- **tracing**: Structured logging

## Project Structure

```
relm4-ui/src/
├── main.rs              # Entry point
├── app.rs               # Root AppModel component
├── components/          # Reusable sub-components
├── *_form.rs            # Form dialogs (system_form, emulator_form, etc.)
├── *_selector.rs        # Selection components
├── *_list.rs            # List view components
├── status_bar.rs        # Status display
├── style.css            # GTK CSS styling
└── utils/               # UI utilities (dialog_utils, etc.)
```

## Relm4 Component Pattern

### Data Flow

UI components never touch the database directly. Always go through the service layer:

```
User interaction → AppMsg → AppModel → AppServices → RepositoryManager → SQLite
```

Components hold a `Arc<AppServices>` (or a domain-specific service), not `Arc<RepositoryManager>`.

### Basic Component Structure

```rust
#[derive(Debug)]
pub struct MyComponentModel {
    // Model state
    pub field1: String,
    pub field2: i64,
    // Dependencies — service layer, not repositories
    pub service: Arc<AppServices>,
}

#[derive(Debug)]
pub enum MyComponentMsg {
    // Input messages
    FieldChanged(String),
    Submit,
    Show,
    Hide,
}

#[derive(Debug)]
pub enum MyComponentOutputMsg {
    // Output messages to parent
    ItemCreated(Model),
    ItemUpdated(Model),
}

#[derive(Debug)]
pub enum MyComponentCommandMsg {
    // Async command results
    OperationCompleted(Result<T, Error>),
}

#[derive(Debug)]
pub struct MyComponentInit {
    // Initialization parameters
    pub service: Arc<AppServices>,
}

#[relm4::component(pub)]
impl Component for MyComponentModel {
    type Input = MyComponentMsg;
    type Output = MyComponentOutputMsg;
    type CommandOutput = MyComponentCommandMsg;
    type Init = MyComponentInit;

    view! {
        gtk::Window {
            // Widget tree
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Initialize model and widgets
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        // Handle messages
    }
}
```

## Critical Patterns & Gotchas

### 1. Entry Field Update Loop Prevention

**Problem**: Using `#[watch]` + `connect_changed` causes infinite update loops.

**WRONG:**
```rust
gtk::Entry {
    #[watch]  // Triggers on model change
    set_text: &model.name,
    connect_changed => /* updates model */ // Causes loop!
}
```

**CORRECT Solution 1: Manual Update with update_with_view**
```rust
// In view!:
#[name = "name_entry"]
gtk::Entry {
    set_text: &model.name,  // No #[watch]!
    connect_changed[sender] => move |entry| {
        sender.input(Msg::NameChanged(entry.buffer().text().into()));
    },
}

// Implement update_with_view instead of update:
fn update_with_view(
    &mut self,
    msg: Self::Input,
    widgets: &Self::Widgets,
    sender: ComponentSender<Self>,
) {
    match msg {
        Msg::NameChanged(name) => {
            self.name = name;
        }
        Msg::Show { data } => {
            self.name = data.name;
            widgets.name_entry.set_text(&self.name);  // Manual widget update
        }
        Msg::Hide => {
            root.hide();
        }
        Msg::ShowError(msg) => {
            show_error_dialog(&msg, root);
        }
    }
    self.update_view(widgets, sender);  // REQUIRED — updates all #[watch] attrs; omitting causes stale UI
}
```

**CORRECT Solution 2: Block Signal (if cursor jump not an issue)**
```rust
gtk::Entry {
    #[watch]
    #[block_signal(name_changed)]
    set_text: &model.name,
    connect_changed => Msg::NameChanged @name_changed,
}
```

**Why we prefer manual updates:** `set_text` causes cursor jump to beginning when typing.

### 2. Window Close Handling

**CORRECT Pattern:**
```rust
gtk::Window {
    connect_close_request[sender] => move |_| {
        sender.input(Msg::Hide);
        glib::Propagation::Stop  // or Proceed
    },
}

// In message handler:
Msg::Hide => {
    root.hide();  // NOT root.close()!
}
```

**Why:** Allows reusing dialog windows instead of recreating them.

### 3. Async Operations (Commands)

For service calls or long-running operations:

```rust
fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
    match msg {
        Msg::Submit => {
            let service = self.services.clone();
            let data = self.data.clone();

            sender.oneshot_command(async move {
                let result = service.create_system(&data.name).await;
                CommandMsg::SubmitCompleted(result)
            });
        }
    }
}

fn update_cmd(&mut self, msg: Self::CommandOutput, sender: ComponentSender<Self>) {
    match msg {
        CommandMsg::SubmitCompleted(Ok(item)) => {
            // Success - notify parent, then hide via a message (root not available here)
            sender.output(OutputMsg::ItemCreated(item)).ok();
            sender.input(Msg::Hide);
        }
        CommandMsg::SubmitCompleted(Err(e)) => {
            // Errors also require a message round-trip to access widgets
            sender.input(Msg::ShowError(e.to_string()));
        }
    }
}
```

**Key Points:**
- Always call services, never repositories directly from GUI
- Clone `Arc`'d data before moving into async block
- Use `oneshot_command` for single async operations
- `update_cmd` has no access to `root` or `widgets` — send a message back to self and handle UI changes in `update_with_view`
- Use `show_error_dialog` (from `utils::dialog_utils`) in the `update_with_view` handler

### 4. Component Controllers

For child components:

```rust
struct AppModel {
    settings_form: OnceCell<Controller<SettingsForm>>,
}

// Initialize child:
let settings_form = SettingsForm::builder()
    .transient_for(&root)
    .launch(SettingsFormInit {
        services: app_services.clone()
    })
    .forward(sender.input_sender(), |msg| match msg {
        SettingsFormOutputMsg::Updated => AppMsg::UpdateSettings,
    });

model.settings_form.set(settings_form).ok();

// Show child later:
Msg::OpenSettings => {
    if let Some(form) = self.settings_form.get() {
        form.emit(SettingsFormMsg::Show);
    }
}
```

**Pattern:** Use `OnceCell` for lazy initialization, `forward` for output message mapping.

## Shutdown & Async Handling

### Shutdown with Running Operations

The app uses `Arc<Mutex<Flags>>` pattern for shutdown coordination:

```rust
struct Flags {
    app_closing: bool,           // Shutdown in progress
    cloud_sync_in_progress: bool, // Background operation running
    close_requested: bool,        // User requested close
}

struct AppModel {
    flags: Arc<Mutex<Flags>>,
}
```

**Critical Rules:**
1. **Short lock scope**: Read flags, release lock, then do UI work
2. **Re-check in callbacks**: State may change while dialog shows
3. **Set close_requested early**: Prevents race conditions with completion dialogs
4. **Check app_closing before starting operations**: Prevent starting during shutdown

**Example:**
```rust
Msg::CloseRequested => {
    let should_show_dialog = {
        let mut flags = self.flags.lock().unwrap();
        if flags.app_closing {
            return;  // Already closing
        }
        flags.close_requested = true;  // Set ASAP
        flags.cloud_sync_in_progress
    };  // Lock released
    
    if should_show_dialog {
        show_confirmation_dialog();
    } else {
        let mut flags = self.flags.lock().unwrap();
        flags.app_closing = true;
        drop(flags);
        root.close();
    }
}
```

See `relm4-ui/APPLICATION_SHUTDOWN_DESIGN.md` for comprehensive shutdown patterns.

## Common UI Patterns

### Show/Hide Pattern for Dialogs

```rust
Msg::Show { data } => {
    self.load_data(data);
    widgets.name_entry.set_text(&self.name);
    widgets.name_entry.grab_focus();
    root.show();
}

Msg::Hide => {
    self.clear_form();
    root.hide();
}
```

### Form Validation

```rust
gtk::Button {
    set_label: "Submit",
    #[watch]
    set_sensitive: !model.name.is_empty() && model.is_valid(),
    connect_clicked => Msg::Submit,
}
```

### Error Display

```rust
use crate::utils::dialog_utils::{show_error_dialog, show_info_dialog};

// In update_with_view (root and widgets are available there):
show_error_dialog(&format!("Failed to save: {}", error), root);
show_info_dialog("Item saved successfully", root);
```

### List with Selection

Use `TypedListView` — not raw `gtk::ListView`. See the `relm4-gui` skill for the full pattern. `TypedListView` supports filtering, sorting, and typed access and is the project standard.

## View Macro Syntax

### Common Attributes

- `#[watch]`: Re-evaluate on model change (use sparingly!)
- `#[name = "widget_name"]`: Named widget access in `update_with_view`
- `#[wrap(Some)]`: Wrap expression in Option
- `#[block_signal(handler_name)]`: Prevent signal during update
- `#[local_ref]`: Embed a pre-built widget (e.g. from `TypedListView` or `FactoryVecDeque`) into the view tree

### Widget Setup

```rust
gtk::Box {
    set_orientation: gtk::Orientation::Vertical,
    set_spacing: 10,
    set_margin_all: 20,
    
    gtk::Label {
        set_label: "Name",
        set_halign: gtk::Align::Start,
    },
    
    gtk::Entry {
        set_placeholder_text: Some("Enter name"),
        connect_changed[sender] => move |entry| {
            sender.input(Msg::Changed(entry.text().to_string()));
        },
    },
}
```

## CSS Styling

Load custom styles in `style.rs`:

```rust
pub fn load_app_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(include_str!("style.css"));
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().unwrap(),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
```

Use CSS classes:

```rust
gtk::Button {
    add_css_class: "destructive-action",
    set_label: "Delete",
}
```

## Common Mistakes to Avoid

- ❌ Using `#[watch]` with `connect_changed` on entries → infinite loop
- ❌ Calling `root.close()` in Hide handler → can't reuse window
- ❌ Holding `Mutex` lock during UI operations → deadlock risk
- ❌ Not calling `self.update_view()` at end of `update_with_view` → stale UI
- ❌ Forgetting to `drop()` lock before blocking operations
- ❌ Starting async operations after `app_closing` flag set
- ❌ Not checking flags again in dialog callbacks → race conditions
- ❌ Implementing business logic in UI layer → violates architecture

## Decision Checklist for GUI Changes

When implementing UI features:

1. **Component type**: New component or extend existing?
2. **Message flow**: What Input/Output/Command messages are needed?
3. **Dependencies**: What `AppServices` methods does it need?
4. **Async operations**: Do I need `oneshot_command`?
5. **Parent communication**: How does it notify parent? (Output messages)
6. **Entry fields**: Am I avoiding the update loop? (Use `update_with_view`)
7. **Window management**: Using `hide()` not `close()` for reusable dialogs?
8. **Error handling**: Using `show_error_dialog` from utils?
9. **Shutdown**: Will this work correctly during app shutdown?

Always ensure UI code stays thin—business logic belongs in the service layer!

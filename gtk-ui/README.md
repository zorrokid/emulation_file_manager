# imp.rs vs mod.rs

## **imp.rs (Implementation struct)**
- Contains the **private/internal implementation** of your widget.
- Methods here are usually:
  - Internal helpers
  - Template child access
  - Signal handlers
  - Trait implementations (`ObjectImpl`, `WidgetImpl`, etc.)
- **Not visible** to users of your widget (unless you expose them via `mod.rs`).

**Put methods in `imp.rs` when:**
- They are only used internally by your widget.
- They are helpers for trait implementations or template setup.
- They should not be part of the public API.

---

## **mod.rs (Public wrapper struct)**
- Contains the **public API** for your widget.
- Methods here are:
  - Constructors (`new`)
  - Public getters/setters
  - Any API you want users of your widget to call
  - Methods that call into `imp.rs` for implementation details

**Put methods in `mod.rs` when:**
- They are part of your widget’s public API.
- You want other code to call them.
- They wrap or expose internal logic from `imp.rs`.


# Implementing a list or grid view

### 1. **Create the UI Template**

- Design your widget’s UI in a `.ui` file using Glade or manually.
- For a grid or list view, include a `GtkGridView` or `GtkListView` with a unique `id`.

**Example (`my_component.ui`):**
```xml
<template class="MyComponent" parent="GtkBox">
  <property name="orientation">vertical</property>
  <child>
    <object class="GtkGridView" id="my_grid"/>
  </child>
</template>
```

---

### 2. **Create the Implementation Struct (`imp.rs`)**

- Define your struct with `#[derive(CompositeTemplate)]`.
- Add `TemplateChild` fields for each widget you want to access.
- Add a field for your model (e.g., `OnceCell<gtk::NoSelection>`).

**Example:**
```rust
#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/org/example/my_component.ui")]
pub struct MyComponent {
    #[template_child(id = "my_grid")]
    pub my_grid: TemplateChild<gtk::GridView>,
    pub grid_model: std::cell::OnceCell<gtk::NoSelection>,
}
```

---

### 3. **Implement `ObjectSubclass` for Your Struct**

- Set the correct type aliases.
- In `instance_init`, only call `obj.init_template();`.

**Example:**
```rust
#[glib::object_subclass]
impl ObjectSubclass for MyComponent {
    const NAME: &'static str = "MyComponent";
    type Type = super::MyComponent;
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        Self::bind_template(klass);
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}
```

---

### 4. **Create the Public Wrapper (`mod.rs`)**

- Use `glib::wrapper!` to define the public struct.
- Implement `Default` and a constructor.

**Example:**
```rust
glib::wrapper! {
    pub struct MyComponent(ObjectSubclass<imp::MyComponent>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl Default for MyComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl MyComponent {
    pub fn new() -> Self {
        Object::builder().build()
    }
}
```

---

### 5. **Set Up the Model in a Public Method**

- Create a method (e.g., `set_items`) to set up the model and populate it with your data.
- Use `gio::ListStore::new::<YourObjectType>()` for the model.
- Use `gtk::NoSelection` or `gtk::SingleSelection` as needed.

**Example:**
```rust
pub fn set_items(&self, items: Vec<YourObjectType>) {
    let imp = self.imp();
    let list_store = gio::ListStore::new::<YourObjectType>();
    for item in items {
        list_store.append(&item);
    }
    let selection_model = gtk::NoSelection::new(Some(list_store));
    imp.my_grid.set_model(Some(&selection_model));
    imp.grid_model.set(selection_model).ok();
}
```

---

### 6. **Import Required Traits**

- Always import necessary traits for upcasting and GTK operations:
```rust
use glib::prelude::*;
use gtk::prelude::*;
```

---

### 7. **Use the Component**

- Add your component to a parent widget or window.
- Call your public method (e.g., `set_items`) to populate the grid or list.

---

**Summary Checklist:**
- [ ] UI template with grid/list view and IDs
- [ ] Implementation struct with `TemplateChild` and model field
- [ ] Correct `ObjectSubclass` and wrapper setup
- [ ] Model setup in a public method, not in `instance_init`
- [ ] Use `gio::ListStore::new::<T>()` for models
- [ ] Import all required traits


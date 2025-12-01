# Notes about using Relm4 

## Component 

### update_with_view

This is handy when you want to access the widgets in the update function.

When handling updates with `update_with_view` and using `component` marco with `view` macro, remember to call `update_view(widgets, sender)` in the end of the `update_with_view` function to ensure the view is updated. Otherwise the view will not reflect the changes in the model. For example the `#[watch]` attributes will not react to model changes.


### view macro
#### entry fields
- when using: 
-- `connect_changed` to update the model
-- `#[watch]` attribute to update the view
This causes infinite update loop that can be solved by naming the signal handler (e.g. `@some_change_handler`) and blocking the signal with watch attribute.

For example:
```rust
gtk::Entry {
    #[watch]
    #[block_signal(executable_changed)]
    set_text: &model.executable,
    set_placeholder_text: Some("Emulator executable"),
    connect_changed[sender] => move |entry| {
        let buffer = entry.buffer();
        sender.input(
            EmulatorFormMsg::ExecutableChanged(buffer.text().into()),
        );
    } @executable_changed,
},
```

But in this case `set_text` was causing a cursor jump to the beginning of the entry field.
Because of all this, decided to use manual update when needed with `update_with_view` insead of using `#[watch]` with `#[block_signal(...)]`.

```
#[name = "executable_entry"]
gtk::Entry {
    set_text: &model.executable,
    set_placeholder_text: Some("Emulator executable"),
    connect_changed[sender] => move |entry| {
        let buffer = entry.buffer();
        sender.input(
            EmulatorFormMsg::ExecutableChanged(buffer.text().into()),
        );
    },
},
```

```
fn update_with_view(
    &mut self,
    msg: EmulatorFormMsg,
    widgets: &Self::Widgets,
    sender: ComponentSender<Self>,
) {
    match msg {
      EmulatorFormMsg::ExecutableChanged(executable) => {
            self.model.executable = executable;
      }
      EmulatorFormMsg::Show { editable_emulator } => {
            // ... 
            widgets.executable_entry.set_text(&self.executable);
            // ...
      }
    }
    self.update_view(widgets, sender);
}
```

## Closing dialog window

Add a `connect_close_request` handler to the `Window` component, which is triggered on close button (X) click:
```
view! {
    gtk::Window {
         connect_close_request[sender] => move |_| {
            sender.input(Msg::Hide);
            glib::Propagation::Stop
        },
    }
}
```

In message handler, do NOT call `root.close()`:
```
Msg::Hide => {
    root.hide();
}
```



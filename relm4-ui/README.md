# Notes about using Relm4 

## Component 

### update_with_view

This is handy when you want to access the widgets in the update function.

When handling updates with `update_with_view` and using `component` marco with `view` macro, remember to call `update_view(widgets, sender)` in the end of the `update_with_view` function to ensure the view is updated. Otherwise the view will not reflect the changes in the model. For example the `#[watch]` attributes will not react to model changes.



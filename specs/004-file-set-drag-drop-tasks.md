# 004 — Drag-and-Drop File Import — Tasks

## Model changes

- [ ] Add `drop_queue: Vec<PathBuf>` field to `FileSetFormModel` (initialised
  empty).

## Message changes

- [ ] Add `FilesDropped(Vec<PathBuf>)` variant to `FileSetFormMsg`.

## Message handling

- [x] Handle `FilesDropped` in `update_with_view`:
  - If `selected_file_type` is `None`, return early (no-op).
  - Append all paths to `self.drop_queue`.
  - If `self.processing` is `false`, pop the first path and send
    `FileSetFormMsg::FileSelected` for it (this sets `processing = true`).
- [x] Modify the `FileImportPrepared(Ok)` arm in `update_cmd`:
  - After handling the successful result as today, check `self.drop_queue`.
  - If the queue is non-empty, pop the next path and send
    `FileSetFormMsg::FileSelected` for it (keeps `processing = true`).
  - If the queue is empty, `processing` becomes `false` as today.
- [ ] Modify the `FileImportPrepared(Err)` arm in `update_cmd`:
  - Show the error dialog as today.
  - Check `self.drop_queue` and continue processing (same logic as the `Ok`
    arm).

## GTK4 wiring

- [x] In `init()`, after `view_output!()`, create a `gtk::DropTarget`:
  - Type: `gdk::FileList::static_type()`
  - Action: `gdk::DragAction::COPY`
- [x] Connect `connect_drop`: extract paths from `gdk::FileList`, send
  `FileSetFormMsg::FilesDropped(paths)`, return `true`.
- [ ] Connect `connect_enter`: add CSS class `drop-target-hover` to the
  scrolled window, return `gdk::DragAction::COPY`.
- [ ] Connect `connect_leave`: remove CSS class `drop-target-hover`.
- [x] Call `scrolled_window.add_controller(drop_target)`.
- [x] Name the scrolled window widget in the `view!` macro so it is accessible
  from `widgets` in `init()`.

## Manual verification checklist

- [ ] Open the file set form.
- [ ] Without selecting a file type: drag a file onto the list area — nothing
  happens (no crash, no error).
- [ ] Select a file type, then drag a single file onto the list area — the file
  appears in the list and the Create button becomes enabled.
- [ ] Drag two or more files at once — files appear in the list in drop order,
  the Create button stays disabled until all are processed.
- [ ] Drop a file that causes an import error — error dialog appears, remaining
  queued files continue to be processed afterwards.
- [ ] While hovering with files over the list area, a visual highlight is
  visible; it disappears when the drag leaves.
- [ ] The existing "Open File Picker" button still works normally alongside
  drag-and-drop.
- [ ] Mixing drag-and-drop and picker in the same session accumulates files
  correctly.

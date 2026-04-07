# 004 — Drag-and-Drop File Import for File Set Form

## Status
<!-- Planning | In Progress | Complete | Abandoned -->
In Progress

## Affected Crates
- `relm4-ui` — `FileSetFormModel`, `FileSetFormMsg`, GTK4 drop target wiring

## Overview

Users can drag one or more files from their file manager and drop them onto the
file set form as an alternative to using the "Open File Picker" button. Files
are processed sequentially (queued) through the existing `prepare_import`
pipeline.

## Requirements

### Drop target

- The scrolled window containing the file list is the drop target.
- Accepts `gdk::FileList` — the type GTK4 uses when files are dragged from a
  file manager.
- Drop action is `gdk::DragAction::COPY`.

### File type guard

- If no file type is selected when files are dropped, the drop is silently
  ignored (consistent with the existing "Open File Picker" guard).

### Queued processing

- Dropped files are appended to an internal pending queue (`Vec<PathBuf>`).
- If no import is currently in progress, the first queued file is dequeued and
  processed immediately via `prepare_import`.
- When a `FileImportPrepared` result is received (success or failure), the next
  file in the queue is dequeued and processed.
- Processing continues until the queue is empty.
- The `processing` flag is `true` for as long as any file is being processed or
  remains in the queue. The Create/Edit button therefore stays disabled
  throughout the whole batch.

### Error handling

- A failed `FileImportPrepared` shows the existing error dialog, then continues
  processing the remaining queued files.

### Visual feedback during hover

- While the user is hovering with files over the drop target, a CSS class
  `drop-target-hover` is applied to the scrolled window.
- The class is removed when the drag leaves or the drop completes.

## Out of scope

- Parallel (simultaneous) processing of multiple dropped files.
- Accepting URLs or text drops (files only).
- Any changes to the service or database layers.

## As Implemented
_(Pending)_

# Application Shutdown Design Documentation

## Overview

This document explains the design decisions around application shutdown, particularly how we handle concurrent operations (cloud sync) and prevent race conditions during the shutdown process.

## The Challenge

The application has asynchronous operations (cloud sync) that can be running when the user attempts to close the application. This creates several race conditions that must be handled carefully:

1. **User closes app while sync is running**
2. **Sync completes while close confirmation dialog is showing**
3. **User cancels close, but sync completes shortly after**
4. **Multiple close attempts in rapid succession**

## Shared State Design

### Why We Need Shared State

The close-request handler (`connect_close_request`) runs in a GTK callback closure and needs to check application state to decide whether to allow the close (return `Propagation::Proceed`) or block it (return `Propagation::Stop`).

However, the closure cannot directly access the `AppModel` state. We need a way to share state between:
- The GTK close-request handler (closure)
- The message handler (`update()` method)
- Async tasks (cloud sync)

### The Solution: `Arc<Mutex<Flags>>`

```rust
struct Flags {
    app_closing: bool,           // App is in shutdown process
    cloud_sync_in_progress: bool, // Cloud sync is running
    close_requested: bool,        // User requested close (dialog may be showing)
}

struct AppModel {
    flags: Arc<Mutex<Flags>>,
    // ...
}
```

**Why `Arc`?**
- Allows sharing ownership between closures and the model
- Clone is cheap (only increments reference count)
- Ensures data lives as long as needed

**Why `Mutex`?**
- Provides interior mutability (can modify through shared reference)
- Ensures thread-safe access (prevents data races)
- Lock/unlock pattern ensures no simultaneous modifications

**Why not `RwLock`?**
- We tried `RwLock` initially but it's more complex
- Most operations need write access anyway
- `Mutex` is simpler and sufficient for our needs

## The Three Flags Explained

### 1. `app_closing: bool`

**Purpose:** Indicates the app is in the shutdown process and close should proceed.

**Set to `true` when:**
- User confirms close (clicks OK on dialog)
- No sync is running and user closes app
- Sync completes after close was requested

**Used in:**
- `connect_close_request` - Returns `Propagation::Proceed` if true
- `CloseRequested` - Guards against duplicate processing
- `SyncWithCloud` - Prevents starting new sync during shutdown
- `SyncToCloudCompleted` - Skips showing completion dialog

**Example:**
```rust
// In close-request handler
if app_closing {
    Propagation::Proceed  // Let GTK close the window
} else {
    Propagation::Stop     // Block close, show our dialog
}
```

### 2. `cloud_sync_in_progress: bool`

**Purpose:** Tracks whether a cloud sync operation is currently running.

**Set to `true` when:**
- `SyncWithCloud` message starts the sync

**Set to `false` when:**
- `SyncToCloudCompleted` receives result (success or failure)

**Used in:**
- `CloseRequested` - Determines if we need to show confirmation dialog
- `SyncWithCloud` - Prevents starting multiple syncs simultaneously
- Dialog response - Checks if sync still running (might have completed)

**Example:**
```rust
if cloud_sync_in_progress {
    // Show confirmation dialog
    "Do you want to cancel the sync?"
} else {
    // Just close
}
```

### 3. `close_requested: bool`

**Purpose:** Tracks that user attempted to close, even if we blocked it with a dialog.

**Why needed?** Prevents race condition where:
1. User clicks X → Dialog shows
2. Sync completes → Would normally show completion dialog
3. Now we have TWO dialogs → Confusion and potential hang

**Set to `true` when:**
- `CloseRequested` message is received

**Used in:**
- `SyncToCloudCompleted` - Suppresses completion dialog and closes app instead

**Example:**
```rust
CommandMsg::SyncToCloudCompleted => {
    if close_requested {
        // User wanted to close, don't show completion dialog
        // Just close the app now that sync is done
        app_closing = true;
        root.close();
        return;
    }
    // Normal flow - show completion dialog
}
```

## Race Conditions and How We Handle Them

### Race Condition 1: Sync Completes While Dialog Showing

**Scenario:**
```
T0: User clicks X
T1: Show confirmation dialog
T2: Sync completes (in background)
T3: Completion dialog tries to show
T4: User responds to confirmation dialog
Result: Two dialogs, confused state
```

**Solution:**
```rust
// In CloseRequested
flags.close_requested = true;  // Mark immediately

// In SyncToCloudCompleted
if close_requested {
    // Skip completion dialog, just close
    flags.app_closing = true;
    root.close();
    return;
}
```

**Why it works:**
- `close_requested` flag prevents showing the completion dialog
- App closes cleanly once sync finishes
- User only sees one dialog (the confirmation)

### Race Condition 2: User Cancels Close, Then Sync Completes

**Scenario:**
```
T0: User clicks X
T1: Show confirmation dialog
T2: User clicks "Cancel" on dialog
T3: Sync completes 100ms later
Result: What should happen?
```

**Solution:**
```rust
// close_requested remains true even if user cancels
if close_requested {
    // Close automatically when sync completes
    flags.app_closing = true;
    root.close();
}
```

**Design Decision:**
Once user attempts to close, we interpret that as "close when safe". Even if they cancel the immediate close, we close when the sync completes. This prevents user confusion about app state.

**Alternative (not chosen):**
Reset `close_requested` when user cancels. But this requires passing more state into the dialog callback and is more complex.

### Race Condition 3: Multiple Close Attempts

**Scenario:**
```
T0: User clicks X
T1: Dialog shows
T2: User presses Ctrl+Q or clicks X again
Result: Multiple dialogs? Duplicate close logic?
```

**Solution:**
```rust
// In CloseRequested
if flags.app_closing {
    return;  // Already processing close, ignore
}
```

**Why it works:**
- First close request sets `close_requested = true`
- Subsequent close requests are ignored
- Only one dialog ever shows

### Race Condition 4: Sync Completes Between Dialog Check and Show

**Scenario:**
```
T0: Check sync_in_progress (true)
T1: Decide to show dialog
T2: Sync completes, sets sync_in_progress = false
T3: Dialog shows (but sync is already done!)
T4: User clicks OK -> tries to cancel already-done sync
```

**Solution:**
```rust
dialog.connect_response(move |dialog, response| {
    dialog.close();  // Close dialog first
    
    if response == gtk::ResponseType::Ok {
        // Check AGAIN if sync is still running
        let still_syncing = flags.cloud_sync_in_progress;
        
        if still_syncing {
            // Send cancel signal
        }
        
        // Close app regardless
        flags.app_closing = true;
        root.close();
    }
});
```

**Why it works:**
- We re-check the sync status when user responds
- If sync completed, we skip sending cancel signal (harmless)
- We close the app anyway (that's what user wanted)

## Flow Diagrams

### Normal Close (No Sync)

```
User clicks X
    ↓
connect_close_request fires
    ↓
Check: !app_closing && sync_in_progress
    ↓
Result: !false && false = false
    ↓
should_show_dialog = false
    ↓
Propagation::Proceed (immediate!)
    ↓
GTK closes window naturally
    ↓
App exits
```

**Note:** No `CloseRequested` message is sent. The close happens immediately because there's nothing to wait for.

### Close During Sync (User Confirms)

```
User clicks X
    ↓
connect_close_request fires
    ↓
app_closing = false → Propagation::Stop
    ↓
CloseRequested message sent
    ↓
close_requested = true
sync_in_progress = true
    ↓
Show confirmation dialog
    ↓
User clicks OK
    ↓
Send cancel signal to sync
    ↓
app_closing = true
root.close()
    ↓
connect_close_request fires
    ↓
app_closing = true → Propagation::Proceed
    ↓
GTK closes window
    ↓
(Sync cancel is handled asynchronously)
    ↓
App exits
```

### Close During Sync (Sync Completes First)

```
User clicks X
    ↓
connect_close_request fires
    ↓
CloseRequested message sent
    ↓
close_requested = true
sync_in_progress = true
    ↓
Show confirmation dialog
    ↓
    [Sync completes in background]
        ↓
    SyncToCloudCompleted fires
        ↓
    close_requested = true (detected!)
        ↓
    Skip completion dialog
        ↓
    app_closing = true
        ↓
    root.close()
            ↓
User sees dialog close on its own
    ↓
App exits
```

### Close During Sync (User Cancels)

```
User clicks X
    ↓
CloseRequested message
    ↓
close_requested = true
sync_in_progress = true
    ↓
Show confirmation dialog
    ↓
User clicks Cancel
    ↓
Dialog closes
(close_requested stays true!)
    ↓
    [Sync continues]
    ↓
    [Sync completes]
    ↓
SyncToCloudCompleted fires
    ↓
close_requested = true (still set!)
    ↓
Skip completion dialog
    ↓
app_closing = true
root.close()
    ↓
App exits automatically
```

**Note:** This behavior might seem unexpected (app closes even though user cancelled), but it's intentional. Once user attempts to close, we assume intent to close and wait for safe moment.

## Lock Management Best Practices

### Pattern 1: Short Lock Scope

```rust
// GOOD: Lock only for reading/writing flags
let sync_in_progress = {
    let flags = self.flags.lock().unwrap();
    flags.cloud_sync_in_progress
};  // Lock released here

if sync_in_progress {
    // Do expensive work without holding lock
}
```

```rust
// BAD: Holding lock during UI operations
let flags = self.flags.lock().unwrap();
if flags.cloud_sync_in_progress {
    show_dialog();  // Might block! Lock held entire time
}
```

### Pattern 2: Drop Locks Before Blocking Operations

```rust
let mut flags = self.flags.lock().unwrap();
flags.cloud_sync_in_progress = false;
drop(flags);  // Explicitly release lock

// Now safe to show dialog (might block)
show_info_dialog(message, root);
```

### Pattern 3: Re-check State After Lock Release

```rust
// Read state with lock
let should_show_dialog = {
    let flags = self.flags.lock().unwrap();
    flags.cloud_sync_in_progress
};

// Lock released, show dialog
if should_show_dialog {
    let dialog = create_dialog();
    
    // In dialog callback, re-check state!
    dialog.connect_response(move |dialog, response| {
        let flags = flags_clone.lock().unwrap();
        // State might have changed while dialog was showing
        if flags.cloud_sync_in_progress {
            // Still running, send cancel
        }
    });
}
```

## Common Pitfalls and How to Avoid Them

### Pitfall 1: Forgetting to Set close_requested

**Problem:**
```rust
// In CloseRequested
if sync_in_progress {
    show_dialog();  // Forgot to set close_requested!
}
```

**Consequence:** Sync completion shows dialog, race condition occurs.

**Solution:** Always set `close_requested = true` immediately in `CloseRequested`.

### Pitfall 2: Holding Lock During UI Operations

**Problem:**
```rust
let flags = self.flags.lock().unwrap();
show_dialog();  // Dialog blocks, lock held!
// Other code waiting on lock will deadlock
```

**Solution:** Always drop locks before UI operations.

### Pitfall 3: Not Re-checking State in Callbacks

**Problem:**
```rust
if sync_in_progress {
    dialog.connect_response(move |_, response| {
        // Assumes sync still running, might have completed!
        send_cancel_signal();
    });
}
```

**Solution:** Always re-check flags inside callbacks.

### Pitfall 4: Forgetting close_requested Reset

Actually, we **don't** reset `close_requested` in the current design. This is intentional but worth documenting.

**Design Choice:** Once user attempts to close, we assume intent to close when safe.

**Alternative Design:** Reset `close_requested` when user cancels dialog.
```rust
// If we wanted this behavior:
if response == gtk::ResponseType::Cancel {
    let mut flags = flags_clone.lock().unwrap();
    flags.close_requested = false;  // Let them keep working
}
```

We chose not to do this because it's simpler and matches user expectations better.

## Testing Checklist

When modifying shutdown logic, test these scenarios:

- [ ] Close app when idle → Closes immediately
- [ ] Close app during sync → Shows dialog
- [ ] Confirm close during sync → Cancels and closes
- [ ] Cancel close dialog → App stays open, sync continues
- [ ] Sync completes while dialog showing → Dialog closes, app exits
- [ ] Click X multiple times rapidly → Only one dialog shows
- [ ] Start sync while `app_closing` is true → Sync is prevented
- [ ] Completion dialog appears if close was not requested
- [ ] Completion dialog is suppressed if close was requested
- [ ] App doesn't hang with multiple dialogs

## Future Improvements

### 1. Reset close_requested on Cancel

If we want to let users keep working after cancelling close:

```rust
if response == gtk::ResponseType::Cancel {
    let mut flags = flags_clone.lock().unwrap();
    flags.close_requested = false;
}
```

**Pros:** More intuitive behavior  
**Cons:** More complex, need to handle sync completion differently

### 2. Show Progress in Close Dialog

Instead of modal blocking dialog, show progress:

```rust
"Cloud sync in progress (3 of 10 files)"
[Progress bar]
[Cancel and Close] [Continue Working]
```

**Pros:** Better UX, clear feedback  
**Cons:** More complex UI state management

### 3. Timeout for Sync Cancellation

If sync doesn't respond to cancel in N seconds, force close:

```rust
timeout::spawn(Duration::from_secs(10), async {
    if still_syncing {
        force_close();
    }
});
```

**Pros:** Prevents indefinite waiting  
**Cons:** Might leave partial uploads, need careful handling

### 4. Save State and Resume

Allow app to close immediately, resume sync on next launch:

```rust
// On close
save_sync_state_to_db();

// On launch
if has_incomplete_sync() {
    show_resume_dialog();
}
```

**Pros:** Better UX for large syncs  
**Cons:** Complex state management, need persistent storage

## Related Files

- `relm4-ui/src/main.rs` - Main shutdown logic
- `service/src/cloud_sync/steps.rs` - Cancellation checks (lines 188, 436)
- `service/src/cloud_sync/context.rs` - Cancel receiver in context
- `service/src/cloud_sync/service.rs` - Sync service entry point
- `service/src/error.rs` - `OperationCancelled` error variant

## Summary

The shutdown design uses three flags (`app_closing`, `cloud_sync_in_progress`, `close_requested`) in shared state (`Arc<Mutex<Flags>>`) to handle complex race conditions between user actions, async operations, and GTK window management.

The key insight is that **`close_requested` acts as a latch** - once set, it changes the behavior of sync completion from "show dialog" to "close app". This prevents the common race condition of multiple dialogs appearing simultaneously.

While the design is somewhat complex, it handles all edge cases correctly and provides a clean user experience. The complexity is necessary due to the asynchronous nature of cloud sync and the limitations of GTK's close-request handling.

---

*Last updated: 2024-11-24*
*Author: Implementation team*
*Status: Current implementation*

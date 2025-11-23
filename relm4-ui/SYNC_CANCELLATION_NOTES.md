# Cloud Sync Cancellation Implementation Notes

## Current Issue (before this pull request)

When the user closes the application during an active cloud sync operation:

### Problems:
- ❌ Sync task continues briefly then gets killed when app exits
- ❌ Partial uploads may be left incomplete in cloud storage
- ❌ No cleanup handlers are executed
- ❌ Database may show inconsistent state (files marked as "uploading" forever)
- ❌ No warning to user that sync is in progress
- ❌ Wasted bandwidth from incomplete uploads that must restart

### Current Code Flow (before this pull request)

```rust
// In main.rs, line ~290
AppMsg::SyncWithCloud => {
    // Spawns detached async task
    task::spawn(async move {
        sync_service_clone.sync_to_cloud(tx).await
        // If app closes, this is killed mid-upload!
    });
}

// When user closes window:
// 1. Window close event fires
// 2. App exits immediately
// 3. Spawned task is terminated
// 4. No cleanup
```

## Recommended Solution

### Phase 1: Block Window Close During Sync ⭐ Priority [DONE]

**Goals:**
- Prevent user from accidentally closing during sync
- Show clear feedback that sync is in progress
- Allow graceful cancellation

**Implementation Steps:**

1. **Track Sync State in AppModel**
   ```rust
   struct AppModel {
       // ... existing fields
       sync_in_progress: bool,
       sync_cancel_tx: Option<async_std::channel::Sender<()>>,
   }
   ```

2. **Connect to Window Close Request**
   ```rust
   // In init() or init_root()
   root.connect_close_request(move |window| {
       if sync_in_progress {
           // Show dialog
           let dialog = gtk::MessageDialog::builder()
               .transient_for(window)
               .modal(true)
               .message_type(gtk::MessageType::Warning)
               .buttons(gtk::ButtonsType::OkCancel)
               .text("Cloud sync in progress")
               .secondary_text("Closing now will cancel the sync. Current file will finish uploading. Continue?")
               .build();
           
           dialog.connect_response(move |dialog, response| {
               if response == gtk::ResponseType::Ok {
                   // Send cancel signal
                   // Wait for sync to finish current file
                   // Then close
               }
               dialog.close();
           });
           
           dialog.present();
           glib::Propagation::Stop // Block close
       } else {
           glib::Propagation::Proceed // Allow close
       }
   });
   ```

3. **Update Sync State**
   ```rust
   AppMsg::SyncWithCloud => {
       self.sync_in_progress = true;
       
       let (cancel_tx, cancel_rx) = unbounded::<()>();
       self.sync_cancel_tx = Some(cancel_tx);
       
       // Pass cancel_rx to sync service
       task::spawn(async move {
           let result = sync_service_clone.sync_to_cloud(tx, cancel_rx).await;
           // Send completion message to reset sync_in_progress
       });
   }
   ```

### Phase 2: Add Cancellation to Sync Pipeline

**Files to Modify:**

1. **service/src/cloud_sync/context.rs**
   ```rust
   pub struct SyncContext {
       // ... existing fields
       pub cancel_rx: async_std::channel::Receiver<()>,
   }
   ```

2. **service/src/cloud_sync/service.rs**
   ```rust
   pub async fn sync_to_cloud(
       &self, 
       progress_tx: Sender<SyncEvent>,
       cancel_rx: Receiver<()>,  // Add this parameter
   ) -> Result<SyncResult, Error> {
       let mut context = SyncContext::new(
           self.repository_manager.clone(),
           self.settings.clone(),
           progress_tx.clone(),
           cancel_rx,  // Pass it through
       );
       // ... rest of implementation
   }
   ```

3. **service/src/cloud_sync/steps.rs** (line ~186)
   ```rust
   // In UploadPendingFilesStep::execute()
   loop {
       // Check for cancellation before each file
       if context.cancel_rx.try_recv().is_ok() {
           tracing::info!("Sync cancelled by user");
           return StepAction::Abort(Error::Cancelled);
       }
       
       // Fetch next batch of files
       let pending_files = ...;
       
       if pending_files.is_empty() {
           break;
       }
       
       for file in pending_files {
           // Upload file (this will complete)
           // ...
       }
   }
   ```

4. **service/src/error.rs**
   ```rust
   #[derive(Error, Debug)]
   pub enum Error {
       // ... existing variants
       #[error("Operation cancelled by user")]
       Cancelled,
   }
   ```

### Phase 3: Handle Completion/Cancellation in UI

```rust
enum CommandMsg {
    // ... existing variants
    SyncCompleted(Result<SyncResult, Error>),
}

// In update_cmd()
CommandMsg::SyncCompleted(result) => {
    self.sync_in_progress = false;
    self.sync_cancel_tx = None;
    
    match result {
        Ok(sync_result) => {
            // Show success message
            println!("Sync completed: {} uploads", sync_result.successful_uploads);
        }
        Err(Error::Cancelled) => {
            // Show cancellation message
            println!("Sync cancelled by user");
        }
        Err(e) => {
            // Show error
            eprintln!("Sync failed: {}", e);
        }
    }
}
```

## Benefits of This Approach

✅ **Clean Cancellation**
- Current file upload completes normally
- No partial uploads left in inconsistent state
- Database state remains consistent

✅ **User Control**
- Clear warning before closing during sync
- User can choose to wait or cancel
- Cancellation is immediate (after current file)

✅ **Similar to Download Cancellation**
- Uses same pattern as implemented for HTTP downloads
- Consistent UX across the application
- Proven approach

✅ **No Data Loss**
- Files that were uploaded successfully remain uploaded
- Files not yet started remain marked for next sync
- Current file either completes or fails normally

## Testing Checklist

When implementing, test these scenarios:

- [ ] Start sync, try to close window → Dialog appears
- [ ] Cancel sync via dialog → Current file completes, sync stops
- [ ] Let sync complete normally → Window can close freely
- [ ] Start sync with large files → Cancellation is responsive
- [ ] Check database state after cancellation → No stuck "uploading" files
- [ ] Restart sync after cancellation → Picks up where it left off
- [ ] Multiple close attempts during sync → Dialog shown each time

## Similar Implementations in Codebase

This is very similar to the download cancellation we implemented:

**Download Cancellation** (`http_downloader/src/lib.rs`):
- Uses `Receiver<()>` for cancellation signal
- Checks `cancel_rx.try_recv()` in download loop
- Returns `DownloadError::Cancelled` 
- UI blocks progress bar and sends cancel signal

**Sync Cancellation** (to implement):
- Use same pattern with `Receiver<()>`
- Check in upload loop (steps.rs line 186)
- Return `Error::Cancelled`
- UI blocks window close and sends cancel signal

## Future Enhancements

Once basic cancellation is working, consider:

1. **Progress Estimation**
   - Show "Cancelling after current file..." message
   - Display current file being uploaded

2. **Retry Failed Files**
   - Add button to retry just the failed uploads
   - Don't need to rescan entire collection

3. **Background Sync**
   - Continue sync even after window closes
   - More complex, requires service/daemon approach

4. **Pause/Resume**
   - Pause sync, close app, resume later
   - Would need persistent state management

## Related Files

- `relm4-ui/src/main.rs` - Main app, sync trigger, window management
- `service/src/cloud_sync/service.rs` - Sync service entry point
- `service/src/cloud_sync/context.rs` - Sync context with state
- `service/src/cloud_sync/steps.rs` - Upload loop (line 130-380)
- `service/src/cloud_sync/pipeline.rs` - Pipeline execution
- `http_downloader/src/lib.rs` - Reference implementation for cancellation

## Implementation Timeline

**Estimated effort: 2-3 hours**

1. Phase 1 (Block close): ~1 hour
2. Phase 2 (Cancellation): ~1 hour  
3. Phase 3 (UI polish): ~30 min
4. Testing: ~30 min

---

*Last updated: 2025-11-23*
*Status: Phase 1 implemented, Phases 2-3 pending*
*Priority: Medium-High (prevents data inconsistency)*

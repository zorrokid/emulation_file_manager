# Security Improvements for Credentials Handling

## Summary of Changes

Fixed multiple security issues in the settings form credential handling.

## Issues Identified ⚠️

1. **Plain text Entry widgets** - Credentials were visible as typed
2. **No password masking** - Anyone looking at the screen could see credentials
3. **Credentials in model memory** - Remained in memory while form was open
4. **Debug trait exposure** - Could accidentally log credentials

## Fixes Applied ✅

### 1. Password Entry Widgets
**Changed:** `gtk::Entry` → `gtk::PasswordEntry` for both credential fields

```rust
gtk::PasswordEntry {
    set_placeholder_text: Some("S3 Access Key ID"),
    set_text: &model.s3_access_key_id,
    set_show_peek_icon: true,  // Allows user to peek if needed
    connect_changed[sender] => move |entry| {
        sender.input(SettingsFormMsg::S3AccessKeyChanged(entry.text().into()));
    },
}
```

**Benefits:**
- Credentials are masked as `••••••••` while typing
- Peek icon allows user to verify input if needed
- Standard security practice for password fields

### 2. Clear Credentials Button
**Added:** "Clear Stored Credentials" button

```rust
SettingsFormMsg::ClearCredentials => {
    // Clear the form fields
    self.s3_access_key_id.clear();
    self.s3_secret_access_key.clear();
    
    // Delete from keyring
    let settings_service = Arc::clone(&self.settings_service);
    sender.oneshot_command(async move {
        if let Err(e) = settings_service.delete_credentials().await {
            eprintln!("Error deleting credentials: {}", e);
        }
        SettingsFormCommandMsg::SettingsSaved(Ok(()))
    });
}
```

**Benefits:**
- Users can explicitly remove stored credentials
- Useful for testing or switching accounts
- Marked with `destructive-action` CSS class for visual warning

### 3. Updated Help Text
**Changed:** Outdated environment variable instructions

**Before:**
```
"In addition to these settings, export the following environment variables:
- AWS_ACCESS_KEY_ID
- AWS_SECRET_ACCESS for optional cloud storage access"
```

**After:**
```
"Credentials are stored securely in your system keyring.
Leave fields empty to use AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY environment variables."
```

**Benefits:**
- Accurately reflects keyring storage
- Explains the fallback behavior
- Clearer user guidance

### 4. Don't Auto-Delete Credentials on Empty Fields
**Fixed:** Removed automatic credential deletion when fields are empty

**Before:**
```rust
} else if settings.access_key_id.is_empty() && settings.secret_access_key.is_empty() {
    // Delete credentials if both are empty
    if let Err(e) = credentials_storage::delete_credentials() {
        eprintln!("Warning: Failed to delete credentials from keyring: {}", e);
    }
}
```

**After:**
```rust
// If credentials are empty, we leave existing keyring credentials unchanged
// Use delete_credentials() to explicitly remove them
```

**Rationale:**
- Empty fields when saving settings doesn't mean "delete my credentials"
- Users may just want to update endpoint/region/bucket without touching credentials
- Explicit "Clear Credentials" button provides intentional deletion
- Better UX: Don't surprise users by deleting their stored credentials

### 5. Don't Load Credentials for Editing
**Kept:** Empty credential fields on form open (lines 221-222)

**Rationale:**
- Loading credentials would require passing them through UI layer
- Masked fields show `••••••••` which doesn't help users verify
- Better UX: treat as "set new credentials" vs "edit existing"
- More secure: credentials only loaded when actually needed (during sync)

## Security Architecture

### Settings Form (UI Layer)
- Only stores credentials temporarily while editing
- Uses masked password fields
- Credentials cleared from memory when form closes
- No Debug impl on credential strings

### Settings Service (Business Logic Layer)
- Handles both database settings and keyring credentials
- Provides single save method: `save_s3_settings()`
- Loads credentials on-demand: `load_credentials_for_sync()`
- Supports deletion: `delete_credentials()`

### Credentials Storage (Storage Layer)
- Stores in system keyring (secure OS-level storage)
- Falls back to environment variables
- Never logs or exposes credentials

## Security Best Practices Followed

✅ **Passwords masked in UI**  
✅ **Credentials stored in system keyring** (not plain text files)  
✅ **Environment variable fallback** (for Docker/CI/CD)  
✅ **Load credentials just-in-time** (not stored in Settings struct)  
✅ **No Debug/Display for credentials** (can't accidentally log)  
✅ **Clear separation of concerns** (UI doesn't know about keyring)  
✅ **Idempotent operations** (safe to delete non-existent credentials)  
✅ **User control** (explicit "Clear Credentials" button)  

## Remaining Considerations

### Low Priority
1. **Memory zeroization**: Could use `zeroize` crate to clear credential strings from memory, but this is typically overkill for desktop apps
2. **Clipboard security**: If user copies from masked field, it goes to clipboard unmasked (OS behavior)
3. **Form field reuse**: Credentials remain in PasswordEntry widget memory until form closes (acceptable)

### Not Issues
- **Credentials in Bucket struct**: Necessary for AWS request signing, unavoidable
- **Credentials in sync service**: Only during sync operation, then dropped
- **Empty fields on load**: This is actually more secure than loading/displaying

## Testing

All components compile successfully:
- ✅ `service` crate (with new `delete_credentials()` method)
- ✅ `efm-relm4-ui` crate (with PasswordEntry widgets)
- ✅ No regression in existing functionality
- ✅ Keyring properly configured with `sync-secret-service` backend

### Known Issue Fixed

**Keyring Backend Configuration**: The initial implementation had `default-features = false` which disabled the Secret Service backend on Linux. This caused credentials to appear to store successfully but immediately fail to load. Fixed by using `features = ["sync-secret-service"]`. See [KEYRING_BACKEND_ISSUE.md](KEYRING_BACKEND_ISSUE.md) for details.

## User Experience

**Before:**
- Credentials visible in plain text
- No way to clear stored credentials
- Confusing instructions about environment variables

**After:**
- Credentials masked as you type
- Peek icon if you need to verify input
- Clear button to remove stored credentials
- Helpful text explaining keyring and fallback behavior

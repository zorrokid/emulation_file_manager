# Keyring Backend Configuration Issue

## Problem Summary

Credentials were being stored successfully but immediately failed to load with `NoEntry` error, even though GNOME Keyring daemon was running and `secret-tool` worked correctly.

## Symptoms

```
DEBUG: Storing credentials to keyring...
DEBUG [credentials_storage]: ✓ Successfully called set_password on keyring
DEBUG: ✓ Credentials stored successfully

DEBUG: Attempting to load credentials from keyring...
DEBUG [credentials_storage]: ✗ NoEntry error from keyring
DEBUG: No credentials found in keyring or environment
```

## Root Cause

The `keyring` Rust crate was configured with incorrect features:

1. **Initial configuration**: `keyring = { version="3.6.3", default-features = false }`
   - This disabled ALL backends, using a no-op implementation
   - `set_password()` succeeded but didn't actually store anything
   - `get_password()` always returned `NoEntry`

2. **First fix attempt**: `keyring = { version = "3.6.3" }`
   - Enabled default features, but still didn't work
   - Default features use async backends which didn't function in our context

3. **Working solution**: `keyring = { version = "3.6.3", features = ["sync-secret-service"] }`
   - Explicitly enables synchronous Secret Service backend for Linux
   - Properly integrates with GNOME Keyring via D-Bus

## Investigation Steps

### 1. Verified GNOME Keyring Works
```bash
# Test with secret-tool command
echo "test-password" | secret-tool store --label="Test" service efm-cloud-sync username s3-credentials
secret-tool lookup service efm-cloud-sync username s3-credentials
# ✓ Worked perfectly
```

### 2. Verified Keyring Daemon Running
```bash
systemctl --user status gnome-keyring-daemon
# Active: active (running)
```

### 3. Created Standalone Test
Created `credentials_storage/examples/test_keyring.rs` to isolate the issue:
```rust
use credentials_storage::{CloudCredentials, store_credentials, load_credentials};

fn main() {
    let test_creds = CloudCredentials {
        access_key_id: "TEST_KEY_ID".to_string(),
        secret_access_key: "TEST_SECRET".to_string(),
    };
    
    store_credentials(&test_creds).unwrap();
    let loaded = load_credentials().unwrap();
    assert_eq!(loaded.access_key_id, "TEST_KEY_ID");
}
```

This test failed with `default-features = false` and default features, but succeeded with `sync-secret-service`.

### 4. Debug Logging
Added detailed debug logging to trace the exact point of failure:
- `set_password()` completed successfully
- `get_password()` immediately returned `NoEntry` error
- No exceptions or panics - the backend was simply not persisting data

## Solution

**File**: `credentials_storage/Cargo.toml`

```toml
[dependencies]
keyring = { version = "3.6.3", features = ["sync-secret-service"] }
```

## Platform-Specific Backends

The `keyring` crate supports multiple backends:

- **Linux**: 
  - `sync-secret-service` - Synchronous GNOME Keyring/KDE Wallet (recommended) ✅
  - `async-secret-service` - Async version
  
- **macOS**: 
  - `apple-native` - Uses macOS Keychain
  
- **Windows**: 
  - `windows-native` - Uses Windows Credential Manager

For cross-platform support, you could use:
```toml
[target.'cfg(target_os = "linux")'.dependencies]
keyring = { version = "3.6.3", features = ["sync-secret-service"] }

[target.'cfg(target_os = "macos")'.dependencies]
keyring = { version = "3.6.3", features = ["apple-native"] }

[target.'cfg(target_os = "windows")'.dependencies]
keyring = { version = "3.6.3", features = ["windows-native"] }
```

## Verification

After the fix:
```
1. Storing credentials...
   ✓ Stored successfully

2. Loading credentials...
   ✓ Got password from keyring (len=73)
   ✓ Deserialized credentials successfully
   ✓ Loaded successfully
   Access Key ID: TEST_KEY_ID_123
   Secret matches: true
```

## Lessons Learned

1. **Don't disable default features without understanding implications**
   - `default-features = false` can disable critical functionality
   
2. **Platform-specific features matter**
   - Keyring storage requires OS-specific backend implementations
   
3. **Test with minimal examples**
   - Creating `examples/test_keyring.rs` isolated the problem from the application
   
4. **System tools can verify backend works**
   - `secret-tool` confirmed GNOME Keyring was functional
   - The issue was in the Rust library configuration, not the system

5. **Debug logging is invaluable**
   - Adding `eprintln!` statements at each step pinpointed exact failure point
   - Showed operations succeeded locally but didn't persist

## Related Files

- `credentials_storage/Cargo.toml` - Keyring dependency configuration
- `credentials_storage/src/lib.rs` - Credentials storage implementation
- `credentials_storage/examples/test_keyring.rs` - Standalone test
- `service/src/settings_service.rs` - Service layer using credentials storage

## Date

Fixed: October 29, 2025

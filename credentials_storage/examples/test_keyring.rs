use credentials_storage::{CloudCredentials, store_credentials, load_credentials};

fn main() {
    println!("=== Testing Keyring Storage ===\n");
    
    let test_creds = CloudCredentials {
        access_key_id: "TEST_KEY_ID_123".to_string(),
        secret_access_key: "TEST_SECRET_456".to_string(),
    };
    
    println!("1. Storing credentials...");
    match store_credentials(&test_creds) {
        Ok(_) => println!("   ✓ Stored successfully\n"),
        Err(e) => {
            println!("   ✗ Failed: {}\n", e);
            return;
        }
    }
    
    println!("2. Loading credentials...");
    match load_credentials() {
        Ok(loaded) => {
            println!("   ✓ Loaded successfully");
            println!("   Access Key ID: {}", loaded.access_key_id);
            println!("   Secret matches: {}\n", loaded.secret_access_key == test_creds.secret_access_key);
        }
        Err(e) => {
            println!("   ✗ Failed: {}\n", e);
        }
    }
    
    println!("3. Cleaning up...");
    match credentials_storage::delete_credentials() {
        Ok(_) => println!("   ✓ Deleted successfully"),
        Err(e) => println!("   ✗ Failed to delete: {}", e),
    }
}

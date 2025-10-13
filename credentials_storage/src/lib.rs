use keyring::Entry;

const SERVICE_NAME: &str = "efm-cloud-sync";

pub fn store_credentials(
    access_key_id: &str,
    secret_access_key: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let entry = Entry::new(SERVICE_NAME, access_key_id)?;
    entry.set_password(secret_access_key)?;
    Ok(())
}

pub fn load_credentials(access_key_id: &str) -> Result<Option<String>, keyring::Error> {
    let entry = Entry::new(SERVICE_NAME, access_key_id)?;
    match entry.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn delete_credentials(access_key_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let entry = Entry::new(SERVICE_NAME, access_key_id)?;
    entry.delete_credential()?;
    Ok(())
}

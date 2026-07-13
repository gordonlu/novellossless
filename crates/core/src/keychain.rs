const SERVICE_NAME: &str = "novellossless";
const KEY_NAME: &str = "ai_api_key";

/// Store an API key in the OS keychain.
/// Returns Ok(true) if stored, Ok(false) if keychain is unavailable
/// (e.g., headless Linux without a keyring daemon).
pub fn store_api_key(key: &str) -> Result<bool, String> {
    match keyring::Entry::new(SERVICE_NAME, KEY_NAME) {
        Ok(entry) => entry
            .set_password(key)
            .map(|_| true)
            .map_err(|e| format!("keychain write failed: {e}")),
        Err(e) => {
            eprintln!("[novellossless] keychain unavailable: {e}");
            Ok(false)
        }
    }
}

/// Retrieve an API key from the OS keychain.
/// Returns None if:
///   - No entry exists in the keychain
///   - The keychain is unavailable
pub fn get_api_key() -> Option<String> {
    let entry = keyring::Entry::new(SERVICE_NAME, KEY_NAME).ok()?;
    entry.get_password().ok()
}

/// Delete an API key from the OS keychain.
pub fn delete_api_key() -> Result<(), String> {
    match keyring::Entry::new(SERVICE_NAME, KEY_NAME) {
        Ok(entry) => entry
            .delete_credential()
            .map_err(|e| format!("keychain delete failed: {e}")),
        Err(e) => Err(format!("keychain unavailable: {e}")),
    }
}

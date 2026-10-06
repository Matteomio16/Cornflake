//! API keys live in the OS credential store (Windows Credential Manager), never in SQLite or files.

const SERVICE: &str = "Cornflake";

fn entry(name: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, name).map_err(|e| format!("credential store unavailable: {e}"))
}

pub fn get(name: &str) -> Result<Option<String>, String> {
    match entry(name)?.get_password() {
        Ok(v) if !v.is_empty() => Ok(Some(v)),
        Ok(_) | Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("failed to read credential '{name}': {e}")),
    }
}

pub fn set(name: &str, value: &str) -> Result<(), String> {
    if value.is_empty() {
        return delete(name);
    }
    entry(name)?
        .set_password(value)
        .map_err(|e| format!("failed to store credential '{name}': {e}"))
}

pub fn delete(name: &str) -> Result<(), String> {
    match entry(name)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("failed to delete credential '{name}': {e}")),
    }
}

pub fn llm_key_name(provider: &str) -> String {
    format!("llm:{provider}")
}

pub fn transcription_key_name(provider: &str) -> String {
    format!("stt:{provider}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_in_credential_store() {
        let name = format!("test:{}", uuid::Uuid::new_v4());
        assert_eq!(get(&name).unwrap(), None);
        set(&name, "dummy-value").unwrap();
        assert_eq!(get(&name).unwrap().as_deref(), Some("dummy-value"));
        delete(&name).unwrap();
        assert_eq!(get(&name).unwrap(), None);
    }
}

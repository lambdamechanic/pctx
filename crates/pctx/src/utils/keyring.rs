use anyhow::{Context, Result};
use log::debug;

/// Store a value in the system keychain as a password
pub(crate) fn store_in_keychain(key: &str, val: &str) -> Result<()> {
    let entry = keyring::Entry::new("pctx", key).context("Failed to create keychain entry")?;
    debug!("Value stored in keychain service=\"pctx\", user=\"{key}\"");

    entry
        .set_password(val)
        .context("Failed to store password in keychain")?;

    Ok(())
}

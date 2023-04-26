use std::path::Path;

use eyre::Context;
use serde::de::DeserializeOwned;

pub mod secret_key {
    use ethers::prelude::k256::SecretKey;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(
        _key: &SecretKey,
        _serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // let mut bytes = [0u8; 32];
        // key.write(&mut bytes[..])?;
        // serializer.serialize_str(&hex::encode(&bytes))
        todo!()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SecretKey, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let s = s.trim_start_matches("0x");

        let bytes = hex::decode(s).map_err(serde::de::Error::custom)?;

        SecretKey::from_slice(&bytes).map_err(serde::de::Error::custom)
    }
}

pub async fn read_deserialize<T>(path: impl AsRef<Path>) -> eyre::Result<T>
where
    T: DeserializeOwned,
{
    let path = path.as_ref();

    let content = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("Reading from {}", path.display()))?;

    let value = toml::from_str(&content)?;

    Ok(value)
}

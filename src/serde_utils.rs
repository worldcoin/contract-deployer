use std::path::Path;

use eyre::Context;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub mod secret_key {
    use ethers::prelude::k256::SecretKey;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(
        key: &SecretKey,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let gen_arr = key.to_bytes();
        let bytes = gen_arr.as_slice();
        serializer.serialize_str(&hex::encode(bytes))
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

    let value = serde_yaml::from_str(&content).with_context(|| {
        format!("Parsing {} content was {content}", path.display())
    })?;

    Ok(value)
}

pub async fn write_serialize<T>(
    path: impl AsRef<Path>,
    value: T,
) -> eyre::Result<()>
where
    T: Serialize,
{
    let path = path.as_ref();

    let content = serde_yaml::to_string(&value)
        .with_context(|| format!("Serializing {}", path.display()))?;

    tokio::fs::write(path, content)
        .await
        .with_context(|| format!("Writing to {}", path.display()))?;

    Ok(())
}

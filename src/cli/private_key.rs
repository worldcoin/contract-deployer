use std::fmt;
use std::str::FromStr;

use ethers::prelude::k256::SecretKey;

#[derive(Debug, Clone)]
pub struct PrivateKey {
    pub key: SecretKey,
}

impl FromStr for PrivateKey {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim_start_matches("0x");

        let bytes = hex::decode(s)?;

        let key = SecretKey::from_slice(&bytes)?;

        Ok(Self { key })
    }
}

impl fmt::Display for PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "{}", hex::encode(self.key.to_bytes()))
        } else {
            let encoded = hex::encode(self.key.to_bytes());

            let first_4 = &encoded[0..4];
            let last_4 = &encoded[encoded.len() - 4..];

            write!(f, "{}...{}", first_4, last_4)
        }
    }
}

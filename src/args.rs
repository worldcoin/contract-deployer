use std::fmt;
use std::str::FromStr;

use clap::Args;
use ethers::prelude::k256::SecretKey;
use reqwest::Url;

#[derive(Debug, Clone, Args)]
#[clap(rename_all = "kebab-case")]
pub struct DeploymentArgs {
    #[clap(short, long, env)]
    pub private_key: PrivateKey,

    #[clap(short, long, env)]
    pub rpc_url: Url,
}

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
        write!(f, "{}", hex::encode(self.key.to_bytes()))
    }
}

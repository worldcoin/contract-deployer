use std::path::PathBuf;

use reqwest::Url;

use crate::cli::PrivateKey;

pub struct Cmd {
    pub config: PathBuf,
    pub deployment_name: String,
    pub private_key: PrivateKey,
    pub rpc_url: Url,
    pub etherscan_api_key: Option<String>,
    pub cache_dir: PathBuf,
}

impl Cmd {
    pub fn new(
        config: PathBuf,
        deployment_name: String,
        private_key: PrivateKey,
        rpc_url: Url,
        etherscan_api_key: Option<String>,
        cache_dir: PathBuf,
    ) -> Self {
        Self {
            config,
            deployment_name,
            private_key,
            rpc_url,
            etherscan_api_key,
            cache_dir,
        }
    }
}

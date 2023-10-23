use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;

use ethers::types::Address;
use reqwest::Url;

use crate::cli::PrivateKey;
use crate::dependency_map::DependencyMap;
use crate::forge_utils::verify::ForgeVerify;
use crate::forge_utils::{ContractSpec, ForgeCreate};
use crate::report::Report;

#[derive(Debug)]
pub struct DeploymentContext {
    pub deployment_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub dep_map: DependencyMap,
    pub nonce: AtomicU64,
    pub report: Report,
    pub private_key: PrivateKey,
    pub rpc_url: Url,
    pub etherscan_api_key: Option<String>,
}

impl DeploymentContext {
    pub fn next_nonce(&self) -> u64 {
        self.nonce.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    pub fn cache_path(&self, path: impl AsRef<Path>) -> PathBuf {
        self.cache_dir.join(path)
    }

    pub fn forge_create(&self, contract_spec: ContractSpec) -> ForgeCreate {
        let mut forge_create = ForgeCreate::new(contract_spec)
            .with_private_key(self.private_key.clone())
            .with_rpc_url(self.rpc_url.to_string())
            .with_override_nonce(self.next_nonce());

        if let Some(etherscan_api_key) = self.etherscan_api_key.as_ref() {
            forge_create = forge_create
                .with_verification_api_key(etherscan_api_key.clone());
        }

        forge_create
    }

    pub fn forge_verify(
        &self,
        contract_spec: ContractSpec,
        address: Address,
    ) -> ForgeVerify {
        let forge_verify = ForgeVerify::new(contract_spec, address)
            .with_etherscan_api_key(self.etherscan_api_key.clone().unwrap());

        forge_verify
    }
}

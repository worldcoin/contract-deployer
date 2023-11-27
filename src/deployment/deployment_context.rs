use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use ethers::types::Address;
use reqwest::Url;

use crate::cli::{Args, PrivateKey};
use crate::common_keys::RpcSigner;
use crate::forge_utils::verify::ForgeVerify;
use crate::forge_utils::{ContractSpec, ForgeCreate};
use crate::report::Report;

#[derive(Debug)]
pub struct DeploymentContext {
    pub deployment_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub nonce: AtomicU64,
    pub report: Report,
    pub private_key: PrivateKey,
    pub rpc_signer: Arc<RpcSigner>,
    pub rpc_url: Url,
    pub etherscan_api_key: Option<String>,
    pub cmd: Args,
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

        if let Some(verifier) = self.cmd.verifier.as_ref() {
            forge_create = forge_create.with_verifier(verifier.clone());
        }

        if let Some(verifier_url) = self.cmd.verifier_url.as_ref() {
            forge_create = forge_create.with_verifier_url(verifier_url.clone());
        }

        forge_create
    }

    pub fn forge_verify(
        &self,
        contract_spec: ContractSpec,
        address: Address,
    ) -> ForgeVerify {
        ForgeVerify::new(contract_spec, address)
            .with_etherscan_api_key(self.etherscan_api_key.clone().unwrap())
    }
}

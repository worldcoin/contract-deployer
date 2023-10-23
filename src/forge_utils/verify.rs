use std::path::PathBuf;

use ethers::types::Address;
use eyre::ContextCompat;
use tracing::{info, instrument};

use super::ContractSpec;

pub struct ForgeVerify {
    spec: ContractSpec,
    address: Address,
    root: Option<PathBuf>,
    chain: Option<u64>,
    etherscan_api_key: Option<String>,
}

impl ForgeVerify {
    pub fn new(spec: ContractSpec, address: Address) -> Self {
        Self {
            spec,
            address,
            root: None,
            chain: None,
            etherscan_api_key: None,
        }
    }

    pub fn with_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.root = Some(root.into());
        self
    }

    pub fn with_chain(mut self, chain: u64) -> Self {
        self.chain = Some(chain);
        self
    }

    pub fn with_etherscan_api_key(
        mut self,
        etherscan_api_key: impl ToString,
    ) -> Self {
        self.etherscan_api_key = Some(etherscan_api_key.to_string());
        self
    }

    #[instrument(name = "forge_verify", skip_all)]
    pub async fn run(&self) -> eyre::Result<()> {
        let mut cmd = tokio::process::Command::new("forge");
        cmd.arg("verify-contract");

        cmd.arg("--watch");
        cmd.arg("--flatten");

        let root = self.root.as_ref().context("Missing root")?;

        cmd.arg("--root");
        cmd.arg(root);

        let chain = self.chain.as_ref().context("Missing chain")?;

        cmd.arg("--chain");
        cmd.arg(chain.to_string());

        let etherscan_api_key = self
            .etherscan_api_key
            .as_ref()
            .context("Missing etherscan api key")?;

        cmd.arg("--etherscan-api-key");
        cmd.arg(etherscan_api_key);

        cmd.arg(format!("{:?}", self.address));
        cmd.arg(self.spec.to_string());

        info!("Verifying contract with {cmd:#?}");

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eyre::bail!("forge verify failed: {}", stderr);
        }

        Ok(())
    }
}

use std::path::{Path, PathBuf};

use tracing::instrument;

use super::common::ContractSpec;

pub struct ForgeInspectAbi {
    cwd: Option<PathBuf>,
    contract_spec: ContractSpec,
    override_contract_source: Option<PathBuf>,
}

impl ForgeInspectAbi {
    pub fn new(contract_spec: ContractSpec) -> Self {
        Self {
            cwd: None,
            contract_spec,
            override_contract_source: None,
        }
    }

    pub fn with_cwd(mut self, cwd: impl AsRef<Path>) -> Self {
        self.cwd = Some(cwd.as_ref().to_owned());
        self
    }

    pub fn with_contract_spec(mut self, contract_spec: ContractSpec) -> Self {
        self.contract_spec = contract_spec;
        self
    }

    pub fn with_override_contract_source(
        mut self,
        override_contract_source: impl AsRef<Path>,
    ) -> Self {
        self.override_contract_source =
            Some(override_contract_source.as_ref().to_owned());
        self
    }

    #[instrument(name = "forge_inspect_abi", skip_all)]
    pub async fn run(&self) -> eyre::Result<ethers::abi::Abi> {
        let mut cmd = tokio::process::Command::new("forge");

        cmd.arg("inspect");

        if let Some(cwd) = &self.cwd {
            cmd.current_dir(cwd);
        }

        if let Some(override_contract_source) = &self.override_contract_source {
            // TODO: Make the path relative to the working directory
            cmd.arg("-C");
            cmd.arg(override_contract_source);
        }

        cmd.arg(self.contract_spec.to_string());

        cmd.arg("abi");

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(eyre::eyre!("forge create failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        Ok(serde_json::from_str(&stdout)?)
    }
}

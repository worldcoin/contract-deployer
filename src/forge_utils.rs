use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use ethers::types::{Address, H256};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ContractSpec {
    pub path: PathBuf,
    pub name: String,
}

impl ContractSpec {
    pub fn new(path: PathBuf, name: impl ToString) -> Self {
        Self {
            path,
            name: name.to_string(),
        }
    }
}

pub struct ExternalDep {
    pub contract_spec: ContractSpec,
    pub address: Address,
}

impl fmt::Display for ContractSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.path.display(), self.name)
    }
}

impl fmt::Display for ExternalDep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.contract_spec, self.address)
    }
}

pub struct ForgeCreate {
    cwd: Option<PathBuf>,
    contract_spec: ContractSpec,
    override_contract_source: Option<PathBuf>,
    verify: bool,
    private_key: Option<String>,
    rpc_url: Option<String>,
    external_deps: Vec<ExternalDep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForgeOutput {
    pub deployer: Address,
    pub deployed_to: Address,
    pub transaction_hash: H256,
}

impl ForgeCreate {
    pub fn new(contract_spec: ContractSpec) -> Self {
        Self {
            cwd: None,
            contract_spec,
            override_contract_source: None,
            verify: false,
            private_key: None,
            rpc_url: None,
            external_deps: vec![],
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
        self.override_contract_source = Some(override_contract_source.as_ref().to_owned());
        self
    }

    pub fn with_verify(mut self, verify: bool) -> Self {
        self.verify = verify;
        self
    }

    pub fn with_private_key(mut self, private_key: String) -> Self {
        self.private_key = Some(private_key);
        self
    }

    pub fn with_rpc_url(mut self, rpc_url: String) -> Self {
        self.rpc_url = Some(rpc_url);
        self
    }

    pub fn with_external_dep(mut self, external_dep: ExternalDep) -> Self {
        self.external_deps.push(external_dep);
        self
    }

    pub async fn run(&self) -> eyre::Result<ForgeOutput> {
        let mut cmd = tokio::process::Command::new("forge");
        cmd.arg("create");

        if let Some(cwd) = &self.cwd {
            cmd.current_dir(cwd);
        }

        if let Some(override_contract_source) = &self.override_contract_source {
            // TODO: Make the path relative to the working directory
            cmd.arg("-C");
            cmd.arg(override_contract_source);
        }

        cmd.arg(self.contract_spec.to_string());

        if let Some(private_key) = &self.private_key {
            cmd.arg("--private-key");
            cmd.arg(private_key);
        }

        if let Some(rpc_url) = &self.rpc_url {
            cmd.arg("--rpc-url");
            cmd.arg(rpc_url);
        }

        if self.verify {
            cmd.arg("--verify");
        }

        cmd.arg("--json");

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(eyre::eyre!("forge create failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        Ok(serde_json::from_str(&stdout)?)
    }
}

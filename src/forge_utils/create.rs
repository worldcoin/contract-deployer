use std::path::{Path, PathBuf};

use ethers::types::{Address, H256};
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use super::common::{ContractSpec, ExternalDep};
use crate::cli::PrivateKey;

#[derive(Debug)]
pub struct ForgeCreate {
    cwd: Option<PathBuf>,
    contract_spec: ContractSpec,
    override_contract_source: Option<PathBuf>,
    private_key: Option<PrivateKey>,
    rpc_url: Option<String>,
    external_deps: Vec<ExternalDep>,
    override_nonce: Option<u64>,
    constructor_args: Vec<String>,
    verification_args: ForgeCreateVerificationArgs,
    no_verify: bool,
}

#[derive(Debug)]
pub struct ForgeCreateVerificationArgs {
    pub verification_api_key: Option<String>,
    pub verifier: Option<String>,
    pub verifier_url: Option<String>,
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
            override_nonce: None,
            private_key: None,
            rpc_url: None,
            external_deps: vec![],
            constructor_args: vec![],
            verification_args: ForgeCreateVerificationArgs {
                verification_api_key: None,
                verifier: None,
                verifier_url: None,
            },
            no_verify: false,
        }
    }

    pub fn no_verify(mut self) -> Self {
        self.no_verify = true;
        self
    }

    pub fn with_verification_api_key(
        mut self,
        verification_api_key: impl ToString,
    ) -> Self {
        self.verification_args.verification_api_key =
            Some(verification_api_key.to_string());
        self
    }

    pub fn with_verifier(mut self, verifier: impl ToString) -> Self {
        self.verification_args.verifier = Some(verifier.to_string());
        self
    }

    pub fn with_verifier_url(mut self, verifier_url: impl ToString) -> Self {
        self.verification_args.verifier_url = Some(verifier_url.to_string());
        self
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

    pub fn with_override_nonce(mut self, override_nonce: u64) -> Self {
        self.override_nonce = Some(override_nonce);
        self
    }

    pub fn with_private_key(mut self, private_key: PrivateKey) -> Self {
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

    pub fn with_constructor_arg(mut self, arg: impl ToString) -> Self {
        self.constructor_args.push(arg.to_string());
        self
    }

    #[instrument(name = "forge_create", skip_all)]
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

        if !self.external_deps.is_empty() {
            let mut external_deps = Vec::new();

            for external_dep in &self.external_deps {
                external_deps.push(external_dep.to_string());
            }

            let external_deps = external_deps.join(",");

            info!("external_deps = {external_deps}");

            cmd.arg("--libraries");
            cmd.arg(external_deps);
        }

        if let Some(private_key) = &self.private_key {
            cmd.arg("--private-key");
            cmd.arg(format!("{private_key:#}"));
        }

        if let Some(rpc_url) = &self.rpc_url {
            cmd.arg("--rpc-url");
            cmd.arg(rpc_url);
        }

        if let Some(nonce) = self.override_nonce {
            cmd.arg("--nonce");
            cmd.arg(nonce.to_string());
        }

        for constructor_arg in &self.constructor_args {
            cmd.arg("--constructor-args");
            cmd.arg(constructor_arg);
        }

        if !self.no_verify {
            let mut should_verify = false;

            if let Some(verification_api_key) =
                &self.verification_args.verification_api_key
            {
                should_verify = true;

                cmd.arg("--etherscan-api-key");
                cmd.arg(verification_api_key);
            }

            if let Some(verifier) = &self.verification_args.verifier {
                should_verify = true;

                cmd.arg("--verifier");
                cmd.arg(verifier);
            }

            if let Some(verifier_url) = &self.verification_args.verifier_url {
                should_verify = true;

                cmd.arg("--verifier-url");
                cmd.arg(verifier_url);
            }

            if should_verify {
                cmd.arg("--verify");
            }
        }

        cmd.arg("--json");

        info!("Creating contract with {cmd:#?}");

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eyre::bail!("forge create failed: {}", stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let s = strip_non_json(&stdout);

        let output = serde_json::from_str(s)?;

        info!("Created: {output:?}");

        Ok(output)
    }
}

fn strip_non_json(s: &str) -> &str {
    if let Some(last_closing_brace) = s.rfind('}') {
        &s[..=last_closing_brace]
    } else {
        s
    }
}

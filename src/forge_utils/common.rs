use std::fmt;
use std::path::{Path, PathBuf};

use ethers::types::Address;

#[derive(Debug, Clone)]
pub struct ContractSpec {
    pub path: Option<PathBuf>,
    pub name: String,
}

impl ContractSpec {
    pub fn path_name(path: PathBuf, name: impl ToString) -> Self {
        Self {
            path: Some(path),
            name: name.to_string(),
        }
    }

    pub fn name(name: impl ToString) -> Self {
        Self {
            path: None,
            name: name.to_string(),
        }
    }
}

#[derive(Debug)]
pub struct ExternalDep {
    pub contract_spec: ContractSpec,
    pub address: Address,
}

impl ExternalDep {
    pub fn path_name_address(
        path: impl AsRef<Path>,
        name: impl ToString,
        address: Address,
    ) -> Self {
        Self {
            contract_spec: ContractSpec::path_name(
                path.as_ref().to_owned(),
                name,
            ),
            address,
        }
    }

    pub fn name_address(name: impl ToString, address: Address) -> Self {
        Self {
            contract_spec: ContractSpec::name(name),
            address,
        }
    }
}

impl fmt::Display for ContractSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(path) = self.path.as_deref() {
            write!(f, "{}:{}", path.display(), self.name)
        } else {
            write!(f, "{}", self.name)
        }
    }
}

impl fmt::Display for ExternalDep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{:?}", self.contract_spec, self.address)
    }
}

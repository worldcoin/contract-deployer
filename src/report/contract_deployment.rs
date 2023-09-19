use ethers::types::Address;
use serde::{Deserialize, Serialize};

use crate::forge_utils::ForgeOutput;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ContractDeployment {
    pub address: Address,
}

impl From<ForgeOutput> for ContractDeployment {
    fn from(value: ForgeOutput) -> Self {
        Self {
            address: value.deployed_to,
        }
    }
}

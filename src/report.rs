use serde::{Deserialize, Serialize};

use crate::deployment::steps::identity_manager::WorldIDIdentityManagersDeployment;
use crate::deployment::steps::insertion_verifier::InsertionVerifiers;
use crate::deployment::steps::lookup_tables::LookupTables;
use crate::deployment::steps::semaphore_verifier::SemaphoreVerifierDeployment;
use crate::deployment::steps::world_id_router::WorldIdRouterDeployment;
use crate::Config;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    pub config: Config,

    #[serde(default)]
    pub verifiers: InsertionVerifiers,

    #[serde(default)]
    pub lookup_tables: LookupTables,

    #[serde(default)]
    pub semaphore_verifier: Option<SemaphoreVerifierDeployment>,

    #[serde(default)]
    pub identity_managers: WorldIDIdentityManagersDeployment,

    #[serde(default)]
    pub world_id_router: Option<WorldIdRouterDeployment>,
}

impl Report {
    pub fn default_with_config(config: &Config) -> Self {
        Self {
            config: config.clone(),
            verifiers: Default::default(),
            lookup_tables: Default::default(),
            semaphore_verifier: Default::default(),
            identity_managers: Default::default(),
            world_id_router: Default::default(),
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use ethers::types::{Address, H256};

//     use super::*;
//     use crate::forge_utils::ForgeOutput;
//     use crate::insertion_verifier::InsertionVerifiersForGroup;
//     use crate::lookup_tables::{
//         InsertLookupTable, LookupTableForBatchSize, LookupTableForGroup,
//         UpdateLookupTable,
//     };
//     use crate::types::{BatchSize, GroupId, TreeDepth};

//     #[test]
//     fn whatever() {
//         let report = Report {
//             config: Config {
//                 groups: maplit::hashmap! {
//                     GroupId(0) => crate::GroupConfig {
//                         tree_depth: TreeDepth(16),
//                         batch_sizes: vec![BatchSize(3)]
//                     }
//                 },
//                 misc: crate::MiscConfig {
//                     initial_leaf_value: H256::default(),
//                 },
//             },
//             verifiers: InsertionVerifiers {
//                 groups: maplit::hashmap! {
//                     GroupId(0) => InsertionVerifiersForGroup {
//                         batch_sizes: maplit::hashmap! {
//                             BatchSize(3) => ForgeOutput {
//                                 deployed_to: Address::default(),
//                                 deployer: Address::default(),
//                                 transaction_hash: H256::default(),
//                             },
//                         }
//                     }
//                 },
//             },
//             lookup_tables: LookupTables {
//                 groups: maplit::hashmap! {
//                     GroupId(0) => LookupTableForGroup {
//                         batch_sizes: maplit::hashmap! {
//                             BatchSize(3) => LookupTableForBatchSize {
//                                 insert: InsertLookupTable {
//                                     deploy_info: ForgeOutput {
//                                         deployed_to: Address::default(),
//                                         deployer: Address::default(),
//                                         transaction_hash: H256::default(),
//                                     },
//                                     entries: maplit::hashmap! {
//                                         BatchSize(3) => Address::default(),
//                                     },
//                                 },
//                                 update: UpdateLookupTable {
//                                     deploy_info: ForgeOutput {
//                                         deployed_to: Address::default(),
//                                         deployer: Address::default(),
//                                         transaction_hash: H256::default(),
//                                     }
//                                 },
//                             }
//                         }
//                     }
//                 },
//             },
//         };

//         let pretty = serde_yaml::to_string(&report).unwrap();

//         println!("{}", pretty);

//         panic!();
//     }
// }

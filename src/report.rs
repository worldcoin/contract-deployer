use serde::{Deserialize, Serialize};

use crate::insertion_verifier::InsertionVerifiers;
// use crate::lookup_tables::LookupTables;
use crate::Config;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    config: Config,

    verifiers: InsertionVerifiers,
    // lookup_tables: LookupTables,
}

impl Report {
    pub fn default_with_config(config: &Config) -> Self {
        Self {
            config: config.clone(),
            verifiers: Default::default(),
            // lookup_tables: Default::default(),
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

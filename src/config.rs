use std::collections::{HashMap, HashSet};

use ethers::types::H256;
use serde::{Deserialize, Serialize};

use crate::deployment::mtb_utils::ProverMode;
use crate::types::{BatchSize, GroupId, TreeDepth};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub groups: HashMap<GroupId, GroupConfig>,
    pub misc: MiscConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiscConfig {
    #[serde(default)]
    pub initial_leaf_value: H256,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupConfig {
    pub tree_depth: TreeDepth,
    /// Which batch sizes are supported for insertion by this group
    #[serde(alias = "batch_sizes")] // For backwards compatibility
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insertion_batch_sizes: Option<Vec<BatchSize>>,
    /// Which batch sizes are supported for deletion by this group
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deletion_batch_sizes: Option<Vec<BatchSize>>,
    /// Allows overriding the initial root constructor arg
    #[serde(default)]
    pub initial_root: Option<H256>,
}

impl Config {
    pub fn unique_tree_depths_and_batch_sizes(
        &self,
        mode: ProverMode,
    ) -> HashSet<(TreeDepth, BatchSize)> {
        let mut result = HashSet::new();

        for group in self.groups.values() {
            let batch_sizes_for_mode = match mode {
                ProverMode::Insertion => {
                    group.insertion_batch_sizes.as_ref().unwrap()
                }
                ProverMode::Deletion => {
                    group.deletion_batch_sizes.as_ref().unwrap()
                }
            };

            for batch_size in batch_sizes_for_mode {
                result.insert((group.tree_depth, *batch_size));
            }
        }

        result
    }
}

use std::collections::{HashMap, HashSet};

use ethers::types::H256;
use serde::{Deserialize, Serialize};

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
    pub batch_sizes: Vec<BatchSize>,
}

impl Config {
    pub fn unique_tree_depths_and_batch_sizes(
        &self,
    ) -> HashSet<(TreeDepth, BatchSize)> {
        let mut result = HashSet::new();

        for group in self.groups.values() {
            for batch_size in &group.batch_sizes {
                result.insert((group.tree_depth, *batch_size));
            }
        }

        result
    }
}

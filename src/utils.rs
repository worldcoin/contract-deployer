use ethers::types::H256;
use semaphore::poseidon_tree::LazyPoseidonTree;
use semaphore::Field;

pub fn initial_root_hash(tree_depth: usize, initial_leaf_value: H256) -> H256 {
    let initial_leaf_value = Field::from_be_bytes(initial_leaf_value.0);

    let initial_root_hash =
        LazyPoseidonTree::new(tree_depth, initial_leaf_value).root();

    H256(initial_root_hash.to_be_bytes())
}

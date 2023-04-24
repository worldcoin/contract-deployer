use std::sync::Arc;

use ethers::prelude::*;
use ethers::types::H256;

pub struct InitialRoot(pub H256);

// TODO: Allow for different wallet kinds
pub struct RpcSigner(pub Arc<SignerMiddleware<Provider<Http>, LocalWallet>>);

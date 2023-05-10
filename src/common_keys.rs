use std::sync::Arc;

use ethers::prelude::*;

// TODO: Allow for different wallet kinds
#[derive(Debug, Clone)]
pub struct RpcSigner(pub Arc<SignerMiddleware<Provider<Http>, LocalWallet>>);

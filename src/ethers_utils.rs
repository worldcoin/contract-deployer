use std::sync::Arc;

use ethers::abi::Tokenizable;
use ethers::prelude::encode_function_data;
use ethers::providers::Middleware;
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::{Address, Eip1559TransactionRequest};
use eyre::{bail, Context, ContextCompat};

use crate::common_keys::RpcSigner;
use crate::DeploymentContext;

pub struct Transaction<'a, T> {
    context: &'a DeploymentContext,
    abi: ethers::abi::Abi,
    function_name: String,
    args: T,
    signer: Arc<RpcSigner>,
    to: Address,
}

#[derive(Default, Clone, Debug)]
pub struct TransactionBuilder<'a, T> {
    context: Option<&'a DeploymentContext>,
    abi: Option<ethers::abi::Abi>,
    function_name: Option<String>,
    args: Option<T>,
    signer: Option<Arc<RpcSigner>>,
    to: Option<Address>,
}

impl<'a, T> TransactionBuilder<'a, T> {
    pub fn context(mut self, context: &'a DeploymentContext) -> Self {
        self.context = Some(context);
        self
    }

    pub fn abi(mut self, abi: ethers::abi::Abi) -> Self {
        self.abi = Some(abi);
        self
    }

    pub fn function_name(mut self, function_name: impl ToString) -> Self {
        self.function_name = Some(function_name.to_string());
        self
    }

    pub fn args(mut self, args: T) -> Self {
        self.args = Some(args);
        self
    }

    pub fn signer(mut self, signer: Arc<RpcSigner>) -> Self {
        self.signer = Some(signer);
        self
    }

    pub fn to(mut self, to: Address) -> Self {
        self.to = Some(to);
        self
    }

    pub fn build(self) -> eyre::Result<Transaction<'a, T>> {
        Ok(Transaction {
            context: self
                .context
                .context("TransactionBuilder missing context")?,
            abi: self.abi.context("TransactionBuilder missing abi")?,
            function_name: self
                .function_name
                .context("TransactionBuilder missing function_name")?,
            args: self.args.context("TransactionBuilder missing args")?,
            signer: self.signer.context("TransactionBuilder missing signer")?,
            to: self.to.context("TransactionBuilder missing to")?,
        })
    }
}

impl<'a, T> Transaction<'a, T>
where
    T: Tokenizable,
{
    pub async fn send(self) -> eyre::Result<()> {
        let func = self.abi.function(&self.function_name)?;
        let call_data = encode_function_data(func, self.args)?;

        let mut tx = TypedTransaction::Eip1559(
            Eip1559TransactionRequest::new()
                .to(self.to)
                .data(call_data)
                .nonce(self.context.next_nonce()),
        );

        self.signer.0.fill_transaction(&mut tx, None).await?;

        let tx = self
            .signer
            .0
            .send_transaction(tx, None)
            .await
            .context("Send transaction")?;

        let receipt = tx
            .await
            .context("Awaiting receipt")?
            .context("Failed to execute")?;

        if receipt.status != Some(1.into()) {
            bail!("Failed!");
        }

        Ok(())
    }
}

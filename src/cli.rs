use std::path::PathBuf;

use clap::{Parser, ValueEnum};

pub mod private_key;

pub use private_key::PrivateKey;
use reqwest::Url;

#[derive(Debug, Clone, ValueEnum, PartialEq)]
#[clap(rename_all = "kebab-case")]
pub enum DeploymentType {
    Full,
    SemaphoreVerifier,
    InsertionVerifiers,
    DeletionVerifiers,
    Verifiers,
    LookupTables,
    WorldIdRouter,
    IdentityManager,
}

#[derive(Debug, Clone, Parser)]
#[clap(rename_all = "kebab-case")]
pub struct Args {
    #[clap(short, long, env, default_value = "full")]
    pub target: DeploymentType,

    /// Path to the deployment configuration file
    #[clap(short, long, env)]
    pub config: PathBuf,

    /// The name of the deployment
    ///
    /// Should be something meaningful like 'prod-2023-04-18'
    #[clap(short, long, env)]
    pub deployment_name: String,

    /// Private key to use for the deployment
    #[clap(short, long, env)]
    pub private_key: PrivateKey,

    /// The RPC Url to use for the deployment
    #[clap(short, long, env)]
    pub rpc_url: Url,

    /// The etherscan API key to use
    #[clap(short, long, env)]
    pub etherscan_api_key: Option<String>,

    /// The verifier to use for the deployment - blockscout, etherscan
    #[clap(long, env)]
    pub verifier: Option<String>,

    /// The verifier url to use for the deployment
    /// For blockscout, this is the url to the blockscout instance api
    /// For etherscan, this is the url to the etherscan api
    #[clap(long, env)]
    pub verifier_url: Option<String>,

    /// Cache directory
    #[clap(long, env, default_value = ".cache")]
    pub cache_dir: PathBuf,
}

use std::path::PathBuf;

use clap::Parser;

pub mod private_key;

pub use private_key::PrivateKey;
use reqwest::Url;

#[derive(Debug, Clone, Parser)]
#[clap(rename_all = "kebab-case")]
pub struct Args {
    /// Path to the deployment configuration file
    #[clap(short, long, env)]
    pub config: Option<PathBuf>,

    /// The name of the deployment
    ///
    /// Should be something meaningful like 'prod-2023-04-18'
    #[clap(short, long, env)]
    pub deployment_name: Option<String>,

    /// Private key to use for the deployment
    #[clap(short, long, env)]
    pub private_key: Option<PrivateKey>,

    /// The RPC Url to use for the deployment
    #[clap(short, long, env)]
    pub rpc_url: Option<Url>,

    /// The etherscan API key to use
    #[clap(short, long, env)]
    pub etherscan_api_key: Option<String>,

    /// Cache directory
    #[clap(long, env, default_value = ".cache")]
    pub cache_dir: PathBuf,
}

use std::fmt;
use std::path::PathBuf;

use clap::Parser;
use eyre::ContextCompat;
use reqwest::Url;

use crate::args::{DeploymentArgs, PrivateKey};
use crate::assemble_report::REPORT_PATH;
use crate::config::Config;
use crate::{serde_utils, Cmd};

#[derive(Clone, Debug)]
enum MainMenu {
    Rename,
    Proceed,
}

impl fmt::Display for MainMenu {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Rename => write!(f, "Rename"),
            Self::Proceed => write!(f, "Proceed"),
        }
    }
}

#[derive(Debug, Clone, Parser)]
#[clap(rename_all = "kebab-case")]
pub struct InteractiveCmd {
    /// Path to the deployment configuration file
    #[clap(short, long, env)]
    pub config: Option<PathBuf>,

    /// Run in interactive mode
    ///
    /// NOTE: If not running in interactive mode ALL the values must be provided
    #[clap(short, long)]
    pub interactive: bool,

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
}

impl TryFrom<InteractiveCmd> for Cmd {
    type Error = eyre::Error;

    fn try_from(value: InteractiveCmd) -> Result<Self, Self::Error> {
        Ok(Self {
            config: value.config.context("Missing context")?,
            deployment_name: value
                .deployment_name
                .context("Missing deployment name")?,
            args: DeploymentArgs {
                private_key: value
                    .private_key
                    .context("Missing private key")?,
                rpc_url: value.rpc_url.context("Missing rpc url")?,
            },
        })
    }
}

pub async fn run_interactive_session(
    mut cmd: InteractiveCmd,
) -> eyre::Result<Cmd> {
    if let Some(name) = cmd.deployment_name.as_ref() {
        println!("Currently working on deployment: {}", name);
    } else {
        cmd.deployment_name =
            Some(inquire::Text::new("Deployment name:").prompt()?);
    }

    if let Some(private_key) = cmd.private_key.as_ref() {
        println!("Using private key: {private_key}");
    } else {
        let private_key = inquire::Text::new("Private key:").prompt()?;
        cmd.private_key = Some(private_key.parse()?);
    }

    if let Some(rpc_url) = cmd.rpc_url.as_ref() {
        println!("Using RPC: {rpc_url}");
    } else {
        let rpc_url = inquire::Text::new("Rpc Url:").prompt()?;
        cmd.rpc_url = Some(rpc_url.parse()?);
    }

    if let Some(config) = cmd.config.as_ref() {
        println!("Using config at: {}", config.display());
    } else {
        let config_path = inquire::Text::new("Path to config (leave empty to create):").prompt()?;
        cmd.config = Some(config_path.parse()?);
    }

    loop {
        let config: Config = serde_utils::read_deserialize(
            &cmd.config.as_ref().context("Missing config")?,
        )
        .await?;

        let deployment_name = cmd
            .deployment_name
            .as_ref()
            .context("Missing deployment name")?;

        let deployment_dir = PathBuf::from(deployment_name);

        let cache_dir = deployment_dir.join(".cache");

        let report_path = deployment_dir.join(REPORT_PATH);

        if report_path.exists() {
        } else {
            print_deployment_info(&deployment_name, &config);
            let proceed = inquire::Confirm::new(
                "No report found, do you want to proceed with this deployment?",
            )
            .prompt()?;

            if !proceed {
                std::process::exit(0);
            }

            return Ok(cmd.try_into()?);
        }

        if let Some(name) = cmd.deployment_name.as_ref() {
            println!("Currently working on deployment: {}", name);
        }

        match inquire::Select::new(
            "Menu:",
            vec![MainMenu::Rename, MainMenu::Proceed],
        )
        .prompt_skippable()?
        {
            Some(MainMenu::Rename) => {
                let Some(new_name) = inquire::Text::new("New name").prompt_skippable()? else {
                    continue;
                };

                cmd.deployment_name = Some(new_name);
            }
            Some(MainMenu::Proceed) => break,
            None => std::process::exit(0),
        }
    }

    Ok(cmd.try_into()?)
}

fn print_deployment_info(name: &str, config: &Config) {
    println!("Deployment: {name}");
    println!("Groups:");
    for (group_id, group) in &config.groups {
        println!("Group #{}", group_id);
        println!("  Tree depth: {}", group.tree_depth);
        println!("  Batch sizes: ");
        for batch_size in &group.batch_sizes {
            println!("    {}", batch_size);
        }
    }
}

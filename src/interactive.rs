use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use clap::Parser;
use ethers::types::H256;
use eyre::ContextCompat;
use reqwest::Url;

use crate::args::{DeploymentArgs, PrivateKey};
use crate::assemble_report::REPORT_PATH;
use crate::config::{Config, GroupConfig, MiscConfig};
use crate::report::Report;
use crate::types::{BatchSize, GroupId, TreeDepth};
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
    let deployment_name = if let Some(name) = cmd.deployment_name.as_ref() {
        println!("Currently working on deployment: {}", name);
        name.clone()
    } else {
        inquire::Text::new("Deployment name:").prompt()?
    };

    let private_key = if let Some(private_key) = cmd.private_key.as_ref() {
        println!("Using private key: {private_key}");
        private_key.clone()
    } else {
        let private_key = inquire::Text::new("Private key:").prompt()?;
        private_key.parse()?
    };

    let rpc_url = if let Some(rpc_url) = cmd.rpc_url.as_ref() {
        println!("Using RPC: {rpc_url}");
        rpc_url.clone()
    } else {
        let rpc_url = inquire::Text::new("Rpc Url:").prompt()?;
        rpc_url.parse()?
    };

    let config_path = if let Some(config) = cmd.config.as_ref() {
        println!("Using config at: {}", config.display());
        config.clone()
    } else {
        let config_path =
            inquire::Text::new("Path to config (leave empty to create):")
                .prompt()?;

        if config_path.trim().is_empty() {
            create_config_interactive().await?
        } else {
            config_path.parse()?
        }
    };

    loop {
        let config: Config =
            serde_utils::read_deserialize(&config_path).await?;

        let deployment_dir = PathBuf::from(&deployment_name);

        let report_path = deployment_dir.join(REPORT_PATH);

        if !report_path.exists() {
            println!("Deployment name: {deployment_name}");
            print_deployment_info(&config);

            let proceed = inquire::Confirm::new(
                "No report found, do you want to proceed with this deployment?",
            )
            .prompt()?;

            if !proceed {
                std::process::exit(0);
            }

            return Ok(Cmd::new(
                config_path,
                deployment_name.clone(),
                private_key,
                rpc_url,
            ));
        }

        let report: Report =
            serde_utils::read_deserialize(&report_path).await?;

        println!("Deployment name: {deployment_name}");
        print_deployment_diff(&config, &report);

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

fn print_deployment_info(config: &Config) {
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

fn print_deployment_diff(config: &Config, report: &Report) {
    println!("Groups:");
    if let Some(world_id_router) = report.world_id_router.as_ref() {
        println!(
            "Router address: {:?}",
            world_id_router.proxy_deployment.deployed_to
        );
    }

    for (group_id, group) in &config.groups {
        println!("  Group #{}", group_id);
        if let Some(group_report) =
            report.identity_managers.groups.get(group_id)
        {
            println!(
                "    Identity manager: {:?}",
                group_report.proxy_deployment.deployed_to
            );
        }

        println!("    Tree depth: {}", group.tree_depth);

        if let Some(lookup_table_group) =
            report.lookup_tables.groups.get(group_id)
        {
            let lookup_table_address =
                lookup_table_group.insert.deployment.deployed_to;
            println!("    Insert lookup table: {:?}", lookup_table_address);

            for batch_size in &group.batch_sizes {
                if let Some(verifier) =
                    lookup_table_group.insert.entries.get(batch_size)
                {
                    println!("      Batch size {}: {:?}", batch_size, verifier);
                } else {
                    println!("      Batch size {}: (undeployed)", batch_size);
                }
            }
        }
    }
}

enum CreateConfigMenu {
    AddGroup,
    RemoveGroup,
    Proceed,
}

impl fmt::Display for CreateConfigMenu {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CreateConfigMenu::AddGroup => write!(f, "Add group"),
            CreateConfigMenu::RemoveGroup => write!(f, "Remove group"),
            CreateConfigMenu::Proceed => write!(f, "Proceed"),
        }
    }
}

async fn create_config_interactive() -> eyre::Result<PathBuf> {
    let config_path = loop {
        let filename = inquire::Text::new("Config filename:").prompt()?;

        let config_path = PathBuf::from(filename);

        if config_path.exists() {
            let overwrite =
                inquire::Confirm::new("Overwrite existing file?").prompt()?;

            if !overwrite {
                continue;
            }
        }

        break config_path;
    };

    let mut config = Config {
        groups: HashMap::default(),
        misc: MiscConfig {
            initial_leaf_value: H256::zero(),
        },
    };

    loop {
        print_deployment_info(&config);

        let option = inquire::Select::new(
            "Menu (Esc to quit):",
            vec![
                CreateConfigMenu::AddGroup,
                CreateConfigMenu::RemoveGroup,
                CreateConfigMenu::Proceed,
            ],
        )
        .prompt_skippable()?;

        match option {
            Some(CreateConfigMenu::AddGroup) => {
                let group_id: GroupId = prompt_text_handle_errors("Group id:")?;

                let tree_depth: TreeDepth =
                    prompt_text_handle_errors("Tree depth:")?;

                let mut batch_sizes = vec![];

                while let Some(batch_size) =
                    prompt_text_skippable_handle_errors(
                        "Enter new batch size (Esc to finish):",
                    )?
                {
                    batch_sizes.push(batch_size);
                }

                let group = GroupConfig {
                    tree_depth,
                    batch_sizes,
                };

                config.groups.insert(group_id, group);
            }
            Some(CreateConfigMenu::RemoveGroup) => {
                let existing_groups =
                    config.groups.keys().copied().collect::<Vec<_>>();

                let Some(selected_groups) = inquire::MultiSelect::new(
                        "Select groups to remove:",
                        existing_groups,
                    )
                    .prompt_skippable()?
                else {
                    continue;
                };

                for group_id in selected_groups {
                    config.groups.remove(&group_id);
                }
            }
            Some(CreateConfigMenu::Proceed) => break,
            None => std::process::exit(0),
        }
    }

    crate::serde_utils::write_serialize(&config_path, config).await?;

    Ok(config_path)
}

fn prompt_text_handle_errors<T>(prompt: &str) -> eyre::Result<T>
where
    T: FromStr,
    <T as FromStr>::Err: std::error::Error,
{
    loop {
        let t = inquire::Text::new(prompt).prompt()?;

        match t.trim().parse() {
            Ok(t) => return Ok(t),
            Err(e) => {
                println!("Error: {}", e);
                continue;
            }
        }
    }
}

fn prompt_text_skippable_handle_errors<T>(
    prompt: &str,
) -> eyre::Result<Option<T>>
where
    T: FromStr,
    <T as FromStr>::Err: std::error::Error,
{
    loop {
        let t = inquire::Text::new(prompt).prompt_skippable()?;

        let Some(t) = t else {
            return Ok(None);
        };

        match t.trim().parse() {
            Ok(t) => return Ok(Some(t)),
            Err(e) => {
                println!("Error: {}", e);
                continue;
            }
        }
    }
}

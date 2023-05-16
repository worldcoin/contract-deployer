use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use self::create_config::create_config_interactive;
use crate::cli::Args;
use crate::config::Config;
use crate::deployment::steps::assemble_report::REPORT_PATH;
use crate::report::Report;
use crate::{serde_utils, Cmd};

mod create_config;

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

pub async fn run_interactive_session(mut cmd: Args) -> eyre::Result<Cmd> {
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

    Ok(Cmd::new(
        config_path,
        deployment_name.clone(),
        private_key,
        rpc_url,
    ))
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

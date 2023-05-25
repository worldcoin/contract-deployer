use std::path::PathBuf;

use self::create_config::create_config_interactive;
use crate::cli::Args;
use crate::config::Config;
use crate::deployment::steps::assemble_report::REPORT_PATH;
use crate::deployment::Cmd;
use crate::interactive::add_group::add_group;
use crate::report::Report;
use crate::serde_utils;

mod add_group;
mod create_config;
mod utils;

#[derive(Clone, Debug, derive_more::Display)]
enum MainMenu {
    #[display(fmt = "proceed")]
    Proceed,
    #[display(fmt = "Add group")]
    AddGroup,
    #[display(fmt = "Remove groups")]
    RemoveGroups,
}

pub async fn run_interactive_session(cmd: Args) -> eyre::Result<Cmd> {
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

    let etherscan_api_key =
        if let Some(etherscan_api_key) = cmd.etherscan_api_key.as_ref() {
            println!("Using Etherscan API key: {etherscan_api_key}");
            Some(etherscan_api_key.clone())
        } else {
            let etherscan_api_key =
                inquire::Text::new("Etherscan API key (leave empty to skip):")
                    .prompt()?;

            if etherscan_api_key.trim().is_empty() {
                None
            } else {
                Some(etherscan_api_key)
            }
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
        let mut config: Config =
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
                deployment_name,
                private_key,
                rpc_url,
                etherscan_api_key,
                cmd.cache_dir,
            ));
        }

        let mut report: Report =
            serde_utils::read_deserialize(&report_path).await?;

        println!("Deployment name: {deployment_name}");
        print_deployment_diff(&config, &report);

        if let Some(name) = cmd.deployment_name.as_ref() {
            println!("Currently working on deployment: {}", name);
        }

        match inquire::Select::new(
            "Menu:",
            vec![
                MainMenu::Proceed,
                MainMenu::AddGroup,
                MainMenu::RemoveGroups,
            ],
        )
        .prompt_skippable()?
        {
            Some(MainMenu::Proceed) => break,
            Some(MainMenu::AddGroup) => {
                let (group_id, group) = add_group()?;

                if config.groups.contains_key(&group_id) {
                    let should_replace = inquire::Confirm::new(&format!("Group with id {} already exists, do you want to replace it?", group_id)).prompt()?;
                    if !should_replace {
                        continue;
                    }
                }

                if config.groups.insert(group_id, group).is_some() {
                    // We must invalidate any previous deployment
                    report.invalidate_group_id(group_id);
                }
            }
            Some(MainMenu::RemoveGroups) => {
                let available_groups = config.groups.keys().copied().collect();

                if let Some(groups_to_remove) = inquire::MultiSelect::new(
                    "Select groups to remove",
                    available_groups,
                )
                .prompt_skippable()?
                {
                    for group_id in groups_to_remove {
                        config.groups.remove(&group_id);
                        report.invalidate_group_id(group_id);
                    }
                }
            }
            None => std::process::exit(0),
        }

        serde_utils::write_serialize(&config_path, &config).await?;
        serde_utils::write_serialize(&report_path, &report).await?;
    }

    Ok(Cmd::new(
        config_path,
        deployment_name.clone(),
        private_key,
        rpc_url,
        etherscan_api_key,
        cmd.cache_dir,
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
            world_id_router.proxy_deployment.address
        );
    }

    for (group_id, group) in &config.groups {
        println!("  Group #{}", group_id);
        if let Some(group_report) =
            report.identity_managers.groups.get(group_id)
        {
            println!(
                "    Identity manager: {:?}",
                group_report.proxy_deployment.address
            );
        } else {
            println!("    Identity manager: (undeployed)",);
        }

        println!("    Tree depth: {}", group.tree_depth);

        if let Some(lookup_table_group) =
            report.lookup_tables.groups.get(group_id)
        {
            let lookup_table_address =
                lookup_table_group.insert.deployment.address;
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
        } else {
            println!("    Insert lookup table: (undeployed)");
        }
    }
}

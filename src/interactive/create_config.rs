use std::collections::HashMap;
use std::path::PathBuf;

use derive_more::Display;
use ethers::types::H256;

use super::add_group::add_group;
use super::print_deployment_info;
use crate::config::{Config, MiscConfig};

#[derive(Debug, Clone, Copy, Display)]
enum CreateConfigMenu {
    #[display(fmt = "Add group")]
    AddGroup,
    #[display(fmt = "Remove group")]
    RemoveGroup,
    #[display(fmt = "Proceed")]
    Proceed,
}

pub async fn create_config_interactive() -> eyre::Result<PathBuf> {
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
                let (group_id, group) = add_group()?;

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

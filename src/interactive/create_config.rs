use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use ethers::types::H256;

use crate::config::{Config, GroupConfig, MiscConfig};
use crate::types::{GroupId, TreeDepth};

use super::{
    print_deployment_info, prompt_text_handle_errors,
    prompt_text_skippable_handle_errors,
};

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

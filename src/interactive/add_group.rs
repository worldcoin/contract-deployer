use super::utils::{
    prompt_text_handle_errors, prompt_text_skippable_handle_errors,
};
use crate::config::GroupConfig;
use crate::types::{GroupId, TreeDepth};

pub fn add_group() -> eyre::Result<(GroupId, GroupConfig)> {
    let group_id: GroupId = prompt_text_handle_errors("Group id:")?;

    let tree_depth: TreeDepth = prompt_text_handle_errors("Tree depth:")?;

    let mut batch_sizes = vec![];

    while let Some(batch_size) = prompt_text_skippable_handle_errors(
        "Enter new batch size (Esc to finish):",
    )? {
        batch_sizes.push(batch_size);
    }

    let group = GroupConfig {
        tree_depth,
        batch_sizes,
    };

    Ok((group_id, group))
}

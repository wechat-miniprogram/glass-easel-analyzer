use crate::wxss_utils::for_each_selector_in_style_sheet;

use super::*;

pub(super) fn find_declaration(project: &mut Project, abs_path: &Path, pos: lsp_types::Position) -> anyhow::Result<Vec<LocationLink>> {
    let sheet = project.get_style_sheet(abs_path)?;
    todo!()
}

pub(super) fn find_references(project: &mut Project, abs_path: &Path, pos: lsp_types::Position) -> anyhow::Result<Vec<Location>> {
    let sheet = project.get_style_sheet(abs_path)?;
    todo!()
}

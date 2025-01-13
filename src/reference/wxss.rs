use std::collections::HashSet;

use crate::{utils::location_to_lsp_range, wxss::{rule::Selector, Position, StyleSheet}, wxss_utils::{find_token_in_position, for_each_import_in_style_sheet, for_each_selector_in_style_sheet, Token}};

use super::*;

pub(crate) fn rec_imports_style_sheet(
    visited: &mut HashSet<PathBuf>,
    project: &Project,
    abs_path: &Path,
    sheet: &StyleSheet,
    f: &mut impl FnMut(&Path, &Selector),
) {
    visited.insert(abs_path.to_path_buf());
    for_each_import_in_style_sheet(sheet, |rel| {
        if let Ok(p) = project.find_rel_path_for_file(abs_path, rel) {
            if let Some(imported_path) = crate::utils::ensure_file_extension(&p, "wxss") {
                if let Ok(sheet) = project.get_style_sheet(&imported_path) {
                    rec_imports_style_sheet(visited, project, &imported_path, sheet, f);
                }
            }
        }
    });
    for_each_selector_in_style_sheet(sheet, |sel| f(abs_path, sel));
}

pub(super) fn find_declaration(project: &mut Project, abs_path: &Path, pos: lsp_types::Position) -> anyhow::Result<Vec<LocationLink>> {
    let sheet = project.get_style_sheet(abs_path)?;
    let mut ret = vec![];
    let token = find_token_in_position(sheet, Position { line: pos.line, utf16_col: pos.character });
    match token {
        Token::TagName(x) => {
            // returns itself - this will hint the editor to call find-reference for it
            ret.push(LocationLink {
                origin_selection_range: Some(location_to_lsp_range(&x.location)),
                target_uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                target_range: location_to_lsp_range(&x.location),
                target_selection_range: location_to_lsp_range(&x.location),
            });
        }
        Token::Id(x) => {
            // returns itself - this will hint the editor to call find-reference for it
            ret.push(LocationLink {
                origin_selection_range: Some(location_to_lsp_range(&x.location)),
                target_uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                target_range: location_to_lsp_range(&x.location),
                target_selection_range: location_to_lsp_range(&x.location),
            });
        }
        Token::Class(op, x) => {
            // returns itself - this will hint the editor to call find-reference for it
            let loc = op.location.start..x.location.end;
            ret.push(LocationLink {
                origin_selection_range: Some(location_to_lsp_range(&loc)),
                target_uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                target_range: location_to_lsp_range(&loc),
                target_selection_range: location_to_lsp_range(&loc),
            });
        }
        _ => {}
    }
    Ok(ret)
}

pub(super) fn find_references(project: &mut Project, abs_path: &Path, pos: lsp_types::Position) -> anyhow::Result<Vec<Location>> {
    let sheet = project.get_style_sheet(abs_path)?;
    let mut ret = vec![];
    let token = find_token_in_position(sheet, Position { line: pos.line, utf16_col: pos.character });
    match token {
        Token::TagName(x) => {
            rec_imports_style_sheet(&mut HashSet::new(), project, abs_path, sheet, &mut |abs_path, sel| {
                if let Selector::TagName(y) = sel {
                    if x.content == y.content {
                        ret.push(Location {
                            uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                            range: location_to_lsp_range(&y.location),
                        });
                    }
                }
            });
        }
        Token::Id(x) => {
            rec_imports_style_sheet(&mut HashSet::new(), project, abs_path, sheet, &mut |abs_path, sel| {
                if let Selector::Id(y) = sel {
                    if x.content == y.content {
                        ret.push(Location {
                            uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                            range: location_to_lsp_range(&y.location),
                        });
                    }
                }
            });
        }
        Token::Class(_, x) => {
            rec_imports_style_sheet(&mut HashSet::new(), project, abs_path, sheet, &mut |abs_path, sel| {
                if let Selector::Class(op, y) = sel {
                    if x.content == y.content {
                        ret.push(Location {
                            uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                            range: location_to_lsp_range(&(op.location.start..y.location.end)),
                        });
                    }
                }
            });
        }
        _ => {}
    }
    Ok(ret)
}

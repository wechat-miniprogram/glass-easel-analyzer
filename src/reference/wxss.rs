use crate::{utils::location_to_lsp_range, wxss::{rule::Selector, Position}, wxss_utils::{find_token_in_position, for_each_selector_in_style_sheet, Token}};

use super::*;

pub(super) fn find_declaration(project: &mut Project, abs_path: &Path, pos: lsp_types::Position) -> anyhow::Result<Vec<LocationLink>> {
    let sheet = project.get_style_sheet(abs_path)?;
    let mut ret = vec![];
    let token = find_token_in_position(sheet, Position { line: pos.line, utf16_col: pos.character });
    match token {
        Token::TagName(x) => {
            ret.push(LocationLink {
                origin_selection_range: Some(location_to_lsp_range(&x.location)),
                target_uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                target_range: location_to_lsp_range(&x.location),
                target_selection_range: location_to_lsp_range(&x.location),
            });
        }
        Token::Id(x) => {
            ret.push(LocationLink {
                origin_selection_range: Some(location_to_lsp_range(&x.location)),
                target_uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                target_range: location_to_lsp_range(&x.location),
                target_selection_range: location_to_lsp_range(&x.location),
            });
        }
        Token::Class(op, x) => {
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
            for_each_selector_in_style_sheet(sheet, |sel| {
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
            for_each_selector_in_style_sheet(sheet, |sel| {
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
            for_each_selector_in_style_sheet(sheet, |sel| {
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

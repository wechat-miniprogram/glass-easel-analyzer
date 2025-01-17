use glass_easel_template_compiler::parse::{tag::{Element, ElementKind, Ident, Node, StaticAttribute, TemplateDefinition, Value}, Position, Template, TemplateStructure};

use crate::{utils::{add_file_extension, location_to_lsp_range}, wxml_utils::{for_each_scope_ref, for_each_scope_ref_in_subtree, for_each_slot, for_each_static_class_name_in_element, for_each_template_element, insert_element_scopes, ScopeKind, Token}};

use super::*;

pub(super) fn rec_import_and_include_elements(
    project: &Project,
    abs_path: &Path,
    template: &Template,
    mut f: impl FnMut(&Path, &Element),
) {
    project.import_and_include_templates(abs_path, template, |abs_path, template| {
        for_each_template_element(template, |elem, _| f(abs_path, elem));
    });
}

pub(super) fn find_elements_matching_tag_name(
    project: &Project,
    abs_path: &Path,
    template: &Template,
    name: &str,
) -> Vec<Location> {
    let mut ret = vec![];
    rec_import_and_include_elements(project, abs_path, template, |abs_path, elem| {
        if let ElementKind::Normal { tag_name, .. } = &elem.kind {
            if name == tag_name.name {
                ret.push(Location {
                    uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                    range: location_to_lsp_range(&tag_name.location),
                });
            }
        }
    });
    ret
}

pub(super) fn find_elements_matching_id(
    project: &Project,
    abs_path: &Path,
    template: &Template,
    name: &str,
) -> Vec<Location> {
    let mut ret = vec![];
    rec_import_and_include_elements(project, abs_path, template, |abs_path, elem| {
        if let ElementKind::Normal { common, .. } = &elem.kind {
            let elem_id_loc = common.id.as_ref().and_then(|x| {
                match &x.1 {
                    Value::Static { value, location, .. } => Some((value.as_str(), location)),
                    _ => None,
                }
            });
            if let Some((id, loc)) = elem_id_loc {
                if name == id.trim() {
                    ret.push(Location {
                        uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                        range: location_to_lsp_range(loc),
                    });
                }
            }
        }
    });
    ret
}

pub(super) fn find_elements_matching_class(
    project: &Project,
    abs_path: &Path,
    template: &Template,
    name: &str,
) -> Vec<Location> {
    let mut ret = vec![];
    rec_import_and_include_elements(project, abs_path, template, |abs_path, elem| {
        for_each_static_class_name_in_element(elem, |class_name, loc| {
            if name == class_name {
                ret.push(Location {
                    uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                    range: location_to_lsp_range(&loc),
                });
            }
        });
    });
    ret
}

fn find_slot_matching<'a>(
    project: &'a Project,
    abs_path: &Path,
    tag_name: &str,
    slot_name: &Ident,
) -> Option<(PathBuf, Vec<std::ops::Range<Position>>)> {
    let mut ret = vec![];
    let target_path = project.get_target_component_path(abs_path, tag_name)?;
    let target_wxml_path = add_file_extension(&target_path, "wxml")?;
    let template = project.get_wxml_tree(&target_wxml_path).ok()?;
    for_each_slot(template, |slot_elem| {
        match &slot_elem.kind {
            ElementKind::Slot { values, .. } => {
                if let Some(attr) = values.iter().find(|x| &x.name.name == &slot_name.name) {
                    ret.push(attr.name.location());
                }
            }
            _ => {}
        }
    });
    Some((target_wxml_path, ret))
}

fn get_target_template_path<'a>(
    project: &'a Project,
    abs_path: &Path,
    source_tamplate: &Template,
    is: &str,
) -> Option<(PathBuf, &'a TemplateDefinition)> {
    for import in source_tamplate.globals.imports.iter().rev() {
        if let Some(p) = project.find_rel_path_for_file(abs_path, &import.src.name) {
            let Some(imported_path) = crate::utils::ensure_file_extension(&p, "wxml") else {
                continue;
            };
            if let Ok(imported_template) = project.get_wxml_tree(&imported_path) {
                if let Some(x) = imported_template.globals.sub_templates.iter().rfind(|x| x.name.is(is)) {
                    return Some((imported_path.to_path_buf(), x));
                }
            }
        }
    }
    None
}

fn search_wxml_template_usages(
    project: &Project,
    abs_path: &Path,
    is: &str,
    mut f: impl FnMut(&Path, std::ops::Range<glass_easel_template_compiler::parse::Position>),
) {
    let Some(root) = project.root() else { return };
    for (source_p, tree) in project.list_wxml_trees() {
        let Ok(source_p) = crate::utils::join_unix_rel_path(root, source_p, root) else { continue };
        if let Some((p, _)) = get_target_template_path(project, &source_p, tree, is) {
            if p.as_path() != abs_path { continue };
            crate::wxml_utils::for_each_template_element(tree, |elem, _| {
                match &elem.kind {
                    ElementKind::TemplateRef { target, .. } => {
                        match &target.1 {
                            Value::Static { value, location, .. } => {
                                if value.as_str() == is {
                                    f(&source_p, location.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            });
        }
    }
}

pub(super) fn find_declaration(project: &mut Project, abs_path: &Path, pos: lsp_types::Position, to_definition: bool) -> anyhow::Result<Vec<LocationLink>> {
    let mut ret = vec![];
    if let Ok(template) = project.get_wxml_tree(abs_path) {
        let token = crate::wxml_utils::find_token_in_position(template, Position { line: pos.line, utf16_col: pos.character });
        match token {
            Token::TagName(ident) => {
                if let Some(target_path) = project.get_target_component_path(abs_path, &ident.name) {
                    if let Some(target_wxml_path) = add_file_extension(&target_path, "wxml") {
                        let target_range = lsp_types::Range::new(
                            lsp_types::Position { line: 0, character: 0 },
                            lsp_types::Position { line: 0, character: 0 },
                        );
                        ret.push(LocationLink {
                            origin_selection_range: Some(location_to_lsp_range(&ident.location)),
                            target_uri: lsp_types::Url::from_file_path(target_wxml_path).unwrap(),
                            target_range: target_range.clone(),
                            target_selection_range: target_range,
                        });
                    }
                }
            }
            Token::StaticId(loc, _) | Token::StaticClassName(loc, _) => {
                ret.push(LocationLink {
                    origin_selection_range: Some(location_to_lsp_range(&loc)),
                    target_uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                    target_range: location_to_lsp_range(&loc),
                    target_selection_range: location_to_lsp_range(&loc),
                });
            }
            Token::ScopeRef(loc, kind) => {
                match kind {
                    ScopeKind::Script(script) => {
                        ret.push(LocationLink {
                            origin_selection_range: Some(location_to_lsp_range(&loc)),
                            target_uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                            target_range: location_to_lsp_range(&script.module_name().location),
                            target_selection_range: location_to_lsp_range(&script.module_name().location),
                        });
                    }
                    ScopeKind::ForScope(target, _elem) => {
                        ret.push(LocationLink {
                            origin_selection_range: Some(location_to_lsp_range(&loc)),
                            target_uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                            target_range: location_to_lsp_range(&target.location),
                            target_selection_range: location_to_lsp_range(&target.location),
                        });
                    }
                    ScopeKind::SlotValue(attr, parent) => {
                        let parent_tag_name = match &parent.kind {
                            ElementKind::Normal { tag_name, .. } => {
                                Some(tag_name)
                            }
                            _ => None,
                        };
                        if to_definition && parent_tag_name.is_some() {
                            let tag_name_str: &str = &parent_tag_name.unwrap().name;
                            if let Some((target_wxml_path, ranges)) = find_slot_matching(&project, abs_path, tag_name_str, &attr.name) {
                                for target_loc in ranges {
                                    ret.push(LocationLink {
                                        origin_selection_range: Some(location_to_lsp_range(&loc)),
                                        target_uri: lsp_types::Url::from_file_path(&target_wxml_path).unwrap(),
                                        target_range: location_to_lsp_range(&target_loc),
                                        target_selection_range: location_to_lsp_range(&target_loc),
                                    });
                                }
                            }
                        } else {
                            ret.push(LocationLink {
                                origin_selection_range: Some(location_to_lsp_range(&loc)),
                                target_uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                                target_range: location_to_lsp_range(&attr.value.location),
                                target_selection_range: location_to_lsp_range(&attr.value.location),
                            });
                        }
                    }
                }
            }
            Token::TemplateName(name)
            | Token::ScriptModule(name)
            | Token::ForItem(name, _)
            | Token::ForIndex(name, _) => {
                ret.push(LocationLink {
                    origin_selection_range: Some(location_to_lsp_range(&name.location)),
                    target_uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                    target_range: location_to_lsp_range(&name.location),
                    target_selection_range: location_to_lsp_range(&name.location),
                });
            }
            Token::SlotValueScope(name, key, parent) => {
                let parent_tag_name = match &parent.kind {
                    ElementKind::Normal { tag_name, .. } => {
                        Some(tag_name)
                    }
                    _ => None,
                };
                if to_definition && parent_tag_name.is_some() {
                    let tag_name_str: &str = &parent_tag_name.unwrap().name;
                    if let Some((target_wxml_path, ranges)) = find_slot_matching(&project, abs_path, tag_name_str, &key) {
                        for target_loc in ranges {
                            ret.push(LocationLink {
                                origin_selection_range: Some(location_to_lsp_range(&name.location)),
                                target_uri: lsp_types::Url::from_file_path(&target_wxml_path).unwrap(),
                                target_range: location_to_lsp_range(&target_loc),
                                target_selection_range: location_to_lsp_range(&target_loc),
                            });
                        }
                    }
                } else {
                    ret.push(LocationLink {
                        origin_selection_range: Some(location_to_lsp_range(&name.location)),
                        target_uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                        target_range: location_to_lsp_range(&name.location),
                        target_selection_range: location_to_lsp_range(&name.location),
                    });
                }
            }
            Token::SlotValueRef(key, _, parent) | Token::SlotValueRefAndScope(key, _, parent) => {
                let parent_tag_name = match &parent.kind {
                    ElementKind::Normal { tag_name, .. } => {
                        Some(tag_name)
                    }
                    _ => None,
                };
                if to_definition && parent_tag_name.is_some() {
                    let tag_name_str: &str = &parent_tag_name.unwrap().name;
                    if let Some((target_wxml_path, ranges)) = find_slot_matching(&project, abs_path, tag_name_str, &key) {
                        for target_loc in ranges {
                            ret.push(LocationLink {
                                origin_selection_range: Some(location_to_lsp_range(&key.location)),
                                target_uri: lsp_types::Url::from_file_path(&target_wxml_path).unwrap(),
                                target_range: location_to_lsp_range(&target_loc),
                                target_selection_range: location_to_lsp_range(&target_loc),
                            });
                        }
                    }
                } else {
                    ret.push(LocationLink {
                        origin_selection_range: Some(location_to_lsp_range(&key.location)),
                        target_uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                        target_range: location_to_lsp_range(&key.location),
                        target_selection_range: location_to_lsp_range(&key.location),
                    });
                }
            }
            Token::TemplateRef(is, loc) => {
                if let Some(x) = template.globals.sub_templates.iter().rev().find(|x| x.name.is(is)) {
                    ret.push(LocationLink {
                        origin_selection_range: Some(location_to_lsp_range(&loc)),
                        target_uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                        target_range: location_to_lsp_range(&x.name.location()),
                        target_selection_range: location_to_lsp_range(&x.name.location()),
                    });
                } else {
                    if let Some((imported_path, def)) = get_target_template_path(project, abs_path, template, is) {
                        ret.push(LocationLink {
                            origin_selection_range: Some(location_to_lsp_range(&loc)),
                            target_uri: lsp_types::Url::from_file_path(imported_path).unwrap(),
                            target_range: location_to_lsp_range(&def.name.location()),
                            target_selection_range: location_to_lsp_range(&def.name.location()),
                        });
                    }
                }
            }
            Token::Src(src) => {
                if let Some(p) = project.find_rel_path_for_file(abs_path, &src.name) {
                    if let Some(imported_path) = crate::utils::ensure_file_extension(&p, "wxml") {
                        let target_range = lsp_types::Range::new(
                            lsp_types::Position { line: 0, character: 0 },
                            lsp_types::Position { line: 0, character: 0 },
                        );
                        ret.push(LocationLink {
                            origin_selection_range: Some(location_to_lsp_range(&src.location)),
                            target_uri: lsp_types::Url::from_file_path(imported_path).unwrap(),
                            target_range: target_range.clone(),
                            target_selection_range: target_range,
                        });
                    }
                }
            }
            Token::ScriptSrc(src) => {
                if let Some(p) = project.find_rel_path_for_file(abs_path, &src.name) {
                    if let Some(imported_path) = crate::utils::ensure_file_extension(&p, "wxs") {
                        let target_range = lsp_types::Range::new(
                            lsp_types::Position { line: 0, character: 0 },
                            lsp_types::Position { line: 0, character: 0 },
                        );
                        ret.push(LocationLink {
                            origin_selection_range: Some(location_to_lsp_range(&src.location)),
                            target_uri: lsp_types::Url::from_file_path(imported_path).unwrap(),
                            target_range: target_range.clone(),
                            target_selection_range: target_range,
                        });
                    }
                }
            }
            Token::ScriptContent(..) => {
                // TODO pass to wxs ls
            }
            _ => {}
        }
    }
    Ok(ret)
}

pub(super) fn find_references(project: &mut Project, abs_path: &Path, pos: lsp_types::Position) -> anyhow::Result<Vec<Location>> {
    let mut ret: Vec<Location> = find_declaration(project, abs_path, pos, true)
        .unwrap_or_default()
        .into_iter()
        .map(|x| {
            Location { uri: x.target_uri, range: x.target_selection_range }
        })
        .collect();
    if let Ok(template) = project.get_wxml_tree(abs_path) {
        let token = crate::wxml_utils::find_token_in_position(template, Position { line: pos.line, utf16_col: pos.character });
        match token {
            Token::TagName(ident) => {
                project.search_component_wxml_usages(abs_path, &ident.name, |source_wxml, template, expected_tag_name| {
                    crate::wxml_utils::for_each_tag_name(template, |tag_name| {
                        if tag_name.name.as_str() == expected_tag_name {
                            ret.push(Location {
                                uri: lsp_types::Url::from_file_path(&source_wxml).unwrap(),
                                range: location_to_lsp_range(&tag_name.location),
                            });
                        }
                    });
                });
                let mut x = find_elements_matching_tag_name(project, abs_path, template, &ident.name);
                ret.append(&mut x);
                let wxss_path = abs_path.with_extension("wxss");
                if let Ok(sheet) = project.get_style_sheet(&wxss_path) {
                    let mut x = wxss::find_tag_name_selectors(project, &wxss_path, sheet, &ident.name);
                    ret.append(&mut x);
                }
            }
            Token::StaticId(_, id) => {
                let mut x = find_elements_matching_id(project, abs_path, template, id);
                ret.append(&mut x);
                let wxss_path = abs_path.with_extension("wxss");
                if let Ok(sheet) = project.get_style_sheet(&wxss_path) {
                    let mut x = wxss::find_id_selectors(project, &wxss_path, sheet, id);
                    ret.append(&mut x);
                }
            }
            Token::StaticClassName(_, class_name) => {
                let mut x = find_elements_matching_class(project, abs_path, template, class_name);
                ret.append(&mut x);
                let wxss_path = abs_path.with_extension("wxss");
                if let Ok(sheet) = project.get_style_sheet(&wxss_path) {
                    let mut x = wxss::find_class_selectors(project, &wxss_path, sheet, class_name);
                    ret.append(&mut x);
                }
            }
            Token::ScriptModule(name) => {
                for_each_scope_ref(template, |loc, kind| {
                    match kind {
                        ScopeKind::Script(x) => {
                            if x.module_name().name_eq(name) {
                                ret.push(Location {
                                    uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                                    range: location_to_lsp_range(&loc),
                                });
                            }
                        }
                        _ => {}
                    }
                });
            }
            Token::ForItem(name, elem)
            | Token::ForIndex(name, elem) => {
                for_each_scope_ref(template, |loc, kind| {
                    match kind {
                        ScopeKind::ForScope(x, target_elem) => {
                            if x.name_eq(name) && elem as *const _ == target_elem as *const _ {
                                ret.push(Location {
                                    uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                                    range: location_to_lsp_range(&loc),
                                });
                            }
                        }
                        _ => {}
                    }
                });
            }
            Token::ScopeRef(_, ScopeKind::SlotValue(StaticAttribute { name: key, .. }, parent))
            | Token::SlotValueScope(_, key, parent)
            | Token::SlotValueRef(key, _, parent)
            | Token::SlotValueRefAndScope(key, _, parent) => {
                let parent_tag_name = match &parent.kind {
                    ElementKind::Normal { tag_name, .. } => {
                        Some(tag_name)
                    }
                    _ => None,
                };
                if let Some(tag_name) = parent_tag_name {
                    let expected_key_name = key.name.as_str();
                    project.search_component_wxml_usages(abs_path, &tag_name.name, |source_wxml, template, expected_tag_name| {
                        crate::wxml_utils::for_each_template_element(template, |elem, parent_scopes| {
                            let (tag_name, children) = match &elem.kind {
                                ElementKind::Normal { tag_name, children, .. } => (tag_name, children),
                                _ => return,
                            };
                            if tag_name.name.as_str() != expected_tag_name { return };
                            for child in children {
                                let Node::Element(child_elem) = child else { continue };
                                let Some(mut refs) = child_elem.slot_value_refs() else { continue };
                                let Some(expected_attr) = refs.find(|attr| attr.name.name == expected_key_name) else { continue };
                                ret.push(Location {
                                    uri: lsp_types::Url::from_file_path(&source_wxml).unwrap(),
                                    range: location_to_lsp_range(&expected_attr.name.location),
                                });
                                let mut new_scopes = parent_scopes.iter().cloned().collect();
                                insert_element_scopes(&mut new_scopes, elem);
                                for_each_scope_ref_in_subtree(child, &mut new_scopes, |loc, kind| {
                                    if let ScopeKind::SlotValue(attr, _) = kind {
                                        if attr as *const _ == expected_attr as *const _ {
                                            ret.push(Location {
                                                uri: lsp_types::Url::from_file_path(&source_wxml).unwrap(),
                                                range: location_to_lsp_range(&loc),
                                            });
                                        }
                                    }
                                });
                            }
                        });
                    });
                }
            }
            Token::TemplateName(name) => {
                search_wxml_template_usages(project, abs_path, &name.name, |target_wxml, loc| {
                    ret.push(Location {
                        uri: lsp_types::Url::from_file_path(&target_wxml).unwrap(),
                        range: location_to_lsp_range(&loc),
                    });
                });
            }
            Token::TemplateRef(is, _) => {
                if let Some((abs_path, _)) = get_target_template_path(project, abs_path, template, is) {
                    search_wxml_template_usages(project, &abs_path, is, |target_wxml, loc| {
                        ret.push(Location {
                            uri: lsp_types::Url::from_file_path(&target_wxml).unwrap(),
                            range: location_to_lsp_range(&loc),
                        });
                    });
                }
            }
            Token::Src(_) | Token::ScriptSrc(_) => {
                ret.truncate(0);
            }
            Token::ScopeRef(_loc, kind) => {
                for_each_scope_ref(template, |loc, other| {
                    if kind.location_eq(other) {
                        ret.push(Location {
                            uri: lsp_types::Url::from_file_path(abs_path).unwrap(),
                            range: location_to_lsp_range(&loc),
                        });
                    }
                });
            }
            Token::ScriptContent(..) => {
                // TODO pass to wxs ls
            }
            _ => {}
        }
    }
    Ok(ret)
}

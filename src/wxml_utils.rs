use std::ops::Range;

use glass_easel_template_compiler::parse::{expr::Expression, tag::{ClassAttribute, Comment, CommonElementAttributes, Element, ElementKind, Ident, Node, Script, StaticAttribute, StrName, StyleAttribute, TagLocation, UnknownMetaTag, Value}, Position, Template, TemplateStructure};

pub(crate) fn location_to_lsp_range(loc: &Range<Position>) -> lsp_types::Range {
    lsp_types::Range {
        start: lsp_types::Position { line: loc.start.line, character: loc.start.utf16_col },
        end: lsp_types::Position { line: loc.end.line, character: loc.end.utf16_col },
    }
}

pub(crate) fn lsp_range_to_location(loc: &lsp_types::Range) -> Range<Position> {
    let start = Position { line: loc.start.line, utf16_col: loc.start.character };
    let end = Position { line: loc.end.line, utf16_col: loc.end.character };
    start..end
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) enum Token<'a> {
    None,
    ScopeRef(Range<Position>, ScopeKind<'a>),
    DataField(&'a str, Range<Position>),
    StaticMember(&'a str, Range<Position>),
    Keyword(Range<Position>),
    Src(&'a StrName),
    ScriptModule(&'a StrName),
    ScriptSrc(&'a StrName),
    ScriptContent(Range<Position>),
    TemplateName(&'a StrName),
    TemplateRef(&'a str, Range<Position>),
    Comment(&'a Comment),
    UnknownMetaTag(&'a UnknownMetaTag),
    StaticStr(Range<Position>),
    TagName(&'a Ident),
    AttributeName(&'a Ident),
    ClassName(&'a Ident),
    StyleName(&'a Ident),
    EventHandler(&'a StrName),
    GenericRef(&'a StrName),
    SlotValueDefinition(&'a Ident),
    SlotValueRef(&'a Ident),
    SlotValueScope(&'a StrName),
    SlotValueRefAndScope(&'a Ident),
    DataKey(&'a Ident),
    MarkKey(&'a Ident),
    EventName(&'a Ident),
    ForItem(&'a StrName),
    ForIndex(&'a StrName),
    ForKey(&'a StrName),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ScopeKind<'a> {
    Script(&'a Script),
    ForScope(&'a StrName),
    SlotValue(&'a Element, &'a StaticAttribute),
}

fn exclusive_contains(loc: &Range<Position>, pos: Position) -> bool {
    loc.start < pos && pos < loc.end
}

fn inclusive_contains(loc: &Range<Position>, pos: Position) -> bool {
    (loc.start..=loc.end).contains(&pos)
}

fn str_name_contains(x: &StrName, pos: Position) -> bool {
    inclusive_contains(&x.location, pos)
}

fn ident_contains(x: &Ident, pos: Position) -> bool {
    inclusive_contains(&x.location, pos)
}

fn tag_contains(tag_loc: &TagLocation, pos: Position) -> bool {
    let start = tag_loc.start.0.start;
    let end = tag_loc.end.as_ref().unwrap_or(&tag_loc.start).1.end;
    (start..=end).contains(&pos)
}

fn tag_body_contains(tag_loc: &TagLocation, pos: Position) -> bool {
    let start = tag_loc.start.0.start;
    let end = tag_loc.start.1.end;
    (start..=end).contains(&pos)
}

pub(crate) fn find_token_in_position(template: &Template, pos: Position) -> Token {
    fn find_in_expr<'a>(expr: &'a Expression, pos: Position, scopes: &mut Vec<ScopeKind<'a>>) -> Token<'a> {
        match expr {
            Expression::ScopeRef { location, index } => {
                if inclusive_contains(location, pos) {
                    return Token::ScopeRef(location.clone(), scopes[*index]);
                }
            }
            Expression::DataField { name, location } => {
                if inclusive_contains(location, pos) {
                    return Token::DataField(&name, location.clone());
                }
            }
            Expression::StaticMember { field_name, field_location, .. } => {
                if inclusive_contains(field_location, pos) {
                    return Token::DataField(&field_name, field_location.clone());
                }
            }
            _ => {}
        }
        for sub in expr.sub_expressions() {
            let ret = find_in_expr(sub, pos, scopes);
            if let Token::None = ret {
                continue;
            }
            return ret;
        }
        Token::None
    }
    fn find_in_value<'a>(v: &'a Value, pos: Position, scopes: &mut Vec<ScopeKind<'a>>) -> Token<'a> {
        match v {
            Value::Static { location, .. } => {
                Token::StaticStr(location.clone())
            }
            Value::Dynamic { expression, .. } => {
                find_in_expr(expression, pos, scopes)
            }
            _ => Token::None
        }
    }
    fn find_in_nodes<'a>(nodes: &'a [Node], pos: Position, scopes: &mut Vec<ScopeKind<'a>>) -> Token<'a> {
        for node in nodes {
            match node {
                Node::Text(v) => {
                    if exclusive_contains(&v.location(), pos) {
                        return find_in_value(v, pos, scopes);
                    }
                }
                Node::Element(elem) => {
                    if tag_contains(&elem.tag_location, pos) {
                        if let Some(attrs) = elem.slot_value_refs() {
                            for attr in attrs {
                                scopes.push(ScopeKind::SlotValue(elem, attr));
                            }
                        }
                        fn find_in_common<'a>(common: &'a CommonElementAttributes, pos: Position, scopes: &mut Vec<ScopeKind<'a>>) -> Token<'a> {
                            if let Some((loc, v)) = common.id.as_ref() {
                                if inclusive_contains(loc, pos) {
                                    return Token::Keyword(loc.clone());
                                }
                                if inclusive_contains(&v.location(), pos) {
                                    return find_in_value(&v, pos, scopes);
                                }
                            }
                            if let Some((loc, v)) = common.slot.as_ref() {
                                if inclusive_contains(loc, pos) {
                                    return Token::Keyword(loc.clone());
                                }
                                if inclusive_contains(&v.location(), pos) {
                                    return find_in_value(&v, pos, scopes);
                                }
                            }
                            for attr in common.slot_value_refs.iter() {
                                if ident_contains(&attr.name, pos) {
                                    if attr.name.location == attr.value.location {
                                        return Token::SlotValueRefAndScope(&attr.name);
                                    }
                                    return Token::SlotValueRef(&attr.name);
                                }
                                if str_name_contains(&attr.value, pos) {
                                    return Token::SlotValueScope(&attr.value);
                                }
                            }
                            for attr in common.data.iter() {
                                if ident_contains(&attr.name, pos) {
                                    return Token::DataKey(&attr.name);
                                }
                                if inclusive_contains(&attr.value.location(), pos) {
                                    return find_in_value(&attr.value, pos, scopes);
                                }
                            }
                            for attr in common.data.iter() {
                                if ident_contains(&attr.name, pos) {
                                    return Token::MarkKey(&attr.name);
                                }
                                if inclusive_contains(&attr.value.location(), pos) {
                                    return find_in_value(&attr.value, pos, scopes);
                                }
                            }
                            for ev in common.event_bindings.iter() {
                                if ident_contains(&ev.name, pos) {
                                    return Token::EventName(&ev.name);
                                }
                                if inclusive_contains(&ev.value.location(), pos) {
                                    return find_in_value(&ev.value, pos, scopes);
                                }
                            }
                            Token::None
                        }
                        match &elem.kind {
                            ElementKind::Normal {
                                tag_name,
                                attributes,
                                class,
                                style,
                                change_attributes,
                                worklet_attributes,
                                children,
                                generics,
                                extra_attr: _,
                                common,
                                ..
                            } => {
                                if tag_body_contains(&elem.tag_location, pos) {
                                    if ident_contains(tag_name, pos) {
                                        return Token::TagName(tag_name);
                                    }
                                    for attr in attributes.iter().chain(change_attributes.iter()) {
                                        if ident_contains(&attr.name, pos) {
                                            return Token::AttributeName(&attr.name);
                                        }
                                        if inclusive_contains(&attr.value.location(), pos) {
                                            return find_in_value(&attr.value, pos, scopes);
                                        }
                                    }
                                    for attr in worklet_attributes.iter() {
                                        if ident_contains(&attr.name, pos) {
                                            return Token::AttributeName(&attr.name);
                                        }
                                        if str_name_contains(&attr.value, pos) {
                                            return Token::EventHandler(&attr.value);
                                        }
                                    }
                                    for attr in generics.iter() {
                                        if ident_contains(&attr.name, pos) {
                                            return Token::AttributeName(&attr.name);
                                        }
                                        if str_name_contains(&attr.value, pos) {
                                            return Token::GenericRef(&attr.value);
                                        }
                                    }
                                    match class {
                                        ClassAttribute::None => {}
                                        ClassAttribute::String(loc, v) => {
                                            if inclusive_contains(loc, pos) {
                                                return Token::Keyword(loc.clone());
                                            }
                                            if inclusive_contains(&v.location(), pos) {
                                                return find_in_value(&v, pos, scopes);
                                            }
                                        }
                                        ClassAttribute::Multiple(list) => {
                                            for (name, v) in list {
                                                if ident_contains(name, pos) {
                                                    return Token::ClassName(name);
                                                }
                                                if inclusive_contains(&v.location(), pos) {
                                                    return find_in_value(&v, pos, scopes);
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                    match style {
                                        StyleAttribute::None => {}
                                        StyleAttribute::String(loc, v) => {
                                            if inclusive_contains(loc, pos) {
                                                return Token::Keyword(loc.clone());
                                            }
                                            if inclusive_contains(&v.location(), pos) {
                                                return find_in_value(&v, pos, scopes);
                                            }
                                        }
                                        StyleAttribute::Multiple(list) => {
                                            for (name, v) in list {
                                                if ident_contains(name, pos) {
                                                    return Token::StyleName(name);
                                                }
                                                if inclusive_contains(&v.location(), pos) {
                                                    return find_in_value(&v, pos, scopes);
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                    return find_in_common(common, pos, scopes);
                                }
                                return find_in_nodes(&children, pos, scopes);
                            }
                            ElementKind::Pure { children, slot, slot_value_refs, .. } => {
                                if tag_body_contains(&elem.tag_location, pos) {
                                    if let Some((loc, v)) = slot.as_ref() {
                                        if inclusive_contains(loc, pos) {
                                            return Token::Keyword(loc.clone());
                                        }
                                        if inclusive_contains(&v.location(), pos) {
                                            return find_in_value(&v, pos, scopes);
                                        }
                                    }
                                    for attr in slot_value_refs.iter() {
                                        if ident_contains(&attr.name, pos) {
                                            if attr.name.location == attr.value.location {
                                                return Token::SlotValueRefAndScope(&attr.name);
                                            }
                                            return Token::SlotValueDefinition(&attr.name);
                                        }
                                        if str_name_contains(&attr.value, pos) {
                                            return Token::SlotValueScope(&attr.value);
                                        }
                                    }
                                    return Token::None;
                                }
                                return find_in_nodes(&children, pos, scopes);
                            }
                            ElementKind::If { branches, else_branch, .. } => {
                                for (loc, v, nodes) in branches {
                                    if inclusive_contains(loc, pos) {
                                        return Token::Keyword(loc.clone());
                                    }
                                    if inclusive_contains(&v.location(), pos) {
                                        return find_in_value(&v, pos, scopes);
                                    }
                                    let ret = find_in_nodes(nodes, pos, scopes);
                                    if let Token::None = ret {
                                        continue;
                                    }
                                    return ret;
                                }
                                if let Some((loc, nodes)) = else_branch {
                                    if inclusive_contains(loc, pos) {
                                        return Token::Keyword(loc.clone());
                                    }
                                    let ret = find_in_nodes(nodes, pos, scopes);
                                    return ret;
                                }
                                return Token::None;
                            }
                            ElementKind::For { list, item_name, index_name, key, children, .. } => {
                                if tag_body_contains(&elem.tag_location, pos) {
                                    let (loc, v) = list;
                                    if inclusive_contains(loc, pos) {
                                        return Token::Keyword(loc.clone());
                                    }
                                    if inclusive_contains(&v.location(), pos) {
                                        return find_in_value(&v, pos, scopes);
                                    }
                                    let (loc, v) = item_name;
                                    if inclusive_contains(loc, pos) {
                                        return Token::Keyword(loc.clone());
                                    }
                                    if str_name_contains(v, pos) {
                                        return Token::ForItem(v);
                                    }
                                    let (loc, v) = index_name;
                                    if inclusive_contains(loc, pos) {
                                        return Token::Keyword(loc.clone());
                                    }
                                    if str_name_contains(v, pos) {
                                        return Token::ForIndex(v);
                                    }
                                    let (loc, v) = key;
                                    if inclusive_contains(loc, pos) {
                                        return Token::Keyword(loc.clone());
                                    }
                                    if str_name_contains(v, pos) {
                                        return Token::ForKey(v);
                                    }
                                    return Token::None;
                                }
                                scopes.push(ScopeKind::ForScope(&item_name.1));
                                scopes.push(ScopeKind::ForScope(&index_name.1));
                                return find_in_nodes(&children, pos, scopes);
                            }
                            ElementKind::TemplateRef { target, data, .. } => {
                                if tag_body_contains(&elem.tag_location, pos) {
                                    let (loc, v) = target;
                                    if inclusive_contains(loc, pos) {
                                        return Token::Keyword(loc.clone());
                                    }
                                    match v {
                                        Value::Static { value, location, .. } => {
                                            if inclusive_contains(location, pos) {
                                                return Token::TemplateRef(&value, location.clone());
                                            }
                                        }
                                        _ => {
                                            if inclusive_contains(&v.location(), pos) {
                                                return find_in_value(&v, pos, scopes);
                                            }
                                        }
                                    }
                                    let (loc, v) = data;
                                    if inclusive_contains(loc, pos) {
                                        return Token::Keyword(loc.clone());
                                    }
                                    if inclusive_contains(&v.location(), pos) {
                                        return find_in_value(&v, pos, scopes);
                                    }
                                }
                                return Token::None;
                            }
                            ElementKind::Include { .. } => {
                                return Token::None;
                            }
                            ElementKind::Slot { name, values, common, .. } => {
                                if tag_body_contains(&elem.tag_location, pos) {
                                    let (loc, v) = name;
                                    if inclusive_contains(loc, pos) {
                                        return Token::Keyword(loc.clone());
                                    }
                                    if inclusive_contains(&v.location(), pos) {
                                        return find_in_value(&v, pos, scopes);
                                    }
                                    for attr in values {
                                        if ident_contains(&attr.name, pos) {
                                            return Token::SlotValueDefinition(&attr.name);
                                        }
                                        if inclusive_contains(&attr.value.location(), pos) {
                                            return find_in_value(&attr.value, pos, scopes);
                                        }
                                    }
                                    return find_in_common(common, pos, scopes);
                                }
                                return Token::None;
                            }
                            _ => {
                                return Token::None;
                            }
                        }
                        // unreachable!()
                    }
                }
                Node::Comment(x) => {
                    if exclusive_contains(&x.location, pos) {
                        return Token::Comment(x);
                    }
                }
                Node::UnknownMetaTag(x) => {
                    if exclusive_contains(&x.location, pos) {
                        return Token::UnknownMetaTag(x);
                    }
                }
                _ => {}
            }
        }
        Token::None
    }

    // import tag
    for i in template.globals.imports.iter() {
        if tag_contains(&i.tag_location, pos) {
            if inclusive_contains(&i.src_location, pos) {
                return Token::Keyword(i.src_location.clone());
            }
            if str_name_contains(&i.src, pos) {
                return Token::Src(&i.src);
            }
            return Token::None;
        }
    }

    // include tag
    for i in template.globals.includes.iter() {
        if tag_contains(&i.tag_location, pos) {
            if inclusive_contains(&i.src_location, pos) {
                return Token::Keyword(i.src_location.clone());
            }
            if str_name_contains(&i.src, pos) {
                return Token::Src(&i.src);
            }
            return Token::None;
        }
    }

    // script tag
    for i in template.globals.scripts.iter() {
        if tag_contains(&i.tag_location(), pos) {
            if inclusive_contains(&i.module_location(), pos) {
                return Token::Keyword(i.module_location());
            }
            if str_name_contains(i.module_name(), pos) {
                return Token::ScriptModule(i.module_name());
            }
            match i {
                Script::Inline { content_location, .. } => {
                    if inclusive_contains(content_location, pos) {
                        return Token::ScriptContent(content_location.clone());
                    }
                }
                Script::GlobalRef { src_location, src, .. } => {
                    if inclusive_contains(src_location, pos) {
                        return Token::Keyword(src_location.clone());
                    }
                    if str_name_contains(src, pos) {
                        return Token::ScriptSrc(src);
                    }
                }
                _ => {}
            }
            return Token::None;
        }
    }

    // find in sub templates
    let mut scopes = template.globals.scripts.iter().map(|x| ScopeKind::Script(x)).collect();
    for i in template.globals.sub_templates.iter() {
        if tag_contains(&i.tag_location, pos) {
            if inclusive_contains(&i.name_location, pos) {
                return Token::Keyword(i.name_location.clone());
            }
            if str_name_contains(&i.name, pos) {
                return Token::TemplateName(&i.name);
            }
            return find_in_nodes(&i.content, pos, &mut scopes);
        }
    }

    find_in_nodes(&template.content, pos, &mut scopes)
}

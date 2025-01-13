use std::ops::Range;

use glass_easel_template_compiler::parse::{expr::Expression, tag::{ClassAttribute, Comment, CommonElementAttributes, Element, ElementKind, Ident, Node, Script, StaticAttribute, StrName, StyleAttribute, TagLocation, UnknownMetaTag, Value}, Position, Template};

use crate::utils::{exclusive_contains, inclusive_contains};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) enum Token<'a> {
    None,
    StaticTextContent(Range<Position>, &'a str, Option<&'a Element>),
    StaticValuePart(Range<Position>, &'a str),
    StartTagBody(&'a Element),
    EndTagBody(&'a Element),
    ScopeRef(Range<Position>, ScopeKind<'a>),
    DataField(&'a str, Range<Position>),
    StaticMember(&'a str, Range<Position>),
    AttributeKeyword(Range<Position>, &'a Element),
    OtherKeyword(Range<Position>),
    Src(&'a StrName),
    ScriptModule(&'a StrName),
    ScriptSrc(&'a StrName),
    ScriptContent(Range<Position>),
    TemplateName(&'a StrName),
    TemplateRef(&'a str, Range<Position>),
    Comment(&'a Comment),
    UnknownMetaTag(&'a UnknownMetaTag),
    TagName(&'a Ident),
    StaticId(Range<Position>, &'a str),
    AttributeStaticValue(Range<Position>, &'a str, &'a Ident, &'a Element),
    AttributeName(&'a Ident, &'a Element),
    ModelAttributeName(&'a Ident, &'a Element),
    ChangeAttributeName(&'a Ident, &'a Element),
    StaticClassName(Range<Position>, &'a str),
    StyleName(&'a Ident),
    EventHandler(&'a StrName, &'a Ident),
    GenericRef(&'a StrName, &'a Ident),
    SlotValueDefinition(&'a Ident),
    SlotValueRef(&'a Ident, &'a StrName, &'a Element),
    SlotValueScope(&'a StrName, &'a Ident, &'a Element),
    SlotValueRefAndScope(&'a Ident, &'a StrName, &'a Element),
    DataKey(&'a Ident),
    MarkKey(&'a Ident),
    EventName(&'a Ident, &'a Element),
    ForItem(&'a StrName, &'a Element),
    ForIndex(&'a StrName, &'a Element),
    ForKey(&'a StrName, &'a Element),
}

impl<'a> Token<'a> {
    fn or(self, default: Self) -> Self {
        if let Self::None = self {
            default
        } else {
            self
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ScopeKind<'a> {
    Script(&'a Script),
    ForScope(&'a StrName, &'a Element),
    SlotValue(&'a StaticAttribute, &'a Element),
}

impl<'a> ScopeKind<'a> {
    pub(crate) fn location_eq(&self, other: Self) -> bool {
        match *self {
            ScopeKind::Script(a) => {
                if let ScopeKind::Script(b) = other {
                    a as *const _ == b as *const _
                } else {
                    false
                }
            }
            ScopeKind::ForScope(a, a_elem) => {
                if let ScopeKind::ForScope(b, b_elem) = other {
                    a as *const _ == b as *const _ && a_elem as *const _ == b_elem as *const _
                } else {
                    false
                }
            }
            ScopeKind::SlotValue(a, a_elem) => {
                if let ScopeKind::SlotValue(b, b_elem) = other {
                    a as *const _ == b as *const _ && a_elem as *const _ == b_elem as *const _
                } else {
                    false
                }
            }
        }
    }
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

fn start_tag_body_contains(tag_loc: &TagLocation, pos: Position) -> bool {
    let start = tag_loc.start.0.start;
    let end = tag_loc.start.1.end;
    exclusive_contains(&(start..end), pos)
}

fn end_tag_body_contains(tag_loc: &TagLocation, pos: Position) -> bool {
    let Some(end_loc) = tag_loc.end.as_ref() else { return false };
    let start = end_loc.0.start;
    let end = end_loc.1.end;
    exclusive_contains(&(start..end), pos)
}

pub(crate) fn find_token_in_position(template: &Template, pos: Position) -> Token {
    fn find_in_expr<'a>(expr: &'a Expression, pos: Position, scopes: &mut Vec<ScopeKind<'a>>, has_static_parts: bool) -> Token<'a> {
        let mut next_has_static_parts = false;
        match expr {
            Expression::Plus { .. } => {
                next_has_static_parts = true;
            }
            Expression::LitStr { value, location } =>  {
                if has_static_parts {
                    if inclusive_contains(location, pos) {
                        return Token::StaticValuePart(location.clone(), &value);
                    }
                }
            }
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
            let ret = find_in_expr(sub, pos, scopes, next_has_static_parts);
            if let Token::None = ret {
                continue;
            }
            return ret;
        }
        Token::None
    }
    fn find_in_value<'a>(v: &'a Value, pos: Position, scopes: &mut Vec<ScopeKind<'a>>) -> Option<Token<'a>> {
        match v {
            Value::Static { value, location, .. } => {
                if inclusive_contains(location, pos) {
                    Some(Token::StaticValuePart(location.clone(), value))
                } else {
                    None
                }
            }
            Value::Dynamic { expression, double_brace_location, .. } => {
                fn static_parts_range<'a>(loc: &mut Range<Position>, expr: &'a Expression) -> bool {
                    match expr {
                        Expression::LitStr { location, .. } => {
                            loc.start = loc.start.min(location.start);
                            loc.end = loc.end.max(location.end);
                            false
                        }
                        Expression::ToStringWithoutUndefined { .. } => {
                            true
                        }
                        Expression::Plus { left, right, .. } => {
                            static_parts_range(loc, &left) || static_parts_range(loc, &right)
                        }
                        _ => false,
                    }
                }
                let mut loc = double_brace_location.0.start..double_brace_location.1.end;
                let init_has_static_parts = static_parts_range(&mut loc, &expression);
                if inclusive_contains(&loc, pos) {
                    Some(find_in_expr(expression, pos, scopes, init_has_static_parts))
                } else {
                    None
                }
            }
            _ => None
        }
    }
    fn find_in_nodes<'a>(
        parent: Option<&'a Element>,
        nodes: &'a [Node],
        pos: Position,
        scopes: &mut Vec<ScopeKind<'a>>,
    ) -> Token<'a> {
        for node in nodes {
            match node {
                Node::Text(v) => {
                    if let Some(ret) = find_in_value(v, pos, scopes) {
                        if let Token::StaticValuePart(loc, v) = ret {
                            return Token::StaticTextContent(loc, v, parent);
                        }
                        return ret;
                    }
                }
                Node::Element(elem) => {
                    if tag_contains(&elem.tag_location, pos) {
                        if let Some(attrs) = elem.slot_value_refs() {
                            if let Some(parent) = parent {
                                for attr in attrs {
                                    scopes.push(ScopeKind::SlotValue(attr, parent));
                                }
                            }
                        }
                        fn find_in_slot_value_refs<'a>(
                            parent: Option<&'a Element>,
                            slot_value_refs: &'a [StaticAttribute],
                            pos: Position,
                        ) -> Token<'a> {
                            if let Some(parent) = parent {
                                for attr in slot_value_refs.iter() {
                                    if ident_contains(&attr.name, pos) {
                                        if attr.name.location == attr.value.location {
                                            return Token::SlotValueRefAndScope(&attr.name, &attr.value, parent);
                                        }
                                        return Token::SlotValueRef(&attr.name, &attr.value, parent);
                                    }
                                    if str_name_contains(&attr.value, pos) {
                                        return Token::SlotValueScope(&attr.value, &attr.name, parent);
                                    }
                                }
                            }
                            Token::None
                        }
                        fn find_in_common<'a>(
                            parent: Option<&'a Element>,
                            elem: &'a Element,
                            common: &'a CommonElementAttributes,
                            pos: Position,
                            scopes: &mut Vec<ScopeKind<'a>>,
                        ) -> Token<'a> {
                            if let Some((loc, v)) = common.id.as_ref() {
                                if inclusive_contains(loc, pos) {
                                    return Token::AttributeKeyword(loc.clone(), &elem);
                                }
                                match v {
                                    Value::Static { value, location, .. } => {
                                        return Token::StaticId(location.clone(), &value);
                                    },
                                    _ => {
                                        if let Some(ret) = find_in_value(v, pos, scopes) {
                                            return ret;
                                        }
                                    }
                                }
                            }
                            if let Some((loc, v)) = common.slot.as_ref() {
                                if inclusive_contains(loc, pos) {
                                    return Token::AttributeKeyword(loc.clone(), &elem);
                                }
                                if let Some(ret) = find_in_value(v, pos, scopes) {
                                    return ret;
                                }
                            }
                            for attr in common.data.iter() {
                                if ident_contains(&attr.name, pos) {
                                    return Token::DataKey(&attr.name);
                                }
                                if let Some(ret) = find_in_value(&attr.value, pos, scopes) {
                                    return ret;
                                }
                            }
                            for attr in common.data.iter() {
                                if ident_contains(&attr.name, pos) {
                                    return Token::MarkKey(&attr.name);
                                }
                                if let Some(ret) = find_in_value(&attr.value, pos, scopes) {
                                    return ret;
                                }
                            }
                            for ev in common.event_bindings.iter() {
                                if ident_contains(&ev.name, pos) {
                                    return Token::EventName(&ev.name, elem);
                                }
                                if let Some(ret) = find_in_value(&ev.value, pos, scopes) {
                                    return ret;
                                }
                            }
                            find_in_slot_value_refs(parent, &common.slot_value_refs, pos)
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
                                if start_tag_body_contains(&elem.tag_location, pos) {
                                    if ident_contains(tag_name, pos) {
                                        return Token::TagName(tag_name);
                                    }
                                    for attr in attributes.iter() {
                                        if ident_contains(&attr.name, pos) {
                                            if attr.is_model {
                                                return Token::ModelAttributeName(&attr.name, elem);
                                            }
                                            return Token::AttributeName(&attr.name, elem);
                                        }
                                        if let Some(ret) = find_in_value(&attr.value, pos, scopes) {
                                            if let Token::StaticValuePart(loc, v) = ret {
                                                return Token::AttributeStaticValue(loc, v, &attr.name, elem);
                                            }
                                            return ret;
                                        }
                                    }
                                    for attr in change_attributes.iter() {
                                        if ident_contains(&attr.name, pos) {
                                            return Token::AttributeName(&attr.name, elem);
                                        }
                                        if let Some(ret) = find_in_value(&attr.value, pos, scopes) {
                                            if let Token::StaticValuePart(loc, v) = ret {
                                                return Token::AttributeStaticValue(loc, v, &attr.name, elem);
                                            }
                                            return ret;
                                        }
                                    }
                                    for attr in worklet_attributes.iter() {
                                        if ident_contains(&attr.name, pos) {
                                            return Token::AttributeName(&attr.name, elem);
                                        }
                                        if str_name_contains(&attr.value, pos) {
                                            return Token::EventHandler(&attr.value, tag_name);
                                        }
                                    }
                                    for attr in generics.iter() {
                                        if ident_contains(&attr.name, pos) {
                                            return Token::AttributeName(&attr.name, elem);
                                        }
                                        if str_name_contains(&attr.value, pos) {
                                            return Token::GenericRef(&attr.value, tag_name);
                                        }
                                    }
                                    match class {
                                        ClassAttribute::None => {}
                                        ClassAttribute::String(loc, v) => {
                                            if inclusive_contains(loc, pos) {
                                                return Token::AttributeKeyword(loc.clone(), &elem);
                                            }
                                            if let Some(ret) = find_in_value(v, pos, scopes) {
                                                return match ret {
                                                    Token::StaticValuePart(loc, name) => Token::StaticClassName(loc.clone(), name),
                                                    x => x,
                                                };
                                            }
                                        }
                                        ClassAttribute::Multiple(list) => {
                                            for (name, v) in list {
                                                if ident_contains(name, pos) {
                                                    return Token::StaticClassName(name.location.clone(), &name.name);
                                                }
                                                if let Some(ret) = find_in_value(v, pos, scopes) {
                                                    return ret;
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                    match style {
                                        StyleAttribute::None => {}
                                        StyleAttribute::String(loc, v) => {
                                            if inclusive_contains(loc, pos) {
                                                return Token::AttributeKeyword(loc.clone(), &elem);
                                            }
                                            if let Some(ret) = find_in_value(v, pos, scopes) {
                                                return ret;
                                            }
                                        }
                                        StyleAttribute::Multiple(list) => {
                                            for (name, v) in list {
                                                if ident_contains(name, pos) {
                                                    return Token::StyleName(name);
                                                }
                                                if let Some(ret) = find_in_value(v, pos, scopes) {
                                                    return ret;
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                    return find_in_common(parent, elem, common, pos, scopes).or(Token::StartTagBody(elem));
                                }
                                if end_tag_body_contains(&elem.tag_location, pos) {
                                    return Token::EndTagBody(elem);
                                }
                                return find_in_nodes(Some(elem), &children, pos, scopes);
                            }
                            ElementKind::Pure { children, slot, slot_value_refs, .. } => {
                                if start_tag_body_contains(&elem.tag_location, pos) {
                                    if let Some((loc, v)) = slot.as_ref() {
                                        if inclusive_contains(loc, pos) {
                                            return Token::OtherKeyword(loc.clone());
                                        }
                                        if let Some(ret) = find_in_value(v, pos, scopes) {
                                            return ret;
                                        }
                                    }
                                    return find_in_slot_value_refs(parent, slot_value_refs, pos).or(Token::StartTagBody(elem));
                                }
                                if end_tag_body_contains(&elem.tag_location, pos) {
                                    return Token::EndTagBody(elem);
                                }
                                return find_in_nodes(Some(elem), &children, pos, scopes);
                            }
                            ElementKind::If { branches, else_branch, .. } => {
                                for (loc, v, nodes) in branches {
                                    if inclusive_contains(loc, pos) {
                                        return Token::AttributeKeyword(loc.clone(), &elem);
                                    }
                                    if let Some(ret) = find_in_value(v, pos, scopes) {
                                        return ret;
                                    }
                                    let ret = find_in_nodes(Some(elem), nodes, pos, scopes);
                                    if let Token::None = ret {
                                        continue;
                                    }
                                    return ret;
                                }
                                if let Some((loc, nodes)) = else_branch {
                                    if inclusive_contains(loc, pos) {
                                        return Token::AttributeKeyword(loc.clone(), &elem);
                                    }
                                    let ret = find_in_nodes(Some(elem), nodes, pos, scopes);
                                    if let Token::None = ret {
                                        // empty
                                    } else {
                                        return ret;
                                    }
                                }
                                if start_tag_body_contains(&elem.tag_location, pos) {
                                    return Token::StartTagBody(elem);
                                }
                                if end_tag_body_contains(&elem.tag_location, pos) {
                                    return Token::StartTagBody(elem);
                                }
                                return Token::None;
                            }
                            ElementKind::For { list, item_name, index_name, key, children, .. } => {
                                {
                                    let (loc, v) = list;
                                    if inclusive_contains(loc, pos) {
                                        return Token::AttributeKeyword(loc.clone(), &elem);
                                    }
                                    if let Some(ret) = find_in_value(v, pos, scopes) {
                                        return ret;
                                    }
                                    let (loc, v) = item_name;
                                    if inclusive_contains(loc, pos) {
                                        return Token::AttributeKeyword(loc.clone(), &elem);
                                    }
                                    if str_name_contains(v, pos) {
                                        return Token::ForItem(v, elem);
                                    }
                                    let (loc, v) = index_name;
                                    if inclusive_contains(loc, pos) {
                                        return Token::AttributeKeyword(loc.clone(), &elem);
                                    }
                                    if str_name_contains(v, pos) {
                                        return Token::ForIndex(v, elem);
                                    }
                                    let (loc, v) = key;
                                    if inclusive_contains(loc, pos) {
                                        return Token::AttributeKeyword(loc.clone(), &elem);
                                    }
                                    if str_name_contains(v, pos) {
                                        return Token::ForKey(v, elem);
                                    }
                                }
                                scopes.push(ScopeKind::ForScope(&item_name.1, elem));
                                scopes.push(ScopeKind::ForScope(&index_name.1, elem));
                                let ret = find_in_nodes(Some(elem), &children, pos, scopes);
                                if let Token::None = ret {
                                    if start_tag_body_contains(&elem.tag_location, pos) {
                                        return Token::StartTagBody(elem);
                                    }
                                    if end_tag_body_contains(&elem.tag_location, pos) {
                                        return Token::StartTagBody(elem);
                                    }
                                }
                                return ret;
                            }
                            ElementKind::TemplateRef { target, data, .. } => {
                                if start_tag_body_contains(&elem.tag_location, pos) {
                                    let (loc, v) = target;
                                    if inclusive_contains(loc, pos) {
                                        return Token::AttributeKeyword(loc.clone(), &elem);
                                    }
                                    match v {
                                        Value::Static { value, location, .. } => {
                                            if inclusive_contains(location, pos) {
                                                return Token::TemplateRef(&value, location.clone());
                                            }
                                        }
                                        _ => {
                                            if let Some(ret) = find_in_value(v, pos, scopes) {
                                                return ret;
                                            }
                                        }
                                    }
                                    let (loc, v) = data;
                                    if inclusive_contains(loc, pos) {
                                        return Token::AttributeKeyword(loc.clone(), &elem);
                                    }
                                    if let Some(ret) = find_in_value(v, pos, scopes) {
                                        return ret;
                                    }
                                    return Token::StartTagBody(elem);
                                }
                                if end_tag_body_contains(&elem.tag_location, pos) {
                                    return Token::EndTagBody(elem);
                                }
                                return Token::None;
                            }
                            ElementKind::Include { .. } => {
                                return Token::None;
                            }
                            ElementKind::Slot { name, values, common, .. } => {
                                if start_tag_body_contains(&elem.tag_location, pos) {
                                    let (loc, v) = name;
                                    if inclusive_contains(loc, pos) {
                                        return Token::AttributeKeyword(loc.clone(), &elem);
                                    }
                                    if let Some(ret) = find_in_value(v, pos, scopes) {
                                        return ret;
                                    }
                                    for attr in values {
                                        if ident_contains(&attr.name, pos) {
                                            return Token::SlotValueDefinition(&attr.name);
                                        }
                                        if let Some(ret) = find_in_value(&attr.value, pos, scopes) {
                                            return ret;
                                        }
                                    }
                                    return find_in_common(parent, elem, common, pos, scopes).or(Token::StartTagBody(elem));
                                }
                                if end_tag_body_contains(&elem.tag_location, pos) {
                                    return Token::EndTagBody(elem);
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
                return Token::OtherKeyword(i.src_location.clone());
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
                return Token::OtherKeyword(i.src_location.clone());
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
                return Token::OtherKeyword(i.module_location());
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
                        return Token::OtherKeyword(src_location.clone());
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
                return Token::OtherKeyword(i.name_location.clone());
            }
            if str_name_contains(&i.name, pos) {
                return Token::TemplateName(&i.name);
            }
            return find_in_nodes(None, &i.content, pos, &mut scopes);
        }
    }

    find_in_nodes(None, &template.content, pos, &mut scopes)
}

pub(crate) fn for_each_template_root<'a>(template: &'a Template, mut f: impl FnMut(&'a Node, &mut Vec<ScopeKind<'a>>)) {
    let mut scopes: Vec<_> = template.globals.scripts.iter().map(|x| ScopeKind::Script(x)).collect();
    for sub in template.globals.sub_templates.iter() {
        for node in sub.content.iter() {
            f(node, &mut scopes);
        }
    }
    for node in template.content.iter() {
        f(node, &mut scopes);
    }
}

pub(crate) fn insert_element_scopes<'a>(scopes: &mut Vec<ScopeKind<'a>>, elem: &'a Element) {
    match &elem.kind {
        ElementKind::For { item_name, index_name, .. } => {
            scopes.push(ScopeKind::ForScope(&item_name.1, elem));
            scopes.push(ScopeKind::ForScope(&index_name.1, elem));
        }
        ElementKind::Normal { common: CommonElementAttributes { slot_value_refs, .. }, .. }
        | ElementKind::Slot { common: CommonElementAttributes { slot_value_refs, .. }, .. }
        | ElementKind::Pure { slot_value_refs, .. }=> {
            for attr in slot_value_refs {
                scopes.push(ScopeKind::SlotValue(attr, elem));
            }
        }
        _ => {}
    }
}

pub(crate) fn for_each_template_node_in_subtree<'a>(
    node: &'a Node,
    scopes: &mut Vec<ScopeKind<'a>>,
    f: &mut impl FnMut(&'a Node, &[ScopeKind<'a>]),
) {
    match node {
        Node::Element(elem) => {
            let scopes_len = scopes.len();
            insert_element_scopes(scopes, elem);
            f(node, scopes);
            for child in elem.iter_children() {
                for_each_template_node_in_subtree(child, scopes, f);
            }
            scopes.truncate(scopes_len);
        }
        _ => {
            f(node, scopes);
        }
    }
}

pub(crate) fn _for_each_template_node<'a>(template: &'a Template, mut f: impl FnMut(&'a Node, &[ScopeKind<'a>])) {
    for_each_template_root(template, |node, scopes| {
        for_each_template_node_in_subtree(node, scopes, &mut f);
    });
}

pub(crate) fn for_each_template_element_in_subtree<'a>(
    node: &'a Node,
    scopes: &mut Vec<ScopeKind<'a>>,
    f: &mut impl FnMut(&'a Element, &[ScopeKind<'a>]),
) {
    for_each_template_node_in_subtree(node, scopes, &mut |node, scopes| {
        match node {
            Node::Element(elem) => {
                f(elem, scopes)
            }
            _ => {}
        }
    });
}

pub(crate) fn for_each_template_element<'a>(template: &'a Template, mut f: impl FnMut(&'a Element, &[ScopeKind<'a>])) {
    for_each_template_root(template, |node, scopes| {
        for_each_template_element_in_subtree(node, scopes, &mut f);
    });
}

pub(crate) fn for_each_template_value_in_subtree<'a>(node: &'a Node, scopes: &mut Vec<ScopeKind<'a>>, mut f: impl FnMut(&'a Value, &[ScopeKind<'a>])) {
    for_each_template_node_in_subtree(node, scopes, &mut |node, scopes| {
        fn handle_common<'a>(common: &'a CommonElementAttributes, scopes: &[ScopeKind<'a>], f: &mut impl FnMut(&'a Value, &[ScopeKind<'a>])) {
            if let Some((_, value)) = common.id.as_ref() {
                f(value, scopes);
            }
            if let Some((_, value)) = common.slot.as_ref() {
                f(value, scopes);
            }
            for attr in common.data.iter().chain(common.marks.iter()) {
                f(&attr.value, scopes);
            }
            for ev in common.event_bindings.iter() {
                f(&ev.value, scopes);
            }
        }
        match node {
            Node::Text(v) => { f(v, scopes); }
            Node::Element(elem) => {
                match &elem.kind {
                    ElementKind::Normal {
                        tag_name: _,
                        attributes,
                        class,
                        style,
                        change_attributes,
                        worklet_attributes: _,
                        children: _,
                        generics: _,
                        extra_attr: _,
                        common,
                        ..
                    } => {
                        for attr in attributes.iter().chain(change_attributes.iter()) {
                            f(&attr.value, scopes);
                        }
                        match class {
                            ClassAttribute::None => {}
                            ClassAttribute::String(_, value) => { f(value, scopes); }
                            ClassAttribute::Multiple(v) => {
                                for (_, value) in v {
                                    f(value, scopes);
                                }
                            }
                            _ => {}
                        }
                        match style {
                            StyleAttribute::None => {}
                            StyleAttribute::String(_, value) => { f(value, scopes); }
                            StyleAttribute::Multiple(v) => {
                                for (_, value) in v {
                                    f(value, scopes);
                                }
                            }
                            _ => {}
                        }
                        handle_common(common, scopes, &mut f);
                    }
                    ElementKind::Pure { children: _, slot, slot_value_refs: _, .. } => {
                        if let Some((_, value)) = slot {
                            f(value, scopes);
                        }
                    }
                    ElementKind::If { branches, else_branch: _, .. } => {
                        for (_, value, _) in branches {
                            f(&value, scopes);
                        }
                    }
                    ElementKind::For { list, item_name: _, index_name: _, key: _, children: _, .. } => {
                        f(&list.1, scopes);
                    }
                    ElementKind::TemplateRef { target, data, .. } => {
                        f(&target.1, scopes);
                        f(&data.1, scopes);
                    }
                    ElementKind::Include { path: _, .. } => {}
                    ElementKind::Slot { name, values, common, .. } => {
                        f(&name.1, scopes);
                        for attr in values.iter() {
                            f(&attr.value, scopes);
                        }
                        handle_common(common, scopes, &mut f);
                    }
                    _ => {}
                }
            }
            Node::Comment(..) => {}
            Node::UnknownMetaTag(..) => {}
            _ => {}
        }
    });
}

pub(crate) fn for_each_template_expression_root_in_subtree<'a>(node: &'a Node, scopes: &mut Vec<ScopeKind<'a>>, mut f: impl FnMut(&'a Expression, &[ScopeKind<'a>])) {
    for_each_template_value_in_subtree(node, scopes, |value, scopes| {
        match value {
            Value::Dynamic { expression, .. } => f(expression, scopes),
            Value::Static { .. } => {}
            _ => {}
        }
    });
}

pub(crate) fn for_each_template_expression_in_subtree<'a>(node: &'a Node, scopes: &mut Vec<ScopeKind<'a>>, mut f: impl FnMut(&'a Expression, &[ScopeKind<'a>])) {
    fn rec<'a>(expression: &'a Expression, scopes: &[ScopeKind<'a>], f: &mut impl FnMut(&'a Expression, &[ScopeKind<'a>])) {
        f(expression, scopes);
        for sub in expression.sub_expressions() {
            rec(sub, scopes, f);
        }
    }
    for_each_template_expression_root_in_subtree(node, scopes, |expression, scopes| { rec(expression, scopes, &mut f); });
}

pub(crate) fn for_each_scope_ref_in_subtree<'a>(node: &'a Node, scopes: &mut Vec<ScopeKind<'a>>, mut f: impl FnMut(Range<Position>, ScopeKind<'a>)) {
    for_each_template_expression_in_subtree(node, scopes, |expr, scopes| {
        match expr {
            Expression::ScopeRef { location, index } => {
                if let Some(s) = scopes.get(*index) {
                    f(location.clone(), *s);
                }
            }
            _ => {}
        }
    });
}

pub(crate) fn for_each_scope_ref<'a>(template: &'a Template, mut f: impl FnMut(Range<Position>, ScopeKind<'a>)) {
    for_each_template_root(&template, |node, scopes| {
        for_each_scope_ref_in_subtree(node, scopes, &mut f);
    });
}

pub(crate) fn for_each_slot_in_subtree<'a>(node: &'a Node, scopes: &mut Vec<ScopeKind<'a>>, mut f: impl FnMut(&Element)) {
    for_each_template_element_in_subtree(node, scopes, &mut |elem, _scopes| {
        match &elem.kind {
            ElementKind::Slot { .. } => {
                f(elem)
            }
            _ => {}
        }
    });
}

pub(crate) fn for_each_slot<'a>(template: &'a Template, mut f: impl FnMut(&Element)) {
    for_each_template_root(&template, |node, scopes| {
        for_each_slot_in_subtree(node, scopes, &mut f);
    });
}

pub(crate) fn for_each_tag_name_in_subtree<'a>(node: &'a Node, scopes: &mut Vec<ScopeKind<'a>>, mut f: impl FnMut(&'a Ident)) {
    for_each_template_element_in_subtree(node, scopes, &mut |elem, _scopes| {
        match &elem.kind {
            ElementKind::Normal { tag_name, .. } => {
                f(tag_name)
            }
            _ => {}
        }
    });
}

pub(crate) fn for_each_tag_name<'a>(template: &'a Template, mut f: impl FnMut(&'a Ident)) {
    for_each_template_root(&template, |node, scopes| {
        for_each_tag_name_in_subtree(node, scopes, &mut f);
    });
}

pub(crate) fn for_each_static_class_name_in_element<'a>(elem: &'a Element, mut f: impl FnMut(&'a str, Range<Position>)) {
    if let ElementKind::Normal { class, .. } = &elem.kind {
        match class {
            ClassAttribute::String(_, value) => {
                fn rec_expr<'a>(
                    expr: &'a Expression,
                    left_space: bool,
                    right_space: bool,
                    f: &mut impl FnMut(&'a str, &Range<Position>, bool, bool),
                ) {
                    match expr {
                        Expression::Plus { left, right, .. } => {
                            rec_expr(&left, left_space, false, f);
                            rec_expr(&right, false, left_space, f);
                        }
                        Expression::LitStr { value, location, .. } => {
                            f(&value, &location, left_space, right_space);
                        }
                        _ => {}
                    }
                }
                let mut check_str = |s: &'a str, loc: &Range<Position>, left_space: bool, right_space: bool| {
                    let s_end_ptr = s.as_ptr() as usize + s.len();
                    for class_name in s.split_ascii_whitespace() {
                        if !left_space && class_name.as_ptr() == s.as_ptr() { continue; }
                        if !right_space && class_name.as_ptr() as usize + class_name.len() == s_end_ptr { continue; }
                        f(class_name, loc.clone());
                    }
                };
                match value {
                    Value::Static { value, location, .. } => {
                        check_str(&value, location, true, true);
                    }
                    Value::Dynamic { expression, .. } => {
                        rec_expr(&expression, true, true, &mut check_str);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

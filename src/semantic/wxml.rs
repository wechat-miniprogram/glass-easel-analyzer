use glass_easel_template_compiler::parse::{expr::{ArrayFieldKind, Expression, ObjectFieldKind}, tag::{ClassAttribute, CommonElementAttributes, ElementKind, Ident, Node, Script, StrName, StyleAttribute, Value}, Position, Template};

use super::*;

struct WxmlToken {
    location: std::ops::Range<Position>,
    ty: TokenType,
    modifier: u32,
}

impl PartialEq for WxmlToken {
    fn eq(&self, other: &Self) -> bool {
        self.location == other.location
    }
}

impl Eq for WxmlToken {}

impl PartialOrd for WxmlToken {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for WxmlToken {
    fn cmp(&self, other: &Self) -> Ordering {
        let a = [
            self.location.start.line,
            self.location.start.utf16_col,
            self.location.end.line,
            self.location.end.utf16_col,
        ];
        let b = [
            other.location.start.line,
            other.location.start.utf16_col,
            other.location.end.line,
            other.location.end.utf16_col,
        ];
        for i in 0..4 {
            if a[i] < b[i] { return Ordering::Less }
            if a[i] > b[i] { return Ordering::Greater }
        }
        Ordering::Equal
    }
}

impl From<&StrName> for WxmlToken {
    fn from(value: &StrName) -> Self {
        Self {
            location: value.location.clone(),
            ty: TokenType::String,
            modifier: TokenModifier::None as u32,
        }
    }
}

impl From<&Ident> for WxmlToken {
    fn from(value: &Ident) -> Self {
        Self {
            location: value.location.clone(),
            ty: TokenType::Variable,
            modifier: TokenModifier::None as u32,
        }
    }
}

pub(super) fn find_wxml_semantic_tokens(content: &FileContentMetadata, template: &Template, range: std::ops::Range<Position>) -> Vec<SemanticToken> {
    let mut tokens: Vec<WxmlToken> = vec![];

    // collect in node tree recursively
    fn collect_in_common_attrs(tokens: &mut Vec<WxmlToken>, common: &CommonElementAttributes) {
        if let Some((loc, value)) = common.id.as_ref() {
            tokens.push(WxmlToken { location: loc.clone(), ty: TokenType::Keyword, modifier: 0 });
            collect_in_value(tokens, value);
        }
        if let Some((loc, value)) = common.slot.as_ref() {
            tokens.push(WxmlToken { location: loc.clone(), ty: TokenType::Keyword, modifier: 0 });
            collect_in_value(tokens, value);
        }
        for attr in common.slot_value_refs.iter() {
            if let Some(p) = attr.prefix_location.as_ref() {
                tokens.push(WxmlToken { location: p.clone(), ty: TokenType::Keyword, modifier: 0 });
            }
            tokens.push((&attr.name).into());
            tokens.push(WxmlToken { location: attr.value.location.clone(), ty: TokenType::Function, modifier: 0 });
        }
        for attr in common.data.iter().chain(common.marks.iter()) {
            if let Some(p) = attr.prefix_location.as_ref() {
                tokens.push(WxmlToken { location: p.clone(), ty: TokenType::Keyword, modifier: 0 });
            }
            tokens.push((&attr.name).into());
            if !attr.is_value_unspecified {
                collect_in_value(tokens, &attr.value);
            }
        }
        for attr in common.event_bindings.iter() {
            tokens.push(WxmlToken { location: attr.prefix_location.clone(), ty: TokenType::Keyword, modifier: 0 });
            tokens.push(WxmlToken { location: attr.name.location.clone(), ty: TokenType::Event, modifier: 0 });
            if !attr.is_value_unspecified {
                collect_in_value_with_static_type(tokens, &attr.value, TokenType::Method);
            }
        }
    }
    fn collect_in_nodes(tokens: &mut Vec<WxmlToken>, nodes: &[Node]) {
        for node in nodes {
            match node {
                Node::Text(text) => {
                    collect_in_value_with_static_type(tokens, text, TokenType::String);
                }
                Node::Element(elem) => {
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
                            extra_attr,
                            common,
                            ..
                        } => {
                            let tag_locs = [
                                elem.tag_location.start.0.start..tag_name.location.end,
                                elem.tag_location.start.1.clone(),
                            ];
                            for loc in tag_locs {
                                tokens.push(WxmlToken { location: loc, ty: TokenType::Type, modifier: 0 });
                            }
                            if let Some(x) = elem.tag_location.end.as_ref() {
                                let loc = x.0.start..x.1.end;
                                tokens.push(WxmlToken { location: loc, ty: TokenType::Type, modifier: 0 });
                            } else {
                                tokens.push(WxmlToken { location: elem.tag_location.close.clone(), ty: TokenType::Type, modifier: 0 });
                            }
                            for attr in attributes.iter().chain(change_attributes.iter()) {
                                if let Some(p) = attr.prefix_location.as_ref() {
                                    tokens.push(WxmlToken { location: p.clone(), ty: TokenType::Keyword, modifier: 0 });
                                }
                                tokens.push((&attr.name).into());
                                if !attr.is_value_unspecified {
                                    collect_in_value(tokens, &attr.value);
                                }
                            }
                            for attr in worklet_attributes.iter().chain(generics.iter()).chain(extra_attr.iter()) {
                                if let Some(p) = attr.prefix_location.as_ref() {
                                    tokens.push(WxmlToken { location: p.clone(), ty: TokenType::Keyword, modifier: 0 });
                                }
                                tokens.push((&attr.name).into());
                                tokens.push(WxmlToken { location: attr.value.location.clone(), ty: TokenType::Function, modifier: 0 });
                            }
                            match class {
                                ClassAttribute::None => {}
                                ClassAttribute::String(loc, value) => {
                                    tokens.push(WxmlToken { location: loc.clone(), ty: TokenType::Keyword, modifier: 0 });
                                    collect_in_value(tokens, value);
                                }
                                ClassAttribute::Multiple(list) => {
                                    for (key, value) in list {
                                        tokens.push(key.into());
                                        collect_in_value(tokens, value);
                                    }
                                }
                                _ => {}
                            }
                            match style {
                                StyleAttribute::None => {}
                                StyleAttribute::String(loc, value) => {
                                    tokens.push(WxmlToken { location: loc.clone(), ty: TokenType::Keyword, modifier: 0 });
                                    collect_in_value(tokens, value);
                                }
                                StyleAttribute::Multiple(list) => {
                                    for (key, value) in list {
                                        tokens.push(key.into());
                                        collect_in_value(tokens, value);
                                    }
                                }
                                _ => {}
                            }
                            collect_in_common_attrs(tokens, common);
                            collect_in_nodes(tokens, &children);
                        }
                        ElementKind::Pure { children, slot, slot_value_refs, .. } => {
                            if let Some((loc, value)) = slot.as_ref() {
                                tokens.push(WxmlToken { location: loc.clone(), ty: TokenType::Keyword, modifier: 0 });
                                collect_in_value(tokens, value);
                            }
                            for attr in slot_value_refs.iter() {
                                if let Some(p) = attr.prefix_location.as_ref() {
                                    tokens.push(WxmlToken { location: p.clone(), ty: TokenType::Keyword, modifier: 0 });
                                }
                                tokens.push((&attr.name).into());
                                tokens.push((&attr.value).into());
                            }
                            collect_in_nodes(tokens, &children);
                        }
                        ElementKind::Slot { name, values, common, .. } => {
                            {
                                let (loc, value) = name;
                                tokens.push(WxmlToken { location: loc.clone(), ty: TokenType::Keyword, modifier: 0 });
                                collect_in_value(tokens, value);
                            }
                            for attr in values.iter() {
                                if let Some(p) = attr.prefix_location.as_ref() {
                                    tokens.push(WxmlToken { location: p.clone(), ty: TokenType::Keyword, modifier: 0 });
                                }
                                tokens.push((&attr.name).into());
                                if !attr.is_value_unspecified {
                                    collect_in_value(tokens, &attr.value);
                                }
                            }
                            collect_in_common_attrs(tokens, common);
                        }
                        ElementKind::If { branches, else_branch, .. } => {
                            for (loc, value, nodes) in branches {
                                tokens.push(WxmlToken { location: loc.clone(), ty: TokenType::Keyword, modifier: 0 });
                                collect_in_value(tokens, value);
                                collect_in_nodes(tokens, nodes);
                            }
                            if let Some((loc, nodes)) = else_branch {
                                tokens.push(WxmlToken { location: loc.clone(), ty: TokenType::Keyword, modifier: 0 });
                                collect_in_nodes(tokens, nodes);
                            }
                        }
                        ElementKind::For { list, item_name, index_name, key, children, .. } => {
                            {
                                let (loc, value) = list;
                                tokens.push(WxmlToken { location: loc.clone(), ty: TokenType::Keyword, modifier: 0 });
                                collect_in_value(tokens, value);
                            }
                            for (loc, value) in [item_name, index_name, key] {
                                tokens.push(WxmlToken { location: loc.clone(), ty: TokenType::Keyword, modifier: 0 });
                                let mut t: WxmlToken = value.into();
                                t.ty = TokenType::Variable;
                                t.modifier = TokenModifier::Definition as u32;
                                tokens.push(value.into());
                            }
                            collect_in_nodes(tokens, children);
                        }
                        ElementKind::TemplateRef { target, data, .. } => {
                            for (loc, value) in [target, data] {
                                tokens.push(WxmlToken { location: loc.clone(), ty: TokenType::Keyword, modifier: 0 });
                                collect_in_value(tokens, value);
                            }
                        }
                        ElementKind::Include { path, .. } => {
                            let (loc, value) = path;
                            tokens.push(WxmlToken { location: loc.clone(), ty: TokenType::Keyword, modifier: 0 });
                            tokens.push(value.into());
                        }
                        _ => {}
                    }
                }
                Node::Comment(x) => {
                    tokens.push(WxmlToken { location: x.location.clone(), ty: TokenType::Comment, modifier: 0 });
                }
                Node::UnknownMetaTag(x) => {
                    tokens.push(WxmlToken { location: x.location.clone(), ty: TokenType::Macro, modifier: 0 });
                }
                _ => {}
            }
        }
    }

    // collect in expr recursively
    fn collect_in_value(tokens: &mut Vec<WxmlToken>, value: &Value) {
        collect_in_value_with_static_type(tokens, value, TokenType::String);
    }
    fn collect_in_value_with_static_type(tokens: &mut Vec<WxmlToken>, value: &Value, ty: TokenType) {
        match value {
            Value::Static { location, .. } => {
                tokens.push(WxmlToken { location: location.clone(), ty, modifier: 0 });
            }
            Value::Dynamic { expression, double_brace_location, .. } => {
                tokens.push(WxmlToken { location: double_brace_location.0.clone(), ty: TokenType::Operator, modifier: 0 });
                collect_in_expr(tokens, expression);
                tokens.push(WxmlToken { location: double_brace_location.1.clone(), ty: TokenType::Operator, modifier: 0 });
            }
            _ => {}
        }
    }
    fn collect_in_expr(tokens: &mut Vec<WxmlToken>, expr: &Expression) {
        match expr {
            Expression::Cond { cond, true_br, false_br, question_location, colon_location } => {
                collect_in_expr(tokens, cond);
                tokens.push(WxmlToken { location: question_location.clone(), ty: TokenType::Operator, modifier: 0 });
                collect_in_expr(tokens, &true_br);
                tokens.push(WxmlToken { location: colon_location.clone(), ty: TokenType::Operator, modifier: 0 });
                collect_in_expr(tokens, &false_br);
            }
            Expression::BitAnd { left, right, location } |
            Expression::BitOr { left, right, location } |
            Expression::BitXor { left, right, location } |
            Expression::Divide { left, right, location } |
            Expression::Eq { left, right, location } |
            Expression::EqFull { left, right, location } |
            Expression::Gt { left, right, location } |
            Expression::Gte { left, right, location } |
            Expression::InstanceOf { left, right, location } |
            Expression::LeftShift { left, right, location } |
            Expression::LogicAnd { left, right, location } |
            Expression::LogicOr { left, right, location } |
            Expression::Lt { left, right, location } |
            Expression::Lte { left, right, location } |
            Expression::Minus { left, right, location } |
            Expression::Multiply { left, right, location } |
            Expression::Ne { left, right, location } |
            Expression::NeFull { left, right, location } |
            Expression::NullishCoalescing { left, right, location } |
            Expression::Plus { left, right, location } |
            Expression::Remainer { left, right, location } |
            Expression::RightShift { left, right, location } |
            Expression::UnsignedRightShift { left, right, location } => {
                collect_in_expr(tokens, left);
                tokens.push(WxmlToken { location: location.clone(), ty: TokenType::Operator, modifier: 0 });
                collect_in_expr(tokens, right);
            }
            Expression::BitReverse { value, location } |
            Expression::Negative { value, location } |
            Expression::Positive { value, location } |
            Expression::Reverse { value, location } |
            Expression::TypeOf { value, location } |
            Expression::Void { value, location } => {
                tokens.push(WxmlToken { location: location.clone(), ty: TokenType::Operator, modifier: 0 });
                collect_in_expr(tokens, value);
            }
            Expression::ScopeRef { location, index: _ } => {
                tokens.push(WxmlToken { location: location.clone(), ty: TokenType::Variable, modifier: 0 });
            }
            Expression::DataField { name: _, location } => {
                tokens.push(WxmlToken { location: location.clone(), ty: TokenType::Property, modifier: 0 });
            }
            Expression::ToStringWithoutUndefined { value: _, location } => {
                tokens.push(WxmlToken { location: location.clone(), ty: TokenType::Operator, modifier: 0 });
            }
            Expression::LitUndefined { location } => {
                tokens.push(WxmlToken { location: location.clone(), ty: TokenType::Keyword, modifier: 0 });
            }
            Expression::LitNull { location } => {
                tokens.push(WxmlToken { location: location.clone(), ty: TokenType::Keyword, modifier: 0 });
            }
            Expression::LitStr { value: _, location } => {
                tokens.push(WxmlToken { location: location.clone(), ty: TokenType::String, modifier: 0 });
            }
            Expression::LitBool { value: _, location } => {
                tokens.push(WxmlToken { location: location.clone(), ty: TokenType::Keyword, modifier: 0 });
            }
            Expression::LitFloat { value: _, location } |
            Expression::LitInt { value: _, location } => {
                tokens.push(WxmlToken { location: location.clone(), ty: TokenType::Number, modifier: 0 });
            }
            Expression::LitObj { fields, brace_location } => {
                tokens.push(WxmlToken { location: brace_location.0.clone(), ty: TokenType::Operator, modifier: 0 });
                for field in fields {
                    match field {
                        ObjectFieldKind::Named { name: _, location, colon_location, value } => {
                            tokens.push(WxmlToken { location: location.clone(), ty: TokenType::Property, modifier: 0 });
                            if let Some(loc) = colon_location.as_ref() {
                                tokens.push(WxmlToken { location: loc.clone(), ty: TokenType::Operator, modifier: 0 });
                                collect_in_expr(tokens, value);
                            }
                        }
                        ObjectFieldKind::Spread { location, value } => {
                            tokens.push(WxmlToken { location: location.clone(), ty: TokenType::Operator, modifier: 0 });
                            collect_in_expr(tokens, value);
                        }
                    }
                }
                tokens.push(WxmlToken { location: brace_location.1.clone(), ty: TokenType::Operator, modifier: 0 });
            }
            Expression::LitArr { fields, bracket_location } => {
                tokens.push(WxmlToken { location: bracket_location.0.clone(), ty: TokenType::Operator, modifier: 0 });
                for field in fields {
                    match field {
                        ArrayFieldKind::Normal { value } => {
                            collect_in_expr(tokens, value);
                        }
                        ArrayFieldKind::Spread { location, value } => {
                            tokens.push(WxmlToken { location: location.clone(), ty: TokenType::Operator, modifier: 0 });
                            collect_in_expr(tokens, value);
                        }
                        ArrayFieldKind::EmptySlot => {}
                    }
                }
                tokens.push(WxmlToken { location: bracket_location.1.clone(), ty: TokenType::Operator, modifier: 0 });
            }
            Expression::StaticMember { obj, field_name: _, dot_location, field_location } => {
                collect_in_expr(tokens, obj);
                tokens.push(WxmlToken { location: dot_location.clone(), ty: TokenType::Operator, modifier: 0 });
                tokens.push(WxmlToken { location: field_location.clone(), ty: TokenType::Property, modifier: 0 });
            }
            Expression::DynamicMember { obj, field_name, bracket_location } => {
                collect_in_expr(tokens, obj);
                tokens.push(WxmlToken { location: bracket_location.0.clone(), ty: TokenType::Operator, modifier: 0 });
                collect_in_expr(tokens, &field_name);
                tokens.push(WxmlToken { location: bracket_location.1.clone(), ty: TokenType::Operator, modifier: 0 });
            }
            Expression::FuncCall { func, args, paren_location } => {
                collect_in_expr(tokens, &func);
                tokens.push(WxmlToken { location: paren_location.0.clone(), ty: TokenType::Operator, modifier: 0 });
                for arg in args {
                    collect_in_expr(tokens, &arg);
                }
                tokens.push(WxmlToken { location: paren_location.1.clone(), ty: TokenType::Operator, modifier: 0 });
            }
            _ => {}
        }
    }

    // collect all tokens from tree
    for i in template.globals.imports.iter() {
        tokens.push(WxmlToken { location: i.src_location.clone(), ty: TokenType::Keyword, modifier: 0 });
        let mut t: WxmlToken = (&i.src).into();
        t.modifier = TokenModifier::Declaration as u32;
        tokens.push(t);
    }
    for i in template.globals.includes.iter() {
        tokens.push(WxmlToken { location: i.src_location.clone(), ty: TokenType::Keyword, modifier: 0 });
        let mut t: WxmlToken = (&i.src).into();
        t.modifier = TokenModifier::Declaration as u32;
        tokens.push(t);
    }
    for i in template.globals.scripts.iter() {
        tokens.push(WxmlToken { location: i.module_location(), ty: TokenType::Keyword, modifier: 0 });
        let mut t: WxmlToken = i.module_name().into();
        t.modifier = TokenModifier::Definition as u32;
        tokens.push(t);
        match i {
            Script::Inline { .. } => {
                // TODO pass to wxs ls
            }
            Script::GlobalRef { src_location, src, .. } => {
                tokens.push(WxmlToken { location: src_location.clone(), ty: TokenType::Keyword, modifier: 0 });
                tokens.push(src.into());
            }
            _ => {}
        }
    }
    for sub in template.globals.sub_templates.iter() {
        tokens.push(WxmlToken { location: sub.name_location.clone(), ty: TokenType::Keyword, modifier: 0 });
        let mut t: WxmlToken = (&sub.name).into();
        t.modifier = TokenModifier::Definition as u32;
        tokens.push(t);
        collect_in_nodes(&mut tokens, &sub.content);
    }
    collect_in_nodes(&mut tokens, &template.content);

    // construct LSP results
    let mut gen = SemanticTokenGenerator::new(range);
    tokens.sort();
    let mut tokens = tokens.into_iter();
    while let Some(t) = tokens.next() {
        gen.push(content, t.location, t.ty, t.modifier);
    }
    gen.finish()
}

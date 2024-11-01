use std::cmp::Ordering;

use glass_easel_template_compiler::parse::{expr::{ArrayFieldKind, Expression, ObjectFieldKind}, tag::{ClassAttribute, CommonElementAttributes, ElementKind, Ident, Node, StrName, StyleAttribute, Value}, Position, Template};
use lsp_types::{SemanticToken, SemanticTokenModifier, SemanticTokenType, SemanticTokens, SemanticTokensParams, SemanticTokensRangeParams};

use crate::{context::project::FileContentMetadata, ServerContext};

pub(crate) const TOKEN_TYPES: [SemanticTokenType; 13] = [
    SemanticTokenType::TYPE,
    SemanticTokenType::CLASS,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::PROPERTY,
    SemanticTokenType::EVENT,
    SemanticTokenType::FUNCTION,
    SemanticTokenType::METHOD,
    SemanticTokenType::KEYWORD,
    SemanticTokenType::COMMENT,
    SemanticTokenType::STRING,
    SemanticTokenType::NUMBER,
    SemanticTokenType::OPERATOR,
    SemanticTokenType::MACRO,
];

// this list MUST matches the list above
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
enum TokenType {
    Type = 0,
    Class,
    Variable,
    Property,
    Event,
    Function,
    Method,
    Keyword,
    Comment,
    String,
    Number,
    Operator,
    Macro,
}

pub(crate) const TOKEN_MODIFIERS: [SemanticTokenModifier; 3] = [
    SemanticTokenModifier::DECLARATION,
    SemanticTokenModifier::DEFINITION,
    SemanticTokenModifier::DEPRECATED,
];

// this list MUST matches the list above
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
enum TokenModifier {
    None = 0x00000000,
    Declaration = 0x00000001,
    Definition = 0x00000002,
    Deprecated = 0x00000004,
}

pub(crate) async fn tokens_full(ctx: ServerContext, params: SemanticTokensParams) -> anyhow::Result<SemanticTokens> {
    let ret = ctx.clone().project_thread_task(&params.text_document.uri, move |project, abs_path| -> anyhow::Result<_> {
        let data = if let Some(content) = project.get_file_content(&abs_path) {
            match abs_path.extension().and_then(|x| x.to_str()) {
                Some("wxml") => {
                    let template = project.get_wxml_tree(&abs_path)?;
                    let range = Position { line: 0, utf16_col: 0 }..Position { line: u32::MAX, utf16_col: u32::MAX };
                    find_wxml_semantic_tokens(content, template, range)
                }
                _ => vec![],
            }
        } else {
            vec![]
        };
        Ok(SemanticTokens { result_id: None, data })
    }).await??;
    Ok(ret)
}

pub(crate) async fn tokens_range(ctx: ServerContext, params: SemanticTokensRangeParams) -> anyhow::Result<SemanticTokens> {
    let ret = ctx.clone().project_thread_task(&params.text_document.uri, move |project, abs_path| -> anyhow::Result<_> {
        let data = if let Some(content) = project.get_file_content(&abs_path) {
            match abs_path.extension().and_then(|x| x.to_str()) {
                Some("wxml") => {
                    let template = project.get_wxml_tree(&abs_path)?;
                    let start = Position { line: params.range.start.line, utf16_col: params.range.start.character };
                    let end = Position { line: params.range.end.line, utf16_col: params.range.end.character };
                    find_wxml_semantic_tokens(content, template, start..end)
                }
                _ => vec![],
            }
        } else {
            vec![]
        };
        Ok(SemanticTokens { result_id: None, data })
    }).await??;
    Ok(ret)
}

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

fn find_wxml_semantic_tokens(content: &FileContentMetadata, template: &Template, range: std::ops::Range<Position>) -> Vec<SemanticToken> {
    let mut tokens: Vec<WxmlToken> = vec![];
    let start_bound = WxmlToken {
        location: range.start..range.start,
        ty: TokenType::Comment,
        modifier: 0,
    };
    let end_bound = WxmlToken {
        location: range.end..range.end,
        ty: TokenType::Comment,
        modifier: 0,
    };

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
                        } => {
                            let tag_locs = [
                                elem.start_tag_location.0.start..tag_name.location.end,
                                elem.start_tag_location.1.clone(),
                            ];
                            for loc in tag_locs {
                                tokens.push(WxmlToken { location: loc, ty: TokenType::Type, modifier: 0 });
                            }
                            if let Some(x) = elem.end_tag_location.as_ref() {
                                let loc = x.0.start..x.1.end;
                                tokens.push(WxmlToken { location: loc, ty: TokenType::Type, modifier: 0 });
                            } else {
                                tokens.push(WxmlToken { location: elem.close_location.clone(), ty: TokenType::Type, modifier: 0 });
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
                            }
                            collect_in_common_attrs(tokens, common);
                            collect_in_nodes(tokens, &children);
                        }
                        ElementKind::Pure { children, slot, slot_value_refs } => {
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
                        ElementKind::Slot { name, values, common } => {
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
                        ElementKind::If { branches, else_branch } => {
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
                        ElementKind::For { list, item_name, index_name, key, children } => {
                            {
                                let (loc, value) = list;
                                tokens.push(WxmlToken { location: loc.clone(), ty: TokenType::Keyword, modifier: 0 });
                                collect_in_value(tokens, value);
                            }
                            for (loc, value) in [item_name, index_name, key] {
                                tokens.push(WxmlToken { location: loc.clone(), ty: TokenType::Keyword, modifier: 0 });
                                tokens.push(value.into());
                            }
                            collect_in_nodes(tokens, children);
                        }
                        ElementKind::TemplateRef { target, data } => {
                            for (loc, value) in [target, data] {
                                tokens.push(WxmlToken { location: loc.clone(), ty: TokenType::Keyword, modifier: 0 });
                                collect_in_value(tokens, value);
                            }
                        }
                        ElementKind::Include { path } => {
                            let (loc, value) = path;
                            tokens.push(WxmlToken { location: loc.clone(), ty: TokenType::Keyword, modifier: 0 });
                            tokens.push(value.into());
                        }
                    }
                }
                Node::Comment(_, location) => {
                    tokens.push(WxmlToken { location: location.clone(), ty: TokenType::Comment, modifier: 0 });
                }
                Node::UnknownMetaTag(_, location) => {
                    tokens.push(WxmlToken { location: location.clone(), ty: TokenType::Macro, modifier: 0 });
                }
            }
        }
    }

    // collect in expr recursively
    fn collect_in_value(tokens: &mut Vec<WxmlToken>, value: &Value) {
        collect_in_value_with_static_type(tokens, value, TokenType::String);
    }
    fn collect_in_value_with_static_type(tokens: &mut Vec<WxmlToken>, value: &Value, ty: TokenType) {
        match value {
            Value::Static { value: _, location } => {
                tokens.push(WxmlToken { location: location.clone(), ty, modifier: 0 });
            }
            Value::Dynamic { expression, double_brace_location, binding_map_keys: _ } => {
                tokens.push(WxmlToken { location: double_brace_location.0.clone(), ty: TokenType::Operator, modifier: 0 });
                collect_in_expr(tokens, expression);
                tokens.push(WxmlToken { location: double_brace_location.1.clone(), ty: TokenType::Operator, modifier: 0 });
            }
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
        }
    }

    // collect all tokens from tree
    for i in template.globals.imports.iter() {
        let mut t: WxmlToken = i.into();
        t.modifier = TokenModifier::Declaration as u32;
        tokens.push(t);
    }
    for i in template.globals.includes.iter() {
        let mut t: WxmlToken = i.into();
        t.modifier = TokenModifier::Declaration as u32;
        tokens.push(t);
    }
    for i in template.globals.scripts.iter() {
        let mut t: WxmlToken = i.module_name().into();
        t.modifier = TokenModifier::Definition as u32;
        tokens.push(t);
    }
    for (name, nodes) in template.globals.sub_templates.iter() {
        tokens.push(name.into());
        collect_in_nodes(&mut tokens, nodes);
    }
    collect_in_nodes(&mut tokens, &template.content);

    // construct LSP results
    tokens.sort();
    let mut rel_line = 0;
    let mut rel_col = 0;
    let mut ret: Vec<SemanticToken> = vec![];
    for t in tokens {
        if t >= end_bound { break; }
        let t_end = WxmlToken {
            location: t.location.end..t.location.end,
            ty: TokenType::Comment,
            modifier: 0,
        };
        if t_end < start_bound {
            continue;
        }
        for line in t.location.start.line..=t.location.end.line {
            let start = if line == t.location.start.line { t.location.start.utf16_col } else { 0 };
            let end = if line == t.location.end.line { t.location.end.utf16_col } else { content.get_line_utf16_len(line) };
            let length = end - start;
            if length == 0 { continue; }
            let delta_line = line - rel_line;
            let delta_start = start - if delta_line > 0 { 0 } else { rel_col };
            ret.push(SemanticToken {
                delta_line,
                delta_start,
                length,
                token_type: t.ty as u32,
                token_modifiers_bitset: t.modifier,
            });
            rel_line = line;
            rel_col = start;
        }
        // let length = if t.location.start.line == t.location.end.line {
        //     t.location.end.utf16_col - t.location.start.utf16_col
        // } else {
        //     let start_idx = content.content_index_for_line_utf16_col(t.location.start.line, t.location.start.utf16_col);
        //     let end_idx = content.content_index_for_line_utf16_col(t.location.end.line, t.location.end.utf16_col);
        //     eprintln!("!!! multiline {:?} = {:?}", start_idx..end_idx, content.content[start_idx..end_idx].chars().map(|x| x.len_utf16() as u32).sum::<u32>());
        //     content.content[start_idx..end_idx].chars().map(|x| x.len_utf16() as u32).sum()
        // };
        // if length == 0 { continue; }
        // let delta_line = t.location.start.line - rel_line;
        // let delta_start = t.location.start.utf16_col - if delta_line > 0 { 0 } else { rel_col };
        // ret.push(SemanticToken {
        //     delta_line,
        //     delta_start,
        //     length,
        //     token_type: t.ty as u32,
        //     token_modifiers_bitset: t.modifier,
        // });
        // rel_line = t.location.start.line;
        // rel_col = t.location.start.utf16_col;
    }

    ret
}

use std::{collections::HashMap, path::{Path, PathBuf}, sync::Arc};

use futures::StreamExt;
use glass_easel_template_compiler::{parse::{tag::{ElementKind, TemplateDefinition, Value}, ParseError, ParseErrorKind, ParseErrorLevel, Template}, TmplGroup};
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use tokio::sync::Mutex as AsyncMutex;

use crate::wxss::{self, StyleSheet};

pub(crate) struct FileContentMetadata {
    opened: bool,
    pub(crate) content: String,
    pub(crate) line_starts: Vec<usize>,
}

impl FileContentMetadata {
    fn new(content: String) -> Self {
        FileContentMetadata {
            opened: false,
            content,
            line_starts: vec![],
        }
    }

    fn open(&mut self) {
        self.opened = true;
        let mut line_starts = vec![];
        line_starts.push(0);
        let mut iter = self.content.as_bytes().iter().enumerate();
        while let Some((idx, byte)) = iter.next() {
            let byte = *byte;
            if byte == b'\n' {
                line_starts.push(idx + 1);
            } else if byte == b'\r' {
                if self.content.as_bytes()[idx + 1] != b'\n' {
                    line_starts.push(idx + 1);
                }
            }
        }
        self.line_starts = line_starts;
    }

    fn close(&mut self) {
        self.opened = false;
        self.line_starts.truncate(0);
    }

    pub(crate) fn get_line_utf16_len(&self, line: u32) -> u32 {
        let Some(start) = self.line_starts.get(line as usize).cloned() else {
            return 0;
        };
        let end = self.line_starts.get(line as usize + 1).cloned().unwrap_or(self.content.len());
        self.content[start..end]
            .chars()
            .map(|ch| ch.len_utf16() as u32)
            .sum()
    }

    pub(crate) fn content_index_for_line_utf16_col(&self, line: u32, utf16_col: u32) -> usize {
        let Some(line_start) = self.line_starts.get(line as usize).cloned() else {
            return self.content.len();
        };
        let mut col = utf16_col as usize;
        for (idx, ch) in self.content[line_start..].char_indices() {
            if col < ch.len_utf16() {
                return line_start + idx;
            }
            col -= ch.len_utf16();
        }
        self.content.len()
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JsonConfig {
    #[serde(default)]
    #[allow(dead_code)]
    pub(crate) component: bool,
    #[serde(default)]
    #[allow(dead_code)]
    pub(crate) using_components: HashMap<String, String>,
}

pub(crate) struct Project {
    root: PathBuf,
    file_contents: HashMap<PathBuf, FileContentMetadata>,
    json_config_map: HashMap<PathBuf, JsonConfig>,
    template_group: TmplGroup,
    style_sheet_map: HashMap<PathBuf, StyleSheet>,
}

impl Project {
    pub(crate) async fn search_projects(root: &Path, ignore: &[PathBuf]) -> Vec<Self> {
        async fn rec(ret: Arc<AsyncMutex<&mut Vec<Project>>>, p: &Path, ignore: &[PathBuf]) -> anyhow::Result<()> {
            if ignore.iter().map(|x| x.as_path()).find(|x| *x == p).is_some() { return Ok(()) };
            let app_json = p.join("app.json");
            let app_wxss = p.join("app.wxss");
            let contains = tokio::fs::metadata(&app_json).await.map(|x| x.is_file()).unwrap_or(false)
                || tokio::fs::metadata(&app_wxss).await.map(|x| x.is_file()).unwrap_or(false);
            if contains {
                ret.lock().await.push(Project::new(p));
                return Ok(());
            }
            let dir = tokio_stream::wrappers::ReadDirStream::new(tokio::fs::read_dir(p).await?);
            dir.for_each_concurrent(None, |entry| {
                let ret = ret.clone();
                async move {
                    let Ok(entry) = entry else { return };
                    let Ok(ty) = entry.file_type().await else { return };
                    let abs_path = entry.path();
                    if ty.is_dir() {
                        let _ = rec(ret, &abs_path, ignore).await;
                        return;
                    }
                }
            }).await;
            Ok(())
        }
        let mut ret = vec![];
        let _ = rec(Arc::new(AsyncMutex::new(&mut ret)), root, ignore).await;
        ret
    }

    pub(crate) fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            file_contents: HashMap::new(),
            json_config_map: HashMap::new(),
            template_group: TmplGroup::new(),
            style_sheet_map: HashMap::new(),
        }
    }

    pub(crate) async fn init(&mut self) {
        let _ = self.load_all_files_from_fs().await;
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn unix_rel_path(&self, abs_path: &Path) -> anyhow::Result<String> {
        crate::utils::unix_rel_path(&self.root, abs_path)
    }

    pub(crate) fn find_rel_path_for_file(&self, abs_path: &Path, rel_path: &str) -> anyhow::Result<PathBuf> {
        let p = abs_path.parent().unwrap_or(abs_path);
        crate::utils::join_unix_rel_path(p, rel_path, &self.root)
    }

    pub(crate) fn file_created_or_changed(&mut self, abs_path: &Path) {
        if let Some(content_meta) = self.file_contents.get(abs_path) {
            if !content_meta.opened {
                let _ = self.load_file_from_fs(abs_path);
            }
        }
    }

    pub(crate) fn file_removed(&mut self, abs_path: &Path) {
        if let Some(content_meta) = self.file_contents.get(abs_path) {
            if !content_meta.opened {
                match abs_path.extension().and_then(|x| x.to_str()) {
                    Some("wxml") => {
                        let _ = self.cleanup_wxml(abs_path);
                    }
                    Some("wxss") => {
                        let _ = self.cleanup_wxss(abs_path);
                    }
                    Some("json") => {
                        let _ = self.cleanup_json(abs_path);
                    }
                    _ => {}
                }
            }
        }
    }

    fn load_file_from_fs(&mut self, abs_path: &Path) -> anyhow::Result<()> {
        match abs_path.extension().and_then(|x| x.to_str()) {
            Some("wxml") => {
                let content = std::fs::read_to_string(abs_path)?;
                self.update_wxml(abs_path, content)?;
            }
            Some("wxss") => {
                let content = std::fs::read_to_string(abs_path)?;
                self.update_wxss(abs_path, content)?;
            }
            Some("json") => {
                let content = std::fs::read_to_string(abs_path)?;
                self.update_json(abs_path, content)?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn load_all_files_from_fs(&mut self) -> anyhow::Result<()> {
        async fn rec(project: Arc<AsyncMutex<&mut Project>>, p: &Path) -> anyhow::Result<()> {
            let dir = tokio_stream::wrappers::ReadDirStream::new(tokio::fs::read_dir(p).await?);
            dir.for_each_concurrent(None, |entry| {
                let project = project.clone();
                async move {
                    let Ok(entry) = entry else { return };
                    let Ok(ty) = entry.file_type().await else { return };
                    let abs_path = entry.path();
                    if ty.is_dir() {
                        let _ = rec(project, &abs_path).await;
                        return;
                    }
                    if ty.is_file() {
                        let Some(ext) = abs_path.extension().and_then(|x| x.to_str()) else { return };
                        match ext {
                            "wxml" | "wxss" | "json" => {}
                            _ => { return; }
                        }
                        let Ok(content) = tokio::fs::read_to_string(&abs_path).await else { return };
                        let mut project = project.lock().await;
                        match ext {
                            "wxml" => {
                                let _ = project.update_wxml(&abs_path, content);
                            }
                            "wxss" => {
                                let _ = project.update_wxss(&abs_path, content);
                            }
                            "json" => {
                                let _ = project.update_json(&abs_path, content);
                            }
                            _ => unreachable!()
                        }
                    }
                }
            }).await;
            Ok(())
        }
        let root = self.root.to_path_buf();
        let proj = Arc::new(AsyncMutex::new(self));
        rec(proj, &root).await
    }

    pub(crate) fn _file_content(&mut self, abs_path: &Path) -> Option<&FileContentMetadata> {
        if self.file_contents.contains_key(abs_path) {
            return self.file_contents.get(abs_path);
        }
        self.load_file_from_fs(abs_path).ok()?;
        self.file_contents.get(abs_path)
    }

    pub(crate) fn cached_file_content(&self, abs_path: &Path) -> Option<&FileContentMetadata> {
        self.file_contents.get(abs_path)
    }

    fn update_json(&mut self, abs_path: &Path, content: String) -> anyhow::Result<Vec<Diagnostic>> {
        let mut ret = vec![];
        let json_config: Result<JsonConfig, _> = serde_json::from_str(&content);
        match json_config {
            Ok(json_config) => {
                self.json_config_map.insert(abs_path.to_path_buf(), json_config);
            }
            Err(err) => {
                let pos = Position::new(err.line().saturating_sub(1) as u32, err.column().saturating_sub(1) as u32);
                ret.push(Diagnostic {
                    range: Range { start: pos, end: pos },
                    message: err.to_string(),
                    severity: Some(DiagnosticSeverity::ERROR),
                    ..Default::default()
                });
                self.json_config_map.insert(abs_path.to_path_buf(), Default::default());
            }
        }
        Ok(ret)
    }

    fn cleanup_json(&mut self, abs_path: &Path) -> anyhow::Result<()> {
        self.json_config_map.remove(abs_path);
        self.file_contents.remove(abs_path);
        Ok(())
    }

    pub(crate) fn open_json(&mut self, abs_path: &Path, content: String) -> anyhow::Result<Vec<Diagnostic>> {
        let diagnostics = self.update_json(abs_path, content)?;
        if let Some(x) = self.file_contents.get_mut(abs_path) {
            x.open();
        }
        Ok(diagnostics)
    }

    pub(crate) fn close_json(&mut self, abs_path: &Path) -> anyhow::Result<()> {
        if let Some(x) = self.file_contents.get_mut(abs_path) {
            x.close();
        }
        if std::fs::metadata(abs_path).ok().map(|x| x.is_file()) != Some(true) {
            self.cleanup_json(abs_path)?;
        }
        Ok(())
    }

    pub(crate) fn get_json_config(&self, abs_path: &Path) -> Option<&JsonConfig> {
        self.json_config_map.get(abs_path)
    }

    fn update_wxss(&mut self, abs_path: &Path, content: String) -> anyhow::Result<Vec<Diagnostic>> {
        let (ss, err_list) = StyleSheet::parse_str(&content);
        self.file_contents.insert(abs_path.to_path_buf(), FileContentMetadata::new(content));
        self.style_sheet_map.insert(abs_path.to_path_buf(), ss);
        let diagnostics = err_list.into_iter().filter_map(diagnostic_from_wxss_parse_error).collect();
        Ok(diagnostics)
    }

    fn cleanup_wxss(&mut self, abs_path: &Path) -> anyhow::Result<()> {
        self.json_config_map.remove(abs_path);
        self.file_contents.remove(abs_path);
        Ok(())
    }

    pub(crate) fn open_wxss(&mut self, abs_path: &Path, content: String) -> anyhow::Result<Vec<Diagnostic>> {
        let diagnostics = self.update_wxss(abs_path, content)?;
        if let Some(x) = self.file_contents.get_mut(abs_path) {
            x.open();
        }
        Ok(diagnostics)
    }

    pub(crate) fn close_wxss(&mut self, abs_path: &Path) -> anyhow::Result<()> {
        if let Some(x) = self.file_contents.get_mut(abs_path) {
            x.close();
        }
        if std::fs::metadata(abs_path).ok().map(|x| x.is_file()) != Some(true) {
            self.cleanup_wxss(abs_path)?;
        }
        Ok(())
    }

    pub(crate) fn get_style_sheet(&self, abs_path: &Path) -> anyhow::Result<&StyleSheet> {
        let tree = self.style_sheet_map.get(abs_path).ok_or_else(|| anyhow::Error::msg("no such style sheet"))?;
        Ok(tree)
    }

    fn update_wxml(&mut self, abs_path: &Path, content: String) -> anyhow::Result<Vec<Diagnostic>> {
        let tmpl_path = self.unix_rel_path(&abs_path)?;
        let err_list = self.template_group.add_tmpl(&tmpl_path, &content);
        self.file_contents.insert(abs_path.to_path_buf(), FileContentMetadata::new(content));
        let diagnostics = err_list.into_iter().filter_map(diagnostic_from_wxml_parse_error).collect();
        Ok(diagnostics)
    }

    fn cleanup_wxml(&mut self, abs_path: &Path) -> anyhow::Result<()> {
        let tmpl_path = self.unix_rel_path(&abs_path)?;
        self.template_group.remove_tmpl(&tmpl_path);
        self.file_contents.remove(abs_path);
        Ok(())
    }

    pub(crate) fn open_wxml(&mut self, abs_path: &Path, content: String) -> anyhow::Result<Vec<Diagnostic>> {
        let diagnostics = self.update_wxml(abs_path, content)?;
        if let Some(x) = self.file_contents.get_mut(abs_path) {
            x.open();
        }
        Ok(diagnostics)
    }

    pub(crate) fn close_wxml(&mut self, abs_path: &Path) -> anyhow::Result<()> {
        if let Some(x) = self.file_contents.get_mut(abs_path) {
            x.close();
        }
        if std::fs::metadata(abs_path).ok().map(|x| x.is_file()) != Some(true) {
            self.cleanup_wxml(abs_path)?;
        }
        Ok(())
    }

    pub(crate) fn get_wxml_tree(&self, abs_path: &Path) -> anyhow::Result<&Template> {
        let tmpl_path = self.unix_rel_path(&abs_path)?;
        let tree = self.template_group.get_tree(&tmpl_path)?;
        Ok(tree)
    }

    pub(crate) fn for_each_json_config(&self, mut f: impl FnMut(&Path, &JsonConfig)) {
        for (p, json_config) in self.json_config_map.iter() {
            f(p, json_config);
        }
    }

    pub(crate) fn get_target_component_path(&self, abs_path: &Path, tag_name: &str) -> Option<PathBuf> {
        let json_path = abs_path.with_extension("json");
        let Some(json_config) = self.get_json_config(&json_path) else {
            return None;
        };
        let Some(rel_path) = json_config.using_components.get(tag_name) else {
            return None;
        };
        self.find_rel_path_for_file(&json_path, rel_path).ok()
    }

    pub(crate) fn search_component_wxml_usages(&self, abs_path: &Path, tag_name: &str, mut f: impl FnMut(&Path, &Template, &str)) {
        if let Some(expected_target) = self.get_target_component_path(abs_path, &tag_name) {
            self.for_each_json_config(|p, json_config| {
                for (expected_tag_name, rel_path) in json_config.using_components.iter() {
                    let Ok(target) = self.find_rel_path_for_file(p, &rel_path) else {
                        continue;
                    };
                    if target == expected_target {
                        let source_wxml = p.with_extension("wxml");
                        if let Ok(template) = self.get_wxml_tree(&source_wxml) {
                            f(&source_wxml, template, &expected_tag_name);
                        }
                    }
                }
            });
        }
    }

    pub(crate) fn get_target_template_path(&self, abs_path: &Path, source_tamplate: &Template, is: &str) -> Option<(PathBuf, &TemplateDefinition)> {
        for import in source_tamplate.globals.imports.iter().rev() {
            if let Ok(p) = self.find_rel_path_for_file(abs_path, &import.src.name) {
                let Some(imported_path) = crate::utils::ensure_file_extension(&p, "wxml") else {
                    continue;
                };
                if let Ok(imported_template) = self.get_wxml_tree(&imported_path) {
                    if let Some(x) = imported_template.globals.sub_templates.iter().rfind(|x| x.name.is(is)) {
                        return Some((imported_path.to_path_buf(), x));
                    }
                }
            }
        }
        None
    }

    pub(crate) fn get_wxml_template_names(&self, abs_path: &Path) -> Option<Vec<String>> {
        let Ok(tree) = self.get_wxml_tree(&abs_path) else {
            return None;
        };
        let mut names: Vec<_> = tree.globals.sub_templates.iter().map(|x| x.name.name.to_string()).collect();
        for import in tree.globals.imports.iter() {
            if let Ok(p) = self.find_rel_path_for_file(abs_path, &import.src.name) {
                if let Some(p) = crate::utils::ensure_file_extension(&p, "wxml") {
                    if let Ok(tree) = self.get_wxml_tree(&p) {
                        for item in tree.globals.sub_templates.iter() {
                            names.push(item.name.name.to_string());
                        }
                    }
                }
            }
        }
        Some(names)
    }

    pub(crate) fn search_wxml_template_usages(&self, abs_path: &Path, is: &str, mut f: impl FnMut(&Path, std::ops::Range<glass_easel_template_compiler::parse::Position>)) {
        for (source_p, tree) in self.template_group.list_template_trees() {
            let Ok(source_p) = crate::utils::join_unix_rel_path(&self.root, source_p, &self.root) else { continue };
            if let Some((p, _)) = self.get_target_template_path(&source_p, tree, is) {
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
}

fn diagnostic_from_wxml_parse_error(x: ParseError) -> Option<Diagnostic> {
    if x.kind == ParseErrorKind::UnknownMetaTag { return None; }
    Some(Diagnostic {
        range: Range {
            start: Position { line: x.location.start.line, character: x.location.start.utf16_col },
            end: Position { line: x.location.end.line, character: x.location.end.utf16_col },
        },
        severity: Some(match x.level() {
            ParseErrorLevel::Fatal => DiagnosticSeverity::ERROR,
            ParseErrorLevel::Error => DiagnosticSeverity::ERROR,
            ParseErrorLevel::Warn => DiagnosticSeverity::WARNING,
            ParseErrorLevel::Note => DiagnosticSeverity::INFORMATION,
        }),
        code: Some(lsp_types::NumberOrString::Number(x.code() as i32)),
        message: x.kind.to_string(),
        ..Default::default()
    })
}

fn diagnostic_from_wxss_parse_error(x: wxss::ParseError) -> Option<Diagnostic> {
    Some(Diagnostic {
        range: Range {
            start: Position { line: x.location.start.line, character: x.location.start.utf16_col },
            end: Position { line: x.location.end.line, character: x.location.end.utf16_col },
        },
        severity: Some(match x.level() {
            ParseErrorLevel::Fatal => DiagnosticSeverity::ERROR,
            ParseErrorLevel::Error => DiagnosticSeverity::ERROR,
            ParseErrorLevel::Warn => DiagnosticSeverity::WARNING,
            ParseErrorLevel::Note => DiagnosticSeverity::INFORMATION,
        }),
        code: Some(lsp_types::NumberOrString::Number(x.code() as i32)),
        message: x.kind.to_string(),
        ..Default::default()
    })
}

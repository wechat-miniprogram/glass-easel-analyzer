use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

use futures::StreamExt;
use glass_easel_template_compiler::{
    parse::{ParseError, ParseErrorKind, ParseErrorLevel, Template}, TmplConvertedExpr, TmplGroup
};
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use tokio::sync::Mutex as AsyncMutex;

use super::{FileLang, ServerContextOptions};
use crate::wxss::{self, Location, StyleSheet};

#[derive(Debug)]
pub(crate) struct FileContentMetadata {
    opened: bool,
    pub(crate) file_lang: FileLang,
    pub(crate) content: String,
    pub(crate) line_starts: Vec<usize>,
}

impl FileContentMetadata {
    fn new(content: String, file_lang: FileLang) -> Self {
        FileContentMetadata {
            opened: false,
            file_lang,
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
        let end = self
            .line_starts
            .get(line as usize + 1)
            .cloned()
            .unwrap_or(self.content.len());
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

    pub(crate) fn line_utf16_col_for_content_index(&self, index: usize) -> (u32, u32) {
        let line = self.line_starts.partition_point(|x| index >= *x) - 1;
        let utf16_col = self.content[self.line_starts[line]..index]
            .encode_utf16()
            .count();
        (line as u32, utf16_col as u32)
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
    root: Option<PathBuf>,
    file_contents: HashMap<PathBuf, FileContentMetadata>,
    json_config_map: HashMap<PathBuf, JsonConfig>,
    template_group: TmplGroup,
    cached_wxml_converted_expr: HashMap<String, TmplConvertedExpr>,
    style_sheet_map: HashMap<PathBuf, StyleSheet>,
    enable_other_ss: bool,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            root: None,
            file_contents: HashMap::new(),
            json_config_map: HashMap::new(),
            template_group: TmplGroup::new(),
            cached_wxml_converted_expr: HashMap::new(),
            style_sheet_map: HashMap::new(),
            enable_other_ss: false,
        }
    }
}

impl Project {
    pub(crate) async fn search_projects(root: &Path, options: &ServerContextOptions) -> Vec<Self> {
        async fn rec(
            ret: Arc<AsyncMutex<&mut Vec<Project>>>,
            p: &Path,
            options: &ServerContextOptions,
        ) -> anyhow::Result<()> {
            if options
                .ignore_paths
                .iter()
                .map(|x| x.as_path())
                .find(|x| *x == p)
                .is_some()
            {
                return Ok(());
            };
            let app_json = p.join("app.json");
            let app_wxss = p.join("app.wxss");
            let contains = tokio::fs::metadata(&app_json)
                .await
                .map(|x| x.is_file())
                .unwrap_or(false)
                || tokio::fs::metadata(&app_wxss)
                    .await
                    .map(|x| x.is_file())
                    .unwrap_or(false);
            if contains {
                ret.lock().await.push(Project::new(p, options));
                return Ok(());
            }
            let dir = tokio_stream::wrappers::ReadDirStream::new(tokio::fs::read_dir(p).await?);
            dir.for_each_concurrent(None, |entry| {
                let ret = ret.clone();
                async move {
                    let Ok(entry) = entry else { return };
                    let Ok(ty) = entry.file_type().await else {
                        return;
                    };
                    let abs_path = entry.path();
                    if ty.is_dir() {
                        let _ = rec(ret, &abs_path, options).await;
                        return;
                    }
                }
            })
            .await;
            Ok(())
        }
        let mut ret = vec![];
        let _ = rec(Arc::new(AsyncMutex::new(&mut ret)), root, options).await;
        ret
    }

    pub(crate) fn new(root: &Path, options: &ServerContextOptions) -> Self {
        Self {
            root: Some(root.to_path_buf()),
            file_contents: HashMap::new(),
            json_config_map: HashMap::new(),
            template_group: TmplGroup::new(),
            cached_wxml_converted_expr: HashMap::new(),
            style_sheet_map: HashMap::new(),
            enable_other_ss: options.enable_other_ss,
        }
    }

    pub(crate) async fn init(&mut self) {
        let _ = self.load_all_files_from_fs().await;
    }

    pub(crate) fn root(&self) -> Option<&Path> {
        self.root.as_ref().map(|x| x.as_path())
    }

    fn unix_rel_path_or_fallback(&self, abs_path: &Path) -> String {
        self.root
            .as_ref()
            .and_then(|root| crate::utils::unix_rel_path(root, abs_path).ok())
            .unwrap_or_else(|| {
                abs_path
                    .components()
                    .map(|x| x.as_os_str().to_str().unwrap_or_default())
                    .collect::<Vec<_>>()
                    .join("/")
            })
    }

    pub(crate) fn find_rel_path_for_file(
        &self,
        abs_path: &Path,
        rel_path: &str,
    ) -> Option<PathBuf> {
        let p = abs_path.parent().unwrap_or(abs_path);
        crate::utils::join_unix_rel_path(p, rel_path, self.root()?).ok()
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
                match content_meta.file_lang {
                    FileLang::Wxml => {
                        let _ = self.cleanup_wxml(abs_path);
                    }
                    FileLang::Wxss => {
                        let _ = self.cleanup_wxss(abs_path);
                    }
                    FileLang::Json => {
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
                self.update_wxss(abs_path, content, false)?;
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
        async fn rec(
            project: Arc<AsyncMutex<&mut Project>>,
            p: &Path,
            enable_other_ss: bool,
        ) -> anyhow::Result<()> {
            let dir = tokio_stream::wrappers::ReadDirStream::new(tokio::fs::read_dir(p).await?);
            dir.for_each_concurrent(None, |entry| {
                let project = project.clone();
                async move {
                    let Ok(entry) = entry else { return };
                    let Ok(ty) = entry.file_type().await else {
                        return;
                    };
                    let abs_path = entry.path();
                    if ty.is_dir() {
                        let _ = rec(project, &abs_path, enable_other_ss).await;
                        return;
                    }
                    if ty.is_file() {
                        let Some(ext) = abs_path.extension().and_then(|x| x.to_str()) else {
                            return;
                        };
                        match ext {
                            "wxml" | "wxss" | "json" => {}
                            "css" | "less" | "scss" if enable_other_ss => {}
                            _ => {
                                return;
                            }
                        }
                        let Ok(content) = tokio::fs::read_to_string(&abs_path).await else {
                            return;
                        };
                        let mut project = project.lock().await;
                        match ext {
                            "wxml" => {
                                let _ = project.update_wxml(&abs_path, content);
                            }
                            "wxss" => {
                                let _ = project.update_wxss(&abs_path, content, false);
                            }
                            "json" => {
                                let _ = project.update_json(&abs_path, content);
                            }
                            "css" | "less" | "scss" if enable_other_ss => {
                                if tokio::fs::try_exists(abs_path.with_extension("wxml"))
                                    .await
                                    .is_ok_and(|x| x)
                                {
                                    let _ = project.update_wxss(&abs_path, content, true);
                                }
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            })
            .await;
            Ok(())
        }
        if let Some(root) = self.root() {
            let root = root.to_path_buf();
            let enable_other_ss = self.enable_other_ss;
            let proj = Arc::new(AsyncMutex::new(self));
            rec(proj, &root, enable_other_ss).await?;
        }
        Ok(())
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
                self.json_config_map
                    .insert(abs_path.to_path_buf(), json_config);
            }
            Err(err) => {
                let pos = Position::new(
                    err.line().saturating_sub(1) as u32,
                    err.column().saturating_sub(1) as u32,
                );
                ret.push(Diagnostic {
                    range: Range {
                        start: pos,
                        end: pos,
                    },
                    message: err.to_string(),
                    severity: Some(DiagnosticSeverity::ERROR),
                    ..Default::default()
                });
                self.json_config_map
                    .insert(abs_path.to_path_buf(), Default::default());
            }
        }
        self.file_contents.insert(
            abs_path.to_path_buf(),
            FileContentMetadata::new(content, FileLang::Json),
        );
        Ok(ret)
    }

    fn cleanup_json(&mut self, abs_path: &Path) -> anyhow::Result<()> {
        self.json_config_map.remove(abs_path);
        self.file_contents.remove(abs_path);
        Ok(())
    }

    pub(crate) fn open_json(
        &mut self,
        abs_path: &Path,
        content: String,
    ) -> anyhow::Result<Vec<Diagnostic>> {
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

    fn update_wxss(
        &mut self,
        abs_path: &Path,
        content: String,
        is_other_ss: bool,
    ) -> anyhow::Result<Vec<Diagnostic>> {
        let (ss, err_list) = StyleSheet::parse_str(abs_path, &content);
        self.file_contents.insert(
            abs_path.to_path_buf(),
            FileContentMetadata::new(
                content,
                if is_other_ss {
                    FileLang::OtherSs
                } else {
                    FileLang::Wxss
                },
            ),
        );
        self.style_sheet_map.insert(abs_path.to_path_buf(), ss);
        let diagnostics = err_list
            .into_iter()
            .filter_map(diagnostic_from_wxss_parse_error)
            .collect();
        Ok(diagnostics)
    }

    fn cleanup_wxss(&mut self, abs_path: &Path) -> anyhow::Result<()> {
        self.json_config_map.remove(abs_path);
        self.file_contents.remove(abs_path);
        Ok(())
    }

    pub(crate) fn open_wxss(
        &mut self,
        abs_path: &Path,
        content: String,
    ) -> anyhow::Result<Vec<Diagnostic>> {
        let diagnostics = self.update_wxss(abs_path, content, false)?;
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

    pub(crate) fn open_other_ss(
        &mut self,
        abs_path: &Path,
        content: String,
    ) -> anyhow::Result<Vec<Diagnostic>> {
        if self.get_wxml_tree(&abs_path.with_extension("wxml")).is_ok() {
            let diagnostics = self.update_wxss(abs_path, content, true)?;
            if let Some(x) = self.file_contents.get_mut(abs_path) {
                x.open();
            }
            return Ok(diagnostics);
        }
        Ok(vec![])
    }

    pub(crate) fn get_style_sheet(
        &self,
        abs_path: &Path,
        allow_other_ss: bool,
    ) -> anyhow::Result<&StyleSheet> {
        let tree = self
            .style_sheet_map
            .get(abs_path)
            .or_else(|| {
                if allow_other_ss {
                    self.style_sheet_map
                        .get(&abs_path.with_extension("css"))
                        .or_else(|| self.style_sheet_map.get(&abs_path.with_extension("less")))
                        .or_else(|| self.style_sheet_map.get(&abs_path.with_extension("scss")))
                } else {
                    None
                }
            })
            .ok_or_else(|| anyhow::Error::msg("no such style sheet"))?;
        Ok(tree)
    }

    fn update_wxml(&mut self, abs_path: &Path, content: String) -> anyhow::Result<Vec<Diagnostic>> {
        let tmpl_path = self.unix_rel_path_or_fallback(&abs_path);
        let err_list = self.template_group.add_tmpl(&tmpl_path, &content);
        self.file_contents.insert(
            abs_path.to_path_buf(),
            FileContentMetadata::new(content, FileLang::Wxml),
        );
        let diagnostics = err_list
            .into_iter()
            .filter_map(diagnostic_from_wxml_parse_error)
            .collect();
        Ok(diagnostics)
    }

    fn cleanup_wxml(&mut self, abs_path: &Path) -> anyhow::Result<()> {
        let tmpl_path = self.unix_rel_path_or_fallback(&abs_path);
        self.template_group.remove_tmpl(&tmpl_path);
        self.cached_wxml_converted_expr.remove(&tmpl_path);
        self.file_contents.remove(abs_path);
        Ok(())
    }

    pub(crate) fn open_wxml(
        &mut self,
        abs_path: &Path,
        content: String,
    ) -> anyhow::Result<Vec<Diagnostic>> {
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
        let tmpl_path = self.unix_rel_path_or_fallback(&abs_path);
        let tree = self.template_group.get_tree(&tmpl_path)?;
        Ok(tree)
    }

    pub(crate) fn list_wxml_trees(&self) -> impl Iterator<Item = (&str, &Template)> {
        self.template_group.list_template_trees()
    }

    pub(crate) fn wxml_converted_expr_release(&mut self, abs_path: &Path) -> bool {
        let tmpl_path = self.unix_rel_path_or_fallback(&abs_path);
        self.cached_wxml_converted_expr.remove(&tmpl_path).is_some()
    }

    pub(crate) fn wxml_converted_expr_code(&mut self, abs_path: &Path, ts_env: &str) -> anyhow::Result<String> {
        let tmpl_path = self.unix_rel_path_or_fallback(&abs_path);
        let expr = self.template_group.get_tmpl_converted_expr(&tmpl_path, ts_env)?;
        let code = expr.code().to_string();
        self.cached_wxml_converted_expr.insert(tmpl_path, expr);
        Ok(code)
    }

    pub(crate) fn wxml_converted_expr_get_source_location(&self, abs_path: &Path, loc: Location) -> Option<Location> {
        let tmpl_path = self.unix_rel_path_or_fallback(&abs_path);
        self.cached_wxml_converted_expr
            .get(&tmpl_path)
            .and_then(|x| x.get_source_location(loc))
    }

    pub(crate) fn wxml_converted_expr_get_token_at_source_position(
        &self,
        abs_path: &Path,
        pos: crate::wxss::Position,
    ) -> Option<(Location, crate::wxss::Position)> {
        let tmpl_path = self.unix_rel_path_or_fallback(&abs_path);
        self.cached_wxml_converted_expr
            .get(&tmpl_path)
            .and_then(|x| x.get_token_at_source_position(pos))
    }

    pub(crate) fn for_each_json_config(&self, mut f: impl FnMut(&Path, &JsonConfig)) {
        for (p, json_config) in self.json_config_map.iter() {
            f(p, json_config);
        }
    }

    pub(crate) fn get_target_component_path(
        &self,
        abs_path: &Path,
        tag_name: &str,
    ) -> Option<PathBuf> {
        let json_path = abs_path.with_extension("json");
        let Some(json_config) = self.get_json_config(&json_path) else {
            return None;
        };
        let Some(rel_path) = json_config.using_components.get(tag_name) else {
            return None;
        };
        self.find_rel_path_for_file(&json_path, rel_path)
    }

    pub(crate) fn search_component_wxml_usages(
        &self,
        abs_path: &Path,
        tag_name: &str,
        mut f: impl FnMut(&Path, &Template, &str),
    ) {
        if let Some(expected_target) = self.get_target_component_path(abs_path, &tag_name) {
            self.for_each_json_config(|p, json_config| {
                for (expected_tag_name, rel_path) in json_config.using_components.iter() {
                    let Some(target) = self.find_rel_path_for_file(p, &rel_path) else {
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

    pub(crate) fn get_wxml_template_names(&self, abs_path: &Path) -> Option<Vec<String>> {
        let Ok(tree) = self.get_wxml_tree(&abs_path) else {
            return None;
        };
        let mut names: Vec<_> = tree
            .globals
            .sub_templates
            .iter()
            .map(|x| x.name.name.to_string())
            .collect();
        for import in tree.globals.imports.iter() {
            if let Some(p) = self.find_rel_path_for_file(abs_path, &import.src.name) {
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

    pub(crate) fn import_and_include_templates(
        &self,
        abs_path: &Path,
        template: &Template,
        mut f: impl FnMut(&Path, &Template),
    ) {
        fn rec_import_and_include_templates(
            visited: &mut HashSet<PathBuf>,
            project: &Project,
            abs_path: &Path,
            template: &Template,
            f: &mut impl FnMut(&Path, &Template),
        ) {
            visited.insert(abs_path.to_path_buf());
            let imp_iter = template.globals.imports.iter().map(|x| x.src.name.as_str());
            let inc_iter = template
                .globals
                .includes
                .iter()
                .map(|x| x.src.name.as_str());
            for rel in imp_iter.chain(inc_iter) {
                if let Some(p) = project.find_rel_path_for_file(abs_path, rel) {
                    if let Some(imported_path) = crate::utils::ensure_file_extension(&p, "wxml") {
                        if let Ok(template) = project.get_wxml_tree(&imported_path) {
                            rec_import_and_include_templates(
                                visited,
                                project,
                                &imported_path,
                                template,
                                f,
                            );
                        }
                    }
                }
            }
            f(abs_path, template);
        }
        rec_import_and_include_templates(&mut HashSet::new(), self, abs_path, template, &mut f);
    }

    pub(crate) fn import_style_sheets(
        &self,
        abs_path: &Path,
        sheet: &StyleSheet,
        mut f: impl FnMut(&Path, &StyleSheet),
    ) {
        fn rec_import_style_sheets(
            visited: &mut HashSet<PathBuf>,
            project: &Project,
            abs_path: &Path,
            sheet: &StyleSheet,
            f: &mut impl FnMut(&Path, &StyleSheet),
        ) {
            visited.insert(abs_path.to_path_buf());
            crate::wxss_utils::for_each_import_in_style_sheet(sheet, |rel| {
                if let Some(p) = project.find_rel_path_for_file(abs_path, rel) {
                    if let Some(imported_path) = crate::utils::ensure_file_extension(&p, "wxss") {
                        if let Ok(sheet) = project.get_style_sheet(&imported_path, false) {
                            rec_import_style_sheets(visited, project, &imported_path, sheet, f);
                        }
                    }
                }
            });
            f(abs_path, sheet);
        }
        rec_import_style_sheets(&mut HashSet::new(), self, abs_path, sheet, &mut f);
    }
}

fn diagnostic_from_wxml_parse_error(x: ParseError) -> Option<Diagnostic> {
    if x.kind == ParseErrorKind::UnknownMetaTag {
        return None;
    }
    Some(Diagnostic {
        range: Range {
            start: Position {
                line: x.location.start.line,
                character: x.location.start.utf16_col,
            },
            end: Position {
                line: x.location.end.line,
                character: x.location.end.utf16_col,
            },
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
            start: Position {
                line: x.location.start.line,
                character: x.location.start.utf16_col,
            },
            end: Position {
                line: x.location.end.line,
                character: x.location.end.utf16_col,
            },
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

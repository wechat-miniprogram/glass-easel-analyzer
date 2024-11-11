use std::{collections::HashMap, ffi::OsStr, path::{Path, PathBuf}};

use glass_easel_template_compiler::{parse::{ParseError, ParseErrorKind, ParseErrorLevel, Template}, TmplGroup};
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

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
    component: bool,
    #[serde(default)]
    #[allow(dead_code)]
    using_components: HashMap<String, String>,
}

pub(crate) struct Project {
    root: PathBuf,
    file_contents: HashMap<PathBuf, FileContentMetadata>,
    json_config_map: HashMap<PathBuf, JsonConfig>,
    template_group: TmplGroup,
}

impl Project {
    pub(super) fn new(root: PathBuf) -> Self {
        Self {
            root,
            file_contents: HashMap::new(),
            json_config_map: HashMap::new(),
            template_group: TmplGroup::new(),
        }
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

    pub(crate) fn file_changed(&mut self, abs_path: &Path) {
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

    pub(crate) fn file_content(&mut self, abs_path: &Path) -> Option<&FileContentMetadata> {
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

    fn update_wxss(&mut self, abs_path: &Path, content: String) -> anyhow::Result<Vec<Diagnostic>> {
        let mut ret = vec![];
        // TODO
        Ok(ret)
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

    pub(crate) fn load_wxml_direct_deps(&mut self, abs_path: &Path) -> anyhow::Result<()> {
        let paths: Vec<_> = {
            let tmpl_path = self.unix_rel_path(&abs_path)?;
            let tree = self.template_group.get_tree(&tmpl_path)?;
            tree.direct_dependencies().filter_map(|x| {
                self.find_rel_path_for_file(abs_path, &x).ok()
            }).collect()
        };
        for p in paths {
            let Some(p) = crate::utils::ensure_file_extension(&p, "wxml") else {
                continue;
            };
            let _ = self.file_content(&p);
        }
        Ok(())
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

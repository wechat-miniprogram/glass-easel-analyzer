use std::{collections::HashMap, path::{Path, PathBuf}};

use glass_easel_template_compiler::{parse::{ParseError, ParseErrorKind, ParseErrorLevel, Template}, TmplGroup};
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

pub(crate) struct FileContentMetadata {
    pub(crate) content: String,
    pub(crate) line_starts: Vec<usize>,
}

impl FileContentMetadata {
    fn new(content: String) -> Self {
        let mut line_starts = vec![];
        line_starts.push(0);
        let mut iter = content.as_bytes().iter().enumerate();
        while let Some((idx, byte)) = iter.next() {
            let byte = *byte;
            if byte == b'\n' {
                line_starts.push(idx + 1);
            } else if byte == b'\r' {
                if content.as_bytes()[idx + 1] != b'\n' {
                    line_starts.push(idx + 1);
                }
            }
        }
        FileContentMetadata {
            content,
            line_starts,
        }
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
    template_group: TmplGroup,
}

impl Project {
    pub(super) fn new(root: PathBuf) -> Self {
        Self {
            root,
            file_contents: HashMap::new(),
            template_group: TmplGroup::new(),
        }
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn unix_rel_path(&self, abs_path: &Path) -> anyhow::Result<String> {
        crate::utils::unix_rel_path(&self.root, abs_path)
    }

    pub(crate) fn get_file_content(&self, abs_path: &Path) -> Option<&FileContentMetadata> {
        self.file_contents.get(abs_path)
    }

    pub(crate) fn set_json(&mut self, abs_path: impl Into<PathBuf>, content: String) {
        let _json_config: JsonConfig = serde_json::from_str(&content).unwrap_or_default(); // TODO handling failure
        self.file_contents.insert(abs_path.into(), FileContentMetadata::new(content));
    }

    pub(crate) fn set_wxss(&mut self, abs_path: impl Into<PathBuf>, content: String) {
         // TODO handling wxss content
        self.file_contents.insert(abs_path.into(), FileContentMetadata::new(content));
    }

    pub(crate) fn set_wxml(&mut self, abs_path: impl Into<PathBuf>, content: String) -> anyhow::Result<Vec<Diagnostic>> {
        let abs_path = abs_path.into();
        let tmpl_path = self.unix_rel_path(&abs_path)?;
        let err_list = self.template_group.add_tmpl(&tmpl_path, &content);
        self.file_contents.insert(abs_path, FileContentMetadata::new(content));
        let diagnostics = err_list.into_iter().filter_map(diagnostic_from_wxml_parse_error).collect();
        Ok(diagnostics)
    }

    pub(crate) fn remove_wxml(&mut self, abs_path: impl Into<PathBuf>) -> anyhow::Result<()> {
        let abs_path = abs_path.into();
        let tmpl_path = self.unix_rel_path(&abs_path)?;
        self.template_group.remove_tmpl(&tmpl_path);
        self.file_contents.remove(&abs_path);
        Ok(())
    }

    pub(crate) fn get_wxml_tree(&self, abs_path: &Path) -> anyhow::Result<&Template> {
        let tmpl_path = self.unix_rel_path(&abs_path)?;
        let tree = self.template_group.get_tree(&tmpl_path)?;
        Ok(tree)
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

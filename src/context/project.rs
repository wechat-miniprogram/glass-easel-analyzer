use std::{collections::HashMap, path::{Path, PathBuf}};

use glass_easel_template_compiler::{parse::{ParseError, ParseErrorLevel}, TmplGroup};
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JsonConfig {
    #[serde(default)]
    component: bool,
    #[serde(default)]
    using_components: HashMap<String, String>,
}

pub(crate) struct SourceMetadata {

}

pub(crate) struct Project {
    root: PathBuf,
    file_contents: HashMap<PathBuf, String>,
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

    pub(crate) fn get_file_content(&self, abs_path: &Path) -> Option<&str> {
        self.file_contents.get(abs_path).map(|x| x.as_str())
    }

    pub(crate) fn template_group(&mut self) -> &mut TmplGroup {
        &mut self.template_group
    }

    pub(crate) fn set_json(&mut self, abs_path: impl Into<PathBuf>, content: String) {
        let _json_config: JsonConfig = serde_json::from_str(&content).unwrap_or_default(); // TODO handling failure
        self.file_contents.insert(abs_path.into(), content);
    }

    pub(crate) fn set_wxss(&mut self, abs_path: impl Into<PathBuf>, content: String) {
         // TODO handling wxss content
        self.file_contents.insert(abs_path.into(), content);
    }

    pub(crate) fn set_wxml(&mut self, abs_path: impl Into<PathBuf>, content: String) -> anyhow::Result<Vec<Diagnostic>> {
        let abs_path = abs_path.into();
        let tmpl_path = self.unix_rel_path(&abs_path)?;
        let err_list = self.template_group.add_tmpl(&tmpl_path, &content);
        self.file_contents.insert(abs_path, content);
        let diagnostics = err_list.into_iter().map(diagnostic_from_wxml_parse_error).collect();
        Ok(diagnostics)
    }

    pub(crate) fn remove_wxml(&mut self, abs_path: impl Into<PathBuf>) -> anyhow::Result<()> {
        let abs_path = abs_path.into();
        let tmpl_path = self.unix_rel_path(&abs_path)?;
        self.template_group.remove_tmpl(&tmpl_path);
        self.file_contents.remove(&abs_path);
        Ok(())
    }
}

fn diagnostic_from_wxml_parse_error(x: ParseError) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position { line: x.location.start.line, character: x.location.start.utf16_col },
            end: Position { line: x.location.end.line, character: x.location.end.utf16_col },
        },
        severity: Some(match x.level() {
            ParseErrorLevel::Fatal => DiagnosticSeverity::ERROR,
            ParseErrorLevel::Error => DiagnosticSeverity::ERROR,
            ParseErrorLevel::Warn => DiagnosticSeverity::WARNING,
            ParseErrorLevel::Note => DiagnosticSeverity::HINT,
        }),
        code: Some(lsp_types::NumberOrString::Number(x.code() as i32)),
        message: x.kind.to_string(),
        ..Default::default()
    }
}

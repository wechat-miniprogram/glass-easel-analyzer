use std::{borrow::Cow, cmp::Ordering, path::{Path, PathBuf}};

use lsp_types::Url;

use crate::wxss::{Location, Position};

pub(crate) fn log_if_err<T>(r: anyhow::Result<T>) {
    if let Err(err) = r {
        log::error!("{}", err);
    }
}

pub(crate) fn generate_non_fs_fake_path(uri: &Url) -> PathBuf {
    let mut p = PathBuf::from("/");
    p.push(uri.scheme());
    if let Some(domain) = uri.domain() {
        p.push(domain);
    }
    if let Some(segs) = uri.path_segments() {
        for seg in segs {
            p.push(seg);
        }
    }
    p
}

pub(crate) fn unix_rel_path(base: &Path, target: &Path) -> anyhow::Result<String> {
    let rel_path = target.strip_prefix(base)?;
    let rel_path_slices: Vec<_> = rel_path
        .components()
        .map(|x| x.as_os_str().to_str().unwrap_or_default())
        .collect();
    Ok(rel_path_slices.join("/"))
}

pub(crate) fn join_unix_rel_path(base: &Path, rel_or_abs_path: &str, limit: &Path) -> anyhow::Result<PathBuf> {
    let (base, rel_path) = if let Some(rel_path) = rel_or_abs_path.strip_prefix('/') {
        (limit.to_path_buf(), rel_path)
    } else {
        (base.to_path_buf(), rel_or_abs_path)
    };
    let mut base = base.to_path_buf();
    for slice in rel_path.split('/') {
        match slice {
            "." => {}
            ".." => {
                base.pop();
            }
            slice => {
                base.push(slice);
            }
        }
        if !base.starts_with(limit) {
            return Err(anyhow::Error::msg("invalid relative path"));
        }
    }
    Ok(base)
}

pub(crate) fn add_file_extension(p: &Path, ext: &str) -> Option<PathBuf> {
    let mut p = p.to_path_buf();
    let Some(name) = p.file_name().and_then(|x| x.to_str()) else {
        return None;
    };
    p.set_file_name(&format!("{}.{}", name, ext));
    Some(p)
}

pub(crate) fn ensure_file_extension<'a>(p: &'a Path, ext: &str) -> Option<Cow<'a, Path>> {
    if p.extension().and_then(|x| x.to_str()) != Some(ext) {
        add_file_extension(p, ext).map(|x| Cow::Owned(x))
    } else {
        Some(Cow::Borrowed(p))
    }
}

pub(crate) fn location_to_lsp_range(loc: &Location) -> lsp_types::Range {
    lsp_types::Range {
        start: lsp_types::Position { line: loc.start.line, character: loc.start.utf16_col },
        end: lsp_types::Position { line: loc.end.line, character: loc.end.utf16_col },
    }
}

pub(crate) fn _lsp_range_to_location(loc: &lsp_types::Range) -> Location {
    let start = Position { line: loc.start.line, utf16_col: loc.start.character };
    let end = Position { line: loc.end.line, utf16_col: loc.end.character };
    start..end
}

pub(crate) fn exclusive_contains(loc: &Location, pos: Position) -> bool {
    loc.start < pos && pos < loc.end
}

pub(crate) fn inclusive_contains(loc: &Location, pos: Position) -> bool {
    (loc.start..=loc.end).contains(&pos)
}

pub(crate) fn exclusive_ordering(loc: &Location, pos: Position) -> Ordering {
    if pos <= loc.start {
        Ordering::Greater
    } else if pos >= loc.end {
        Ordering::Less
    } else {
        Ordering::Equal
    }
}

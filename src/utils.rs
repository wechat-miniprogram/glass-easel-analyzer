use std::{borrow::Cow, path::{Path, PathBuf}};

pub(crate) fn log_if_err<T>(r: anyhow::Result<T>) {
    if let Err(err) = r {
        log::error!("{}", err);
    }
}

pub(crate) fn unix_rel_path(base: &Path, target: &Path) -> anyhow::Result<String> {
    let rel_path = target.strip_prefix(base)?;
    let rel_path_slices: Vec<_> = rel_path
        .components()
        .map(|x| x.as_os_str().to_str().unwrap_or_default())
        .collect();
    Ok(rel_path_slices.join("/"))
}

pub(crate) fn join_unix_rel_path(base: &Path, rel_path: &str, limit: &Path) -> anyhow::Result<PathBuf> {
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

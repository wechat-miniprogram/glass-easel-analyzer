use std::path::{Path, PathBuf};

use lsp_types::Uri;

pub(crate) fn log_if_err<T>(r: anyhow::Result<T>) {
    if let Err(err) = r {
        log::error!("{}", err);
    }
}

pub(crate) fn url_to_path(uri: &Uri) -> Option<PathBuf> {
    if uri.scheme()?.as_str() != "file" {
        return None;
    }

    // check host
    let host = match uri.authority() {
        None => {
            return None;
        }
        Some(x) if cfg!(target_os = "windows") => {
            Some(x.as_str())
        }
        Some(x) => {
            if x.as_str() == "" {
                None
            } else {
                return None;
            }
        }
    };

    // base path
    let mut segs = uri.path().segments();
    let mut ret = if let Some(host) = host {
        PathBuf::from(format!(r#"\\{}\"#, host))
    } else if cfg!(target_os = "windows") {
        let path = uri.path().as_str();
        if let Some(path) = path.strip_prefix("/") {
            let bytes = path.as_bytes();
            if bytes[1] == b':' && bytes[2] == b'/' && (b'a'..b'z').contains(&bytes[0].to_ascii_lowercase()) {
                PathBuf::from(segs.next().unwrap().as_str())
            } else {
                PathBuf::from(r#"\"#)
            }
        } else {
            PathBuf::from(r#"\"#)
        }
    } else {
        PathBuf::from("/")
    };

    // push paths
    for seg in segs {
        let s: std::borrow::Cow<str> = seg.decode().into_string().ok()?;
        let s: &str = &s;
        ret.push(s);
    }

    Some(ret)
}

pub(crate) fn unix_rel_path(base: &Path, target: &Path) -> anyhow::Result<String> {
    let rel_path = target.strip_prefix(base)?;
    let rel_path_slices: Vec<_> = rel_path
        .components()
        .map(|x| x.as_os_str().to_str().unwrap_or_default())
        .collect();
    Ok(rel_path_slices.join("/"))
}

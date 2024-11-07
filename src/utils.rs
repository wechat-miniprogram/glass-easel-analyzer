use std::path::Path;

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

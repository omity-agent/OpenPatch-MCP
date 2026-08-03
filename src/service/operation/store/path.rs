use std::path::{Path, PathBuf};
#[cfg(unix)]
pub(super) fn encode(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt as _;
    path.as_os_str().as_bytes().to_vec()
}
#[cfg(unix)]
pub(super) fn decode(bytes: &[u8]) -> anyhow::Result<PathBuf> {
    use std::os::unix::ffi::OsStringExt as _;
    Ok(std::ffi::OsString::from_vec(bytes.to_vec()).into())
}
#[cfg(windows)]
pub(super) fn encode(path: &Path) -> Vec<u8> {
    use std::os::windows::ffi::OsStrExt as _;
    path.as_os_str()
        .encode_wide()
        .flat_map(u16::to_be_bytes)
        .collect()
}
#[cfg(windows)]
pub(super) fn decode(bytes: &[u8]) -> anyhow::Result<PathBuf> {
    use std::os::windows::ffi::OsStringExt as _;
    let (pairs, remainder) = bytes.as_chunks::<2>();
    anyhow::ensure!(remainder.is_empty(), "invalid path in operation history");
    let wide = pairs
        .iter()
        .map(|pair| u16::from_be_bytes(*pair))
        .collect::<Vec<_>>();
    Ok(std::ffi::OsString::from_wide(&wide).into())
}

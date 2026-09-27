//! Crystalline Lineage — @prompt lab/doxygen-explorador/CONTRATO.md
use std::{
    fs,
    io::Write,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
/// Publicação exclusiva de bytes completos; não garante fsync do diretório.
pub fn publicar(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let (temp, mut file) = loop {
        let temp = parent.join(format!(
            ".doxygen-{}-{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut opts = fs::OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        match opts.open(&temp) {
            Ok(f) => break (temp, f),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("output_io: {e}")),
        }
    };
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::hard_link(&temp, path)
    })();
    drop(file);
    let _ = fs::remove_file(temp);
    result.map_err(|e| format!("output_io (não sobrescreve): {e}"))
}

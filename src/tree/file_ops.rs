use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum ClipboardOp {
    Copy,
    Cut,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TreeClipboard {
    pub path: PathBuf,
    pub operation: ClipboardOp,
}

pub fn copy_file(src: &Path, dest_dir: &Path) -> Result<PathBuf> {
    let file_name = src
        .file_name()
        .context("Source has no filename")?;
    let dest = dest_dir.join(file_name);

    if src.is_dir() {
        copy_dir_recursive(src, &dest)?;
    } else {
        std::fs::copy(src, &dest).context("Failed to copy file")?;
    }

    Ok(dest)
}

pub fn move_file(src: &Path, dest_dir: &Path) -> Result<PathBuf> {
    let file_name = src
        .file_name()
        .context("Source has no filename")?;
    let dest = dest_dir.join(file_name);

    std::fs::rename(src, &dest).context("Failed to move file")?;
    Ok(dest)
}

pub fn delete_path(path: &Path) -> Result<()> {
    if path.is_dir() {
        std::fs::remove_dir_all(path).context("Failed to delete directory")?;
    } else {
        std::fs::remove_file(path).context("Failed to delete file")?;
    }
    Ok(())
}

pub fn rename_path(old: &Path, new_name: &str) -> Result<PathBuf> {
    let new_path = old
        .parent()
        .context("Path has no parent")?
        .join(new_name);
    std::fs::rename(old, &new_path).context("Failed to rename")?;
    Ok(new_path)
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}


#![allow(dead_code)]

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};


#[derive(Debug, Clone)]
pub struct SymlinkInfo {

    pub link_path: PathBuf,

    pub target_path: PathBuf,

    pub is_absolute: bool,
}


pub fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|m| m.is_symlink())
        .unwrap_or(false)
}


pub fn read_link(path: &Path) -> Result<PathBuf> {
    fs::read_link(path)
        .with_context(|| format!("Failed to read symlink: {}", path.display()))
}


pub fn get_symlink_info(link_path: &Path) -> Result<SymlinkInfo> {
    let target_path = read_link(link_path)?;
    let is_absolute = target_path.is_absolute();

    Ok(SymlinkInfo {
        link_path: link_path.to_path_buf(),
        target_path,
        is_absolute,
    })
}





#[cfg(windows)]
pub fn create_symlink(link_path: &Path, target_path: &Path) -> Result<()> {
    use std::os::windows::fs::{symlink_dir, symlink_file};


    let is_dir = if target_path.exists() {
        target_path.is_dir()
    } else {


        target_path.to_string_lossy().ends_with('\\') ||
        target_path.to_string_lossy().ends_with('/')
    };

    if is_dir {
        symlink_dir(target_path, link_path)
            .with_context(|| format!("Failed to create directory symlink: {} -> {}",
                link_path.display(), target_path.display()))?;
    } else {
        symlink_file(target_path, link_path)
            .with_context(|| format!("Failed to create file symlink: {} -> {}",
                link_path.display(), target_path.display()))?;
    }

    Ok(())
}


#[cfg(unix)]
pub fn create_symlink(link_path: &Path, target_path: &Path) -> Result<()> {
    use std::os::unix::fs::symlink;

    symlink(target_path, link_path)
        .with_context(|| format!("Failed to create symlink: {} -> {}",
            link_path.display(), target_path.display()))?;

    Ok(())
}




pub fn detect_symlink_loop(start_path: &Path, max_depth: usize) -> Result<bool> {
    let mut visited = HashSet::new();
    let mut current = start_path.to_path_buf();
    let mut depth = 0;

    while is_symlink(&current) && depth < max_depth {

        if !visited.insert(current.clone()) {
            return Ok(true);
        }


        current = read_link(&current)?;
        depth += 1;
    }


    Ok(depth >= max_depth)
}




pub fn resolve_symlink(path: &Path, max_depth: usize) -> Result<PathBuf> {
    if detect_symlink_loop(path, max_depth)? {
        anyhow::bail!("Symlink loop detected at: {}", path.display());
    }

    let mut current = path.to_path_buf();
    let mut depth = 0;

    while is_symlink(&current) && depth < max_depth {
        let target = read_link(&current)?;


        current = if target.is_absolute() {
            target
        } else {
            current.parent()
                .ok_or_else(|| anyhow::anyhow!("No parent directory"))?
                .join(target)
        };

        depth += 1;
    }

    Ok(current)
}




pub fn copy_symlink(src: &Path, dst: &Path) -> Result<()> {
    let target = read_link(src)?;
    create_symlink(dst, &target)
}




pub fn copy_symlink_content(src: &Path, dst: &Path) -> Result<()> {
    let resolved = resolve_symlink(src, 40)?;

    if resolved.is_dir() {

        copy_dir_recursive(&resolved, dst)?;
    } else {

        fs::copy(&resolved, dst)
            .with_context(|| format!("Failed to copy file: {} -> {}",
                resolved.display(), dst.display()))?;
    }

    Ok(())
}


fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)
        .with_context(|| format!("Failed to create directory: {}", dst.display()))?;

    for entry in fs::read_dir(src)
        .with_context(|| format!("Failed to read directory: {}", src.display()))?
    {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .with_context(|| format!("Failed to copy file: {} -> {}",
                    src_path.display(), dst_path.display()))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_is_symlink() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "test").unwrap();

        assert!(!is_symlink(&file_path));
    }

    #[test]
    fn test_symlink_info() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target.txt");
        let link = temp.path().join("link.txt");

        fs::write(&target, "content").unwrap();

        #[cfg(windows)]
        {


            if create_symlink(&link, &target).is_err() {
                return;
            }
        }

        #[cfg(unix)]
        {
            create_symlink(&link, &target).unwrap();
        }

        if link.exists() {
            let info = get_symlink_info(&link).unwrap();
            assert_eq!(info.link_path, link);
        }
    }

    #[test]
    fn test_resolve_symlink() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target.txt");
        fs::write(&target, "content").unwrap();


        #[cfg(windows)]
        {

            return;
        }

        #[cfg(unix)]
        {
            let link = temp.path().join("link.txt");
            create_symlink(&link, &target).unwrap();

            let resolved = resolve_symlink(&link, 40).unwrap();
            assert_eq!(resolved, target);
        }
    }
}

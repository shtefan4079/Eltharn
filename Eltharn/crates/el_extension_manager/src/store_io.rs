#[cfg(not(target_os = "linux"))]
use std::fs;
use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
};
pub fn open(path: &Path, create_lock: bool) -> io::Result<File> {
    #[cfg(target_os = "linux")]
    {
        use rustix::fs::{CWD, Mode, OFlags, ResolveFlags, openat2};
        let flags = if create_lock {
            OFlags::RDWR | OFlags::CREATE
        } else {
            OFlags::RDONLY
        };
        let mode = if create_lock {
            Mode::RUSR | Mode::WUSR
        } else {
            Mode::empty()
        };
        let fd = openat2(
            CWD,
            path,
            flags | OFlags::CLOEXEC | OFlags::NONBLOCK,
            mode,
            ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )?;
        Ok(File::from(fd))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let mut options = fs::OpenOptions::new();
        options
            .read(true)
            .write(create_lock)
            .create(create_lock)
            .truncate(false);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.custom_flags(0x0020_0000);
        }
        options.open(resolve_under_parent(path)?)
    }
}
#[cfg(not(target_os = "linux"))]
fn resolve_under_parent(path: &Path) -> io::Result<PathBuf> {
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::other("Missing file name"))?;
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let parent = fs::canonicalize(parent)?;
    let target = parent.join(name);
    if let Ok(metadata) = fs::symlink_metadata(&target) {
        if metadata.file_type().is_symlink() {
            return Err(io::Error::other("Symlink denied"));
        }
        if !fs::canonicalize(&target)?.starts_with(&parent) {
            return Err(io::Error::other("Path escapes its store directory"));
        }
    }
    Ok(target)
}
pub struct Parent {
    pub path: PathBuf,
    _file: Option<File>,
}
pub fn parent(path: &Path) -> io::Result<Parent> {
    #[cfg(target_os = "linux")]
    {
        use rustix::fs::{CWD, Mode, OFlags, ResolveFlags, openat2};
        use std::os::fd::AsRawFd;
        let fd = openat2(
            CWD,
            path,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )?;
        let file = File::from(fd);
        let path = PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd()));
        Ok(Parent {
            path,
            _file: Some(file),
        })
    }
    #[cfg(not(target_os = "linux"))]
    {
        let m = fs::symlink_metadata(path)?;
        if !m.file_type().is_dir() {
            return Err(io::Error::other("Not a real directory"));
        }
        let resolved = fs::canonicalize(path)?;
        Ok(Parent {
            path: resolved,
            _file: None,
        })
    }
}

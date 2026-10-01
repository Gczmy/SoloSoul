//! 文件身份来自已经打开的真实句柄；失败时不能退回路径或时间戳授权。
use std::fs::{File, Metadata, OpenOptions};
use std::io;
use std::path::Path;
use std::time::SystemTime;

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct FileIdentity {
    volume_serial: u64,
    file_id: [u8; 16],
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetFileInformationByHandleEx(
        handle: *mut std::ffi::c_void,
        information_class: i32,
        information: *mut std::ffi::c_void,
        buffer_size: u32,
    ) -> i32;
}

#[cfg(windows)]
pub(super) fn file_identity(file: &File) -> io::Result<FileIdentity> {
    use std::os::windows::io::AsRawHandle;
    let mut value = FileIdentity {
        volume_serial: 0,
        file_id: [0; 16],
    };
    // FILE_ID_INFO 与 Win32 ABI 一致，FileIdInfo=18。使用128位 ID，兼容 ReFS；
    // 不依赖 stable Rust 尚未开放的 windows_by_handle 方法，也不启新 Cargo feature。
    // SAFETY: 句柄由存活的 File 持有，指针指向完整 repr(C) 输出结构。
    let success = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            18,
            (&mut value as *mut FileIdentity).cast(),
            std::mem::size_of::<FileIdentity>() as u32,
        )
    };
    if success == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(value)
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct FileIdentity {
    device: u64,
    inode: u64,
}

#[cfg(unix)]
pub(super) fn file_identity(file: &File) -> io::Result<FileIdentity> {
    use std::os::unix::fs::MetadataExt;
    let meta = file.metadata()?;
    Ok(FileIdentity {
        device: meta.dev(),
        inode: meta.ino(),
    })
}

#[cfg(not(any(unix, windows)))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct FileIdentity;
#[cfg(not(any(unix, windows)))]
pub(super) fn file_identity(_: &File) -> io::Result<FileIdentity> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "attachment_cleanup_identity_unavailable",
    ))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FileStamp {
    identity: FileIdentity,
    length: u64,
    modified: SystemTime,
    links: u64,
}
impl FileStamp {
    pub(super) fn read(file: &File) -> io::Result<Self> {
        let meta = file.metadata()?;
        Ok(Self {
            identity: file_identity(file)?,
            length: meta.len(),
            modified: meta.modified()?,
            links: link_count(file)?,
        })
    }
    pub(super) fn length(&self) -> u64 {
        self.length
    }
    pub(super) fn single_link(&self) -> bool {
        self.links == 1
    }
}

pub(super) fn link_or_reparse(meta: &Metadata) -> bool {
    if meta.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

pub(super) fn open_directory(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // BACKUP_SEMANTICS 打开目录；OPEN_REPARSE_POINT 避免打开时追随链接。
        options.custom_flags(0x0200_0000 | 0x0020_0000);
    }
    options.open(path)
}

pub(super) fn open_regular_file(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x0020_0000); // FILE_FLAG_OPEN_REPARSE_POINT
    }
    options.open(path)
}

#[cfg(unix)]
fn link_count(file: &File) -> io::Result<u64> {
    use std::os::unix::fs::MetadataExt;
    Ok(file.metadata()?.nlink())
}
#[cfg(windows)]
fn link_count(file: &File) -> io::Result<u64> {
    use std::os::windows::io::AsRawHandle;
    #[repr(C)]
    struct StandardInfo {
        allocation_size: i64,
        end_of_file: i64,
        number_of_links: u32,
        delete_pending: u8,
        directory: u8,
    }
    let mut info = StandardInfo {
        allocation_size: 0,
        end_of_file: 0,
        number_of_links: 0,
        delete_pending: 0,
        directory: 0,
    };
    // FileStandardInfo=1，两个 BOOLEAN 是 u8；布局与已安装 windows 0.58 ABI 一致。
    // SAFETY: 存活 File 的句柄与完整、可写的 repr(C) 输出缓冲区。
    let success = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            1,
            (&mut info as *mut StandardInfo).cast(),
            std::mem::size_of::<StandardInfo>() as u32,
        )
    };
    if success == 0 {
        return Err(io::Error::last_os_error());
    }
    if info.delete_pending != 0 || info.directory != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "attachment_cleanup_file_changed",
        ));
    }
    Ok(u64::from(info.number_of_links))
}
#[cfg(not(any(unix, windows)))]
fn link_count(_: &File) -> io::Result<u64> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "attachment_cleanup_identity_unavailable",
    ))
}

use crate::prelude::*;

const FILE_NAME: &str = "age.txt";
const FILE_CONTENTS: &[u8] = b"Hello, world!\n";
const FILE_INO: fuser::INodeNo = fuser::INodeNo(2);
const TTL: Duration = Duration::from_secs(1);

#[derive(Error, Debug)]
pub enum MountError {
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Join(#[from] task::JoinError),
}

#[derive(Args, Debug)]
pub struct MountArgs {
    /// Directory where the virtual file system should be mounted
    pub path: PathBuf,
}

pub struct MountCmd {}

impl MountCmd {
    pub async fn run<'a>(_cli: &'a Cli, args: &'a MountArgs) -> Result<(), MountError> {
        tokio::fs::create_dir_all(&args.path).await?;

        UiMessage::info(&format!(
            "Mounting virtual file system in {:?}. Unmount with `shhhfs unmount {}`.",
            args.path,
            args.path.display()
        ));

        let fs = ShhhFs::new();

        let mut config = fuser::Config::default();
        config.mount_options = vec![
            fuser::MountOption::RO,
            fuser::MountOption::FSName("shhhfs".to_string()),
        ];

        let session = fuser::spawn_mount2(fs, &args.path, &config)
            .map_err(|err| mount_error_context(err, &args.path))?;
        let mount_path = args.path.clone();

        let mut session_task = task::spawn_blocking(move || session.join());

        select! {
            result = &mut session_task => {
                result??;
            }

            signal = shutdown_signal() => {
                signal?;
                UiMessage::info(&format!("Unmounting virtual file system from {:?}", mount_path));
                UnmountCmd::unmount_path(&mount_path)?;
                session_task.await??;
            }
        }

        Ok(())
    }
}

fn mount_error_context(err: std::io::Error, path: &Path) -> anyhow::Error {
    let error = anyhow!(err).context(format!("failed to mount virtual file system at {:?}", path));

    #[cfg(target_os = "macos")]
    let error = error.context(
        "macOS requires macFUSE. Install it with `brew install macfuse pkgconf`, then approve the system extension if macOS prompts for it.",
    );

    error
}

#[cfg(unix)]
async fn shutdown_signal() -> std::io::Result<()> {
    let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())?;

    select! {
        result = signal::ctrl_c() => result,

        _ = sigterm.recv() => Ok(()),
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() -> std::io::Result<()> {
    signal::ctrl_c().await
}

struct ShhhFs {
    uid: u32,
    gid: u32,
    created_at: SystemTime,
}

impl ShhhFs {
    fn new() -> Self {
        Self {
            uid: unsafe { libc::geteuid() },
            gid: unsafe { libc::getegid() },
            created_at: SystemTime::now(),
        }
    }

    fn root_attr(&self) -> fuser::FileAttr {
        fuser::FileAttr {
            ino: fuser::INodeNo::ROOT,
            size: 0,
            blocks: 0,
            atime: self.created_at,
            mtime: self.created_at,
            ctime: self.created_at,
            crtime: self.created_at,
            kind: fuser::FileType::Directory,
            perm: 0o755,
            nlink: 2,
            uid: self.uid,
            gid: self.gid,
            rdev: 0,
            blksize: 4096,
            flags: 0,
        }
    }

    fn file_attr(&self) -> fuser::FileAttr {
        fuser::FileAttr {
            ino: FILE_INO,
            size: FILE_CONTENTS.len() as u64,
            blocks: 1,
            atime: self.created_at,
            mtime: self.created_at,
            ctime: self.created_at,
            crtime: self.created_at,
            kind: fuser::FileType::RegularFile,
            perm: 0o400,
            nlink: 1,
            uid: self.uid,
            gid: self.gid,
            rdev: 0,
            blksize: 4096,
            flags: 0,
        }
    }
}

impl fuser::Filesystem for ShhhFs {
    fn lookup(
        &self,
        _req: &fuser::Request,
        parent: fuser::INodeNo,
        name: &ffi::OsStr,
        reply: fuser::ReplyEntry,
    ) {
        if parent == fuser::INodeNo::ROOT && name == FILE_NAME {
            reply.entry(&TTL, &self.file_attr(), fuser::Generation(0));
        } else {
            reply.error(fuser::Errno::ENOENT);
        }
    }

    fn getattr(
        &self,
        _req: &fuser::Request,
        ino: fuser::INodeNo,
        _fh: Option<fuser::FileHandle>,
        reply: fuser::ReplyAttr,
    ) {
        match ino {
            fuser::INodeNo::ROOT => reply.attr(&TTL, &self.root_attr()),

            FILE_INO => reply.attr(&TTL, &self.file_attr()),

            _ => reply.error(fuser::Errno::ENOENT),
        }
    }

    fn open(
        &self,
        _req: &fuser::Request,
        ino: fuser::INodeNo,
        _flags: fuser::OpenFlags,
        reply: fuser::ReplyOpen,
    ) {
        if ino == FILE_INO {
            reply.opened(fuser::FileHandle(0), fuser::FopenFlags::empty());
        } else {
            reply.error(fuser::Errno::ENOENT);
        }
    }

    fn read(
        &self,
        _req: &fuser::Request,
        ino: fuser::INodeNo,
        _fh: fuser::FileHandle,
        offset: u64,
        size: u32,
        _flags: fuser::OpenFlags,
        _lock_owner: Option<fuser::LockOwner>,
        reply: fuser::ReplyData,
    ) {
        if ino != FILE_INO {
            reply.error(fuser::Errno::ENOENT);
            return;
        }

        let offset = offset as usize;
        let size = size as usize;
        let data = FILE_CONTENTS.get(offset..).unwrap_or_default();
        let end = data.len().min(size);
        reply.data(&data[..end]);
    }

    fn readdir(
        &self,
        _req: &fuser::Request,
        ino: fuser::INodeNo,
        _fh: fuser::FileHandle,
        offset: u64,
        mut reply: fuser::ReplyDirectory,
    ) {
        if ino != fuser::INodeNo::ROOT {
            reply.error(fuser::Errno::ENOENT);
            return;
        }

        let entries = [
            (fuser::INodeNo::ROOT, fuser::FileType::Directory, "."),
            (fuser::INodeNo::ROOT, fuser::FileType::Directory, ".."),
            (FILE_INO, fuser::FileType::RegularFile, FILE_NAME),
        ];

        for (index, entry) in entries.iter().enumerate().skip(offset as usize) {
            let next_offset = (index + 1) as u64;
            if reply.add(entry.0, next_offset, entry.1, entry.2) {
                break;
            }
        }

        reply.ok();
    }
}

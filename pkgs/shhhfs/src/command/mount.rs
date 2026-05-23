use crate::prelude::*;

const FIRST_FILE_INO: u64 = 2;
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

    /// Secrets provider to use
    #[arg(long, default_value = "json")]
    pub provider: String,

    /// Provider-specific options string
    #[arg(long, value_name = "PROVIDER_OPTIONS")]
    pub provider_opts: Option<String>,
}

pub struct MountCmd {}

impl MountCmd {
    pub async fn run<'a>(_cli: &'a Cli, args: &'a MountArgs) -> Result<(), MountError> {
        let provider = provider_from_args(args)?;

        tokio::fs::create_dir_all(&args.path).await?;

        UiMessage::info(&format!(
            "Mounting virtual file system in {:?}. Unmount with `shhhfs unmount {}`.",
            args.path,
            args.path.display()
        ));

        let fs = ShhhFs::new(provider);

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

fn provider_from_args(args: &MountArgs) -> Result<Box<dyn ShhhFsProvider + Send + Sync>> {
    match args.provider.as_str() {
        "json" => {
            let options = args
                .provider_opts
                .as_deref()
                .ok_or_else(|| anyhow!("json provider requires --provider-opts"))?;
            Ok(Box::new(JsonShhhProvider::from_options(options)?))
        }

        provider => Err(anyhow!("unknown provider {:?}", provider)),
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
    provider: Box<dyn ShhhFsProvider + Send + Sync>,
    uid: u32,
    gid: u32,
    created_at: SystemTime,
}

impl ShhhFs {
    fn new(provider: Box<dyn ShhhFsProvider + Send + Sync>) -> Self {
        Self {
            provider,
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

    fn file_ino(index: usize) -> fuser::INodeNo {
        fuser::INodeNo(FIRST_FILE_INO + index as u64)
    }

    fn entry_by_ino(&self, ino: fuser::INodeNo) -> Option<(usize, &ShhhFsEntry)> {
        self.provider
            .entries()
            .iter()
            .enumerate()
            .find(|(index, _entry)| Self::file_ino(*index) == ino)
    }

    fn file_attr(&self, ino: fuser::INodeNo, entry: &ShhhFsEntry) -> fuser::FileAttr {
        fuser::FileAttr {
            ino,
            size: entry.contents.len() as u64,
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
        if parent != fuser::INodeNo::ROOT {
            reply.error(fuser::Errno::ENOENT);
            return;
        }

        for (index, entry) in self.provider.entries().iter().enumerate() {
            if name == entry.name.as_str() {
                let ino = Self::file_ino(index);
                reply.entry(&TTL, &self.file_attr(ino, entry), fuser::Generation(0));
                return;
            }
        }

        reply.error(fuser::Errno::ENOENT);
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

            ino => {
                if let Some((_index, entry)) = self.entry_by_ino(ino) {
                    reply.attr(&TTL, &self.file_attr(ino, entry));
                } else {
                    reply.error(fuser::Errno::ENOENT);
                }
            }
        }
    }

    fn open(
        &self,
        _req: &fuser::Request,
        ino: fuser::INodeNo,
        _flags: fuser::OpenFlags,
        reply: fuser::ReplyOpen,
    ) {
        if self.entry_by_ino(ino).is_some() {
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
        let Some((_index, entry)) = self.entry_by_ino(ino) else {
            reply.error(fuser::Errno::ENOENT);
            return;
        };

        let offset = offset as usize;
        let size = size as usize;
        let data = entry.contents.get(offset..).unwrap_or_default();
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

        let dot_entries = [
            (
                fuser::INodeNo::ROOT,
                fuser::FileType::Directory,
                ffi::OsStr::new("."),
            ),
            (
                fuser::INodeNo::ROOT,
                fuser::FileType::Directory,
                ffi::OsStr::new(".."),
            ),
        ];

        let mut full = false;

        for (index, entry) in dot_entries.iter().enumerate().skip(offset as usize) {
            let next_offset = (index + 1) as u64;
            if reply.add(entry.0, next_offset, entry.1, entry.2) {
                full = true;
                break;
            }
        }

        if !full {
            let file_offset = offset.saturating_sub(dot_entries.len() as u64) as usize;
            for (index, entry) in self.provider.entries().iter().enumerate().skip(file_offset) {
                let next_offset = (dot_entries.len() + index + 1) as u64;
                if reply.add(
                    Self::file_ino(index),
                    next_offset,
                    fuser::FileType::RegularFile,
                    entry.name.as_str(),
                ) {
                    break;
                }
            }
        }

        reply.ok();
    }
}

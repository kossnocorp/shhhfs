use crate::prelude::*;

#[derive(Args, Debug)]
pub struct UnmountArgs {
    /// Directory where the virtual file system is mounted
    pub path: PathBuf,
}

pub struct UnmountCmd {}

impl UnmountCmd {
    pub async fn run(args: &UnmountArgs) -> Result<()> {
        Self::unmount_path(&args.path)?;

        UiMessage::success(&format!(
            "Unmounted virtual file system from {:?}",
            args.path
        ));

        Ok(())
    }

    pub fn unmount_path(path: &Path) -> Result<()> {
        let commands = unmount_commands(path);

        let mut last_error = None;

        for (program, args) in commands {
            match process::Command::new(program).args(&args).output() {
                Ok(output) if output.status.success() => return Ok(()),

                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                    last_error = Some(if stderr.is_empty() {
                        format!("{program} exited with {}", output.status)
                    } else {
                        format!("{program}: {stderr}")
                    });
                }

                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}

                Err(err) => last_error = Some(format!("{program}: {err}")),
            }
        }

        Err(anyhow!(
            "failed to unmount {:?}: {}",
            path,
            last_error.unwrap_or_else(|| "no unmount command was available".to_string())
        ))
    }
}

#[cfg(target_os = "linux")]
fn unmount_commands(path: &Path) -> Vec<(&'static str, Vec<String>)> {
    let path = path.display().to_string();

    vec![
        ("fusermount3", vec!["-u".to_string(), path.clone()]),
        ("fusermount", vec!["-u".to_string(), path.clone()]),
        (
            "fusermount3",
            vec!["-u".to_string(), "-z".to_string(), path.clone()],
        ),
        (
            "fusermount",
            vec!["-u".to_string(), "-z".to_string(), path.clone()],
        ),
        ("umount", vec![path.clone()]),
        ("umount", vec!["-l".to_string(), path]),
    ]
}

#[cfg(target_os = "macos")]
fn unmount_commands(path: &Path) -> Vec<(&'static str, Vec<String>)> {
    let path = path.display().to_string();

    vec![
        ("umount", vec![path.clone()]),
        ("diskutil", vec!["unmount".to_string(), path]),
    ]
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn unmount_commands(path: &Path) -> Vec<(&'static str, Vec<String>)> {
    vec![("umount", vec![path.display().to_string()])]
}

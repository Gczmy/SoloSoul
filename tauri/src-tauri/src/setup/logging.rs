//! RF905：实际 GUI 日志目标及写线程的根所有者寿命。
//! 默认 AppData 日志位于另一个目录时不声称持有该目录的 Vault 锁。

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use solosoul_vault::root_owner::VaultRootOwner;

/// tracing appender 与 panic 直接写入共用固定的 canonical 目标。
/// 全局 panic hook 仍可写入此目标，因此其静态实例也保留重叠根 owner。
#[derive(Clone)]
pub(super) struct LogTarget {
    directory: PathBuf,
    root_owner: Option<Arc<VaultRootOwner>>,
}

impl LogTarget {
    pub(super) fn prepare(
        directory: &Path,
        root_owner: Arc<VaultRootOwner>,
    ) -> Result<Self, String> {
        // 调用方已经严格取得当前 Vault owner；失败不启动无归属的 writer。
        std::fs::create_dir_all(directory).map_err(|e| format!("无法创建日志目录: {e}"))?;
        let directory = directory
            .canonicalize()
            .map_err(|e| format!("无法确认日志目录: {e}"))?;
        if !directory.is_dir() {
            return Err("日志目录不是目录".into());
        }
        // never appender 仍使用 app.log。现有 app.log 若是指入当前 Vault 的
        // 文件链接，同样保活 owner；无法解析的现有目标不能降为 unowned。
        let log_file = directory.join("app.log");
        let file_in_root = match std::fs::symlink_metadata(&log_file) {
            Ok(_) => log_file
                .canonicalize()
                .map_err(|e| format!("无法确认日志文件: {e}"))?
                .starts_with(root_owner.root()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => false,
            Err(e) => return Err(format!("无法检查日志文件: {e}")),
        };
        let in_root = directory.starts_with(root_owner.root()) || file_in_root;
        Ok(Self {
            directory,
            root_owner: in_root.then_some(root_owner),
        })
    }

    pub(super) fn directory(&self) -> &Path {
        &self.directory
    }

    pub(super) fn pin_writer<W>(&self, writer: W) -> OwnerPinnedWriter<W> {
        OwnerPinnedWriter {
            writer,
            _root_owner: self.root_owner.clone(),
        }
    }
}

/// 实际 non-blocking worker 拥有此值；外层 Service/WorkerGuard Drop 不会提前放锁。
/// 字段按声明顺序析构：先关闭 writer，再释放 owner。
pub(super) struct OwnerPinnedWriter<W> {
    writer: W,
    _root_owner: Option<Arc<VaultRootOwner>>,
}

impl<W: Write> Write for OwnerPinnedWriter<W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.writer.write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{mpsc, Weak};
    use std::time::{Duration, Instant};

    const WAIT: Duration = Duration::from_secs(30);

    #[test]
    fn rf905_gui_log_target_only_pins_actual_current_root() {
        let temp = tempfile::tempdir().unwrap();
        let owner = VaultRootOwner::acquire(&temp.path().join("root")).unwrap();
        let weak = Arc::downgrade(&owner);
        let inside = LogTarget::prepare(&owner.root().join("nested/logs"), owner.clone()).unwrap();
        let outside =
            LogTarget::prepare(&temp.path().join("external-logs"), owner.clone()).unwrap();
        assert_eq!(inside.root_owner.as_ref().unwrap().id(), owner.id());
        assert!(outside.root_owner.is_none());
        assert_eq!(
            inside.directory(),
            owner.root().join("nested/logs").canonicalize().unwrap()
        );
        drop(owner);
        assert!(weak.upgrade().is_some());
        drop(inside);
        assert!(
            weak.upgrade().is_none(),
            "unrelated AppData target must not pin the Vault"
        );
        drop(outside);
    }

    #[test]
    fn rf905_gui_panic_target_retains_owner_and_invalid_directory_returns_error() {
        let temp = tempfile::tempdir().unwrap();
        let owner = VaultRootOwner::acquire(&temp.path().join("root")).unwrap();
        let weak = Arc::downgrade(&owner);
        let file = owner.root().join("not-a-directory");
        std::fs::write(&file, b"unchanged").unwrap();
        assert!(LogTarget::prepare(&file, owner.clone()).is_err());
        assert_eq!(std::fs::read(&file).unwrap(), b"unchanged");
        let target = LogTarget::prepare(&owner.root().join("logs"), owner.clone()).unwrap();
        let panic_target = target.clone();
        drop(owner);
        drop(target);
        assert!(
            weak.upgrade().is_some(),
            "panic file path remains a real writer entry point"
        );
        std::fs::write(
            panic_target.directory().join("app.log"),
            b"synthetic panic\n",
        )
        .unwrap();
        drop(panic_target);
        assert!(weak.upgrade().is_none());
    }

    #[cfg(unix)]
    #[test]
    fn rf905_gui_log_target_resolves_directory_and_existing_file_links() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let owner = VaultRootOwner::acquire(&temp.path().join("root")).unwrap();
        let logs = owner.root().join("logs");
        std::fs::create_dir(&logs).unwrap();
        let alias = temp.path().join("log-alias");
        symlink(&logs, &alias).unwrap();
        let target = LogTarget::prepare(&alias, owner.clone()).unwrap();
        assert_eq!(target.directory(), logs.canonicalize().unwrap());
        assert_eq!(target.root_owner.as_ref().unwrap().id(), owner.id());
        let external = temp.path().join("external");
        std::fs::create_dir(&external).unwrap();
        let actual_file = owner.root().join("actual.log");
        std::fs::write(&actual_file, b"existing").unwrap();
        symlink(&actual_file, external.join("app.log")).unwrap();
        let target = LogTarget::prepare(&external, owner.clone()).unwrap();
        assert_eq!(target.root_owner.as_ref().unwrap().id(), owner.id());
        std::fs::remove_file(external.join("app.log")).unwrap();
        symlink(owner.root().join("missing.log"), external.join("app.log")).unwrap();
        assert!(LogTarget::prepare(&external, owner).is_err());
    }

    struct BlockingLogFile {
        file: std::fs::File,
        entered: mpsc::Sender<()>,
        release: Option<mpsc::Receiver<()>>,
        owner: Weak<VaultRootOwner>,
        owner_alive_at_writer_drop: Arc<AtomicBool>,
    }

    impl Write for BlockingLogFile {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            if let Some(release) = self.release.take() {
                self.entered
                    .send(())
                    .map_err(|e| io::Error::other(e.to_string()))?;
                release
                    .recv_timeout(WAIT)
                    .map_err(|e| io::Error::other(e.to_string()))?;
            }
            self.file.write(buffer)
        }
        fn flush(&mut self) -> io::Result<()> {
            self.file.flush()
        }
    }

    impl Drop for BlockingLogFile {
        fn drop(&mut self) {
            self.owner_alive_at_writer_drop
                .store(self.owner.upgrade().is_some(), Ordering::SeqCst);
        }
    }

    struct ReleaseLogWorker {
        release: Option<mpsc::Sender<()>>,
        owner: Weak<VaultRootOwner>,
    }

    impl ReleaseLogWorker {
        fn release(&mut self) {
            if let Some(release) = self.release.take() {
                let _ = release.send(());
            }
        }
        fn wait_for_actual_drop(&self) -> bool {
            let started = Instant::now();
            while self.owner.upgrade().is_some() {
                if started.elapsed() >= WAIT {
                    return false;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            true
        }
    }

    impl Drop for ReleaseLogWorker {
        fn drop(&mut self) {
            // 断言展开也先放行真实 writer，避免临时目录早于实际文件句柄删除。
            self.release();
            let _ = self.wait_for_actual_drop();
        }
    }

    #[test]
    fn rf905_gui_actual_log_writer_pins_owner_after_worker_guard_timeout() {
        let temp = tempfile::tempdir().unwrap();
        let owner = VaultRootOwner::acquire(&temp.path().join("root")).unwrap();
        let target = LogTarget::prepare(&owner.root().join("logs"), owner.clone()).unwrap();
        let log_file = target.directory().join("app.log");
        let weak = Arc::downgrade(&owner);
        let writer_dropped_with_owner = Arc::new(AtomicBool::new(false));
        let (entered, ready) = mpsc::channel();
        let (release, release_rx) = mpsc::channel();
        let mut release = ReleaseLogWorker {
            release: Some(release),
            owner: weak.clone(),
        };
        let file = BlockingLogFile {
            file: std::fs::File::create(&log_file).unwrap(),
            entered,
            release: Some(release_rx),
            owner: weak.clone(),
            owner_alive_at_writer_drop: writer_dropped_with_owner.clone(),
        };
        let (mut sink, guard) = tracing_appender::non_blocking(target.pin_writer(file));
        drop(target);
        drop(owner);
        sink.write_all(b"RF905 synthetic GUI log\n").unwrap();
        ready.recv_timeout(WAIT).unwrap();
        // 真实 locked appender 0.2.5 的 guard Drop 只等有界 ack，不 join。
        drop(guard);
        assert!(
            weak.upgrade().is_some(),
            "actual writer must own the root after guard timeout"
        );
        release.release();
        drop(sink);
        assert!(release.wait_for_actual_drop());
        assert!(writer_dropped_with_owner.load(Ordering::SeqCst));
        assert_eq!(
            std::fs::read(log_file).unwrap(),
            b"RF905 synthetic GUI log\n"
        );
    }
}

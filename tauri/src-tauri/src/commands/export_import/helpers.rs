#[cfg(test)]
use super::*;

// ── Internal helpers ──────────────────────────────────────────

// RF-023：既有测试入口复用 Core 唯一原子输出实现。
#[cfg(test)]
pub(crate) use solosoul_core::export_import::export_output::{
    create_export_output, finish_export_output, write_export_output,
};

pub use solosoul_core::export_import::import::package::ManifestData;
pub use solosoul_core::export_import::{
    build_package_ids, resolve_cross_scope_references, resolve_value_references,
};
pub(crate) fn read_manifest_json(path: &str) -> Result<serde_json::Value, String> {
    solosoul_core::export_import::import::package::read_manifest_json(path)
        .map_err(|error| crate::services::encrypted_import::map_import_failure(error.into()))
}
#[cfg(test)]
pub(crate) fn read_manifest_json_limited(
    path: &str,
    limit: u64,
) -> Result<serde_json::Value, String> {
    solosoul_core::export_import::import::package::read_manifest_json_limited(path, limit)
        .map_err(|error| crate::services::encrypted_import::map_import_failure(error.into()))
}
pub fn read_manifest(path: &str) -> Result<ManifestData, String> {
    solosoul_core::export_import::import::package::read_manifest(path)
        .map_err(|error| crate::services::encrypted_import::map_import_failure(error.into()))
}
/// 为导入的副本生成不冲突的名称，参考回收站命名冲突机制。
/// 根据 locale 选择后缀：
/// - 中文（zh-* / cmn-*）："(原始名称)（导入）" → "(原始名称)（导入 2）"
/// - 其他（默认 en-US）："(原始名称) (Imported)" → "(原始名称) (Imported 2)"
///
/// # 性能
/// 只查询数据库一次，将结果缓存在 HashSet 中做后续判断。
#[cfg(test)]
pub(crate) fn unique_object_name(
    vault: &solosoul_vault::VaultStore,
    account_id: &str,
    base_name: &str,
    locale: &str,
) -> Result<String, String> {
    use std::collections::HashSet;
    let (suffix, sep) = if locale.starts_with("zh") || locale.starts_with("cmn") {
        ("（导入）", " ")
    } else {
        (" (Imported)", " ")
    };
    let names: HashSet<String> = vault
        .list_objects(account_id, None, None, None, false, false)?
        .into_iter()
        .map(|o| o.name)
        .collect();

    let candidate = format!("{}{}", base_name, suffix);
    if !names.contains(&candidate) {
        return Ok(candidate);
    }

    let mut counter = 2u32;
    loop {
        let candidate = format!("{}{}{}{}", base_name, suffix, sep, counter);
        if !names.contains(&candidate) {
            return Ok(candidate);
        }
        counter += 1;
    }
}

/// 若 `obj[key]` 是字符串且在 id_map 中命中，则替换为新 ID。
#[cfg(test)]
pub fn decrypt_zip_entry_streaming(
    path: &str,
    name: &str,
    key: &[u8; 32],
    writer: &mut impl std::io::Write,
) -> Result<(), String> {
    solosoul_core::export_import::import::package::decrypt_zip_entry_streaming(
        path, name, key, writer,
    )
    .map_err(|error| crate::services::encrypted_import::map_import_failure(error.into()))
}
#[cfg(test)]
pub fn read_file_from_zip(path: &str, name: &str) -> Result<Vec<u8>, String> {
    solosoul_core::export_import::import::package::read_file_from_zip(path, name)
        .map_err(|error| crate::services::encrypted_import::map_import_failure(error.into()))
}

#[cfg(test)]
mod rf017_output_tests {
    use super::*;
    use std::cell::Cell;
    use std::io::{self, Seek, SeekFrom};
    use std::path::PathBuf;
    use std::rc::Rc;

    const OLD_BYTES: &[u8] = b"synthetic existing export must survive";

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Fault {
        None,
        Write,
        Seek,
        Flush,
        Sync,
    }

    struct FailingFile {
        file: File,
        fault: Rc<Cell<Fault>>,
        closed: Rc<Cell<bool>>,
    }

    impl Read for FailingFile {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            self.file.read(bytes)
        }
    }

    impl Write for FailingFile {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.fault.get() == Fault::Write {
                return Err(io::Error::other("rf017 synthetic write failure"));
            }
            self.file.write(bytes)
        }

        fn flush(&mut self) -> io::Result<()> {
            if self.fault.get() == Fault::Flush {
                return Err(io::Error::other("rf017 synthetic flush failure"));
            }
            self.file.flush()
        }
    }

    impl Seek for FailingFile {
        fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
            if self.fault.get() == Fault::Seek {
                return Err(io::Error::other("rf017 synthetic seek failure"));
            }
            self.file.seek(position)
        }
    }

    impl Drop for FailingFile {
        fn drop(&mut self) {
            self.closed.set(true);
        }
    }

    struct OutputFixture {
        directory: tempfile::TempDir,
        target: PathBuf,
        fault: Rc<Cell<Fault>>,
        closed: Rc<Cell<bool>>,
    }

    impl OutputFixture {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let target = directory.path().join("synthetic-existing.solosoul");
            std::fs::write(&target, OLD_BYTES).unwrap();
            Self {
                directory,
                target,
                fault: Rc::new(Cell::new(Fault::None)),
                closed: Rc::new(Cell::new(false)),
            }
        }

        fn output(&self) -> (FailingFile, tempfile::TempPath) {
            let (file, output) = create_export_output(&self.target).unwrap();
            assert_eq!(output.parent(), self.target.parent());
            (
                FailingFile {
                    file,
                    fault: self.fault.clone(),
                    closed: self.closed.clone(),
                },
                output,
            )
        }

        fn zip(&self) -> (ZipWriter<FailingFile>, tempfile::TempPath) {
            let (file, output) = self.output();
            let mut zip = ZipWriter::new(file);
            // Stored + 显式禁用逐条刷新，让 Flush 故障准确发生在 finish 后的生产刷新阶段。
            zip.set_flush_on_finish_file(false);
            zip.start_file(
                "synthetic.txt",
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
            zip.write_all(b"synthetic replacement content").unwrap();
            (zip, output)
        }

        fn assert_preserved(&self) {
            assert!(self.closed.get(), "临时文件句柄必须先释放");
            assert_eq!(std::fs::read(&self.target).unwrap(), OLD_BYTES);
            let paths: Vec<_> = std::fs::read_dir(self.directory.path())
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect();
            assert_eq!(paths, vec![self.target.clone()], "失败后不残留临时输出");
        }
    }

    #[test]
    fn rf017_zip_body_write_failure_preserves_target() {
        fn write_body(fixture: &OutputFixture) -> Result<(), String> {
            let (file, output) = fixture.output();
            let zip = write_export_output(file, |zip| {
                zip.start_file(
                    "synthetic.txt",
                    SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
                )
                .map_err(|e| format!("Write ZIP: {e}"))?;
                zip.write_all(b"synthetic replacement content")
                    .map_err(|e| format!("Write ZIP: {e}"))?;
                fixture.fault.set(Fault::Write);
                zip.write_all(b"synthetic additional body")
                    .map_err(|e| format!("Write ZIP: {e}"))
            })?;
            finish_export_output(
                zip,
                output,
                |_| panic!("正文写入失败后不能同步"),
                |_| panic!("正文写入失败后不能发布"),
            )
        }

        let fixture = OutputFixture::new();
        let error = write_body(&fixture).unwrap_err();
        assert!(error.starts_with("Write ZIP:"), "{error}");
        fixture.assert_preserved();
    }

    #[test]
    fn rf017_next_entry_close_failure_preserves_target() {
        fn write_next_entry(fixture: &OutputFixture) -> Result<(), String> {
            let (file, output) = fixture.output();
            let zip = write_export_output(file, |zip| {
                let options =
                    SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
                zip.start_file("first.txt", options)
                    .map_err(|e| format!("Write ZIP: {e}"))?;
                zip.write_all(b"synthetic first entry content")
                    .map_err(|e| format!("Write ZIP: {e}"))?;
                // 持续 Write 故障不降级：开始下一条目时收尾前条目，头写入失败后 Drop 再次收尾。
                fixture.fault.set(Fault::Write);
                zip.start_file("second.txt", options)
                    .map_err(|e| format!("Write ZIP: {e}"))
            })?;
            finish_export_output(
                zip,
                output,
                |_| panic!("切换条目失败后不能同步"),
                |_| panic!("切换条目失败后不能发布"),
            )
        }

        let fixture = OutputFixture::new();
        let error = write_next_entry(&fixture).unwrap_err();
        assert!(error.starts_with("Write ZIP:"), "{error}");
        fixture.assert_preserved();
    }

    #[test]
    fn rf017_zip_finish_write_and_seek_failures_preserve_target() {
        for fault in [Fault::Write, Fault::Seek] {
            let fixture = OutputFixture::new();
            let (zip, output) = fixture.zip();
            // 已写入真实 ZIP 内容后才启用故障，实际执行 ZipWriter::finish 的写入/seek。
            fixture.fault.set(fault);
            let error = finish_export_output(
                zip,
                output,
                |_| panic!("ZIP finish 失败后不能同步"),
                |_| panic!("ZIP finish 失败后不能发布"),
            )
            .unwrap_err();
            assert!(error.starts_with("ZIP finish:"), "{error}");
            fixture.assert_preserved();
        }
    }

    #[test]
    fn rf017_flush_and_sync_failures_preserve_target() {
        for fault in [Fault::Flush, Fault::Sync] {
            let fixture = OutputFixture::new();
            let (zip, output) = fixture.zip();
            fixture.fault.set(fault);
            let synced = Cell::new(false);
            let error = finish_export_output(
                zip,
                output,
                |_| {
                    synced.set(true);
                    Err(io::Error::other("rf017 synthetic sync failure"))
                },
                |_| panic!("刷新/同步失败后不能发布"),
            )
            .unwrap_err();
            let stage = if fault == Fault::Flush {
                "Flush ZIP:"
            } else {
                "Sync ZIP:"
            };
            assert!(error.starts_with(stage), "{error}");
            assert_eq!(synced.get(), fault == Fault::Sync);
            fixture.assert_preserved();
        }
    }

    #[test]
    fn rf017_persist_failure_preserves_destination_and_cleans_output() {
        let directory = tempfile::tempdir().unwrap();
        // 真实 persist 错误：最终目标是含合成文件的目录，不能被 ZIP 文件替换。
        let target = directory.path().join("synthetic-existing.solosoul");
        std::fs::create_dir(&target).unwrap();
        let sentinel = target.join("keep.txt");
        std::fs::write(&sentinel, OLD_BYTES).unwrap();
        let (file, output) = create_export_output(&target).unwrap();
        let temporary_path = output.to_path_buf();
        let mut zip = ZipWriter::new(file);
        zip.start_file("synthetic.txt", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"synthetic replacement content").unwrap();
        let error = finish_export_output(zip, output, File::sync_all, |output| {
            output
                .persist(&target)
                .map_err(|e| format!("Publish ZIP: {}", e.error))
        })
        .unwrap_err();
        assert!(error.starts_with("Publish ZIP:"), "{error}");
        assert_eq!(std::fs::read(&sentinel).unwrap(), OLD_BYTES);
        assert!(!temporary_path.exists());
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
        assert_eq!(std::fs::read_dir(&target).unwrap().count(), 1);
    }
}

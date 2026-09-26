use super::*;

// ── Internal helpers ──────────────────────────────────────────

/// RF-017：输出临时文件与最终目标同目录，保证发布不跨文件系统。
pub(crate) fn create_export_output(
    target: &std::path::Path,
) -> Result<(File, tempfile::TempPath), String> {
    let parent = target
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    tempfile::Builder::new()
        .prefix(".solosoul-export-")
        .suffix(".tmp")
        .tempfile_in(parent)
        .map(|file| file.into_parts())
        .map_err(|e| format!("Create ZIP temporary file: {e}"))
}

/// RF-017：只隔离 ZIP 写入及失败析构；TempPath 由调用者持有，收集/KDF/发布不进入此范围。
pub(crate) fn write_export_output<W: Write + std::io::Seek>(
    file: W,
    write: impl FnOnce(&mut ZipWriter<W>) -> Result<(), String>,
) -> Result<ZipWriter<W>, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        move || -> Result<ZipWriter<W>, String> {
            let mut zip = ZipWriter::new(file);
            // start_file 会先收尾前一条目；写局部头失败后 Drop 重试也可能触发 zip 2.4 断言。
            write(&mut zip)?;
            Ok(zip)
        },
    ))
    .map_err(|_| "Write ZIP: writer cleanup failed".to_string())?
}

/// RF-017：ZIP 完成、刷新、同步并关闭句柄后才能发布。
/// 独立作用域确保任一收尾错误先释放 writer，再由 TempPath 清理临时文件（含 Windows）。
pub(crate) fn finish_export_output<W: Write + std::io::Seek>(
    zip: ZipWriter<W>,
    output: tempfile::TempPath,
    sync: impl FnOnce(&W) -> std::io::Result<()>,
    publish: impl FnOnce(tempfile::TempPath) -> Result<(), String>,
) -> Result<(), String> {
    // zip 2.4 的局部头写入失败后，Drop 重试收尾可能因游标位置触发断言。
    // 这里只消费 writer 并隔离该清理 panic；TempPath 留在外层，退栈关闭文件后再清理。
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || zip.finish()))
        .map_err(|_| "ZIP finish: writer cleanup failed".to_string())?
        .map_err(|e| format!("ZIP finish: {e}"))
        .and_then(|mut file| {
            file.flush().map_err(|e| format!("Flush ZIP: {e}"))?;
            sync(&file).map_err(|e| format!("Sync ZIP: {e}"))?;
            drop(file);
            Ok(())
        })?;
    publish(output)
}

pub struct ManifestData {
    pub salt_hex: String,
    pub has_attachments: bool,
    pub extra_files: Vec<String>,
    /// manifest 声明的 KDF 参数；`None` = 旧格式包（未声明），按 balanced 兜底。
    pub kdf: Option<solosoul_crypto::kdf::KdfConfig>,
}

impl ManifestData {
    /// 用于解包/加密的 KDF 参数：manifest 声明优先，旧格式包回退 balanced（向后兼容）。
    pub fn kdf_config(&self) -> solosoul_crypto::kdf::KdfConfig {
        self.kdf
            .unwrap_or_else(solosoul_crypto::kdf::KdfConfig::balanced)
    }
}

// ── Cross-scope reference resolution（P010: 由 solosoul-core 单一实现）──
//
// build_package_ids / resolve_value_references / resolve_cross_scope_references
// 原在本文件与 solosoul-core::export_import 逐字重复（相似度 ≈100%）。
// 已收敛为 core 侧 pub 实现，此处 re-export 保持既有调用路径（`helpers::*`）不变。
//
// P010 备注：derive_export_key(_cfg) 两侧均已是 solosoul-crypto::kdf 薄包装（P024 收敛），
// 仅错误类型映射不同，不属重复实现。
pub use solosoul_core::export_import::{
    build_package_ids, resolve_cross_scope_references, resolve_value_references,
};

/// 读取 ZIP 包内 manifest.json 并解析为 JSON 值。
///
/// # 安全（P201）
/// - 读取前检查条目声明的未压缩大小，超过 `MAX_ZIP_ENTRY_SIZE`（100 MB）则拒绝；
/// - 使用 `.take()` 限制实际读取字节数作为第二道防线，即使 `size()` 不可信（返回 0/伪造）
///   也不会一次性读入超大块内存导致 OOM。
pub(crate) fn read_manifest_json(file_path: &str) -> Result<serde_json::Value, String> {
    read_manifest_json_limited(file_path, MAX_ZIP_ENTRY_SIZE)
}

/// `read_manifest_json` 的带参版本：以 `max_size` 为上限读取并解析 manifest.json。
/// 上限参数化便于单测用极小值触发拒绝路径，无需在测试中构造 100MB 真实包。
pub(crate) fn read_manifest_json_limited(
    file_path: &str,
    max_size: u64,
) -> Result<serde_json::Value, String> {
    let path = std::path::Path::new(file_path);
    if !path.exists() {
        return Err(import_err_with_detail("FILE_NOT_FOUND", file_path));
    }
    let file = File::open(path).map_err(|e| format!("Cannot open: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|_| import_err("INVALID_PACKAGE"))?;

    let entry = archive
        .by_name("manifest.json")
        .map_err(|_| import_err("MISSING_MANIFEST"))?;
    if entry.size() > max_size {
        return Err(format!(
            "manifest.json is too large ({} bytes, max {} bytes)",
            entry.size(),
            max_size
        ));
    }
    let mut buf = Vec::new();
    entry
        .take(max_size + 1)
        .read_to_end(&mut buf)
        .map_err(|e| format!("Read manifest: {}", e))?;
    // 第二道防线：即使条目声明的 size() 不可信（偏小），实际读取字节数超限也拒绝。
    if buf.len() as u64 > max_size {
        return Err(format!(
            "manifest.json exceeds size limit ({} bytes, max {} bytes)",
            buf.len(),
            max_size
        ));
    }
    let s = String::from_utf8_lossy(&buf);
    serde_json::from_str(&s).map_err(|e| format!("Invalid manifest: {}", e))
}

pub fn read_manifest(file_path: &str) -> Result<ManifestData, String> {
    let v = read_manifest_json(file_path)?;

    let extra_files: Vec<String> = v
        .get("extra_files")
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Ok(ManifestData {
        salt_hex: v["salt_hex"]
            .as_str()
            .ok_or(import_err("MISSING_SALT"))?
            .to_string(),
        has_attachments: v["has_attachments"].as_bool().unwrap_or(false),
        extra_files,
        kdf: solosoul_core::export_import::kdf_from_manifest_value(v.get("kdf"))?,
    })
}

/// ZIP 条目的最大解压大小限制（100 MB），防止 ZIP 炸弹 / OOM。
const MAX_ZIP_ENTRY_SIZE: u64 = 100 * 1024 * 1024;

/// 从 ZIP 中读取指定名称的文件内容，带大小限制。
///
/// # 安全
/// - 读取前检查 `entry.size()`（未压缩大小），超过 `MAX_ZIP_ENTRY_SIZE` 则拒绝。
/// - 使用 `.take()` 限制实际读取字节数，即使 `size()` 返回 0/错误也有第二道防线。
///
/// 为导入的副本生成不冲突的名称，参考回收站命名冲突机制。
/// 根据 locale 选择后缀：
/// - 中文（zh-* / cmn-*）："(原始名称)（导入）" → "(原始名称)（导入 2）"
/// - 其他（默认 en-US）："(原始名称) (Imported)" → "(原始名称) (Imported 2)"
///
/// # 性能
/// 只查询数据库一次，将结果缓存在 HashSet 中做后续判断。
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
fn rewrite_str_ref(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    id_map: &std::collections::HashMap<String, String>,
) {
    let Some(val) = obj.get_mut(key) else {
        return;
    };
    let Some(s) = val.as_str() else {
        return;
    };
    if let Some(new_id) = id_map.get(s) {
        *val = serde_json::Value::String(new_id.clone());
    }
}

/// 若 `obj[key]` 是字符串数组，则逐元素在 id_map 中命中后替换。
fn rewrite_str_array_ref(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    id_map: &std::collections::HashMap<String, String>,
) {
    let Some(arr) = obj.get_mut(key).and_then(|v| v.as_array_mut()) else {
        return;
    };
    for item in arr.iter_mut() {
        let Some(s) = item.as_str() else {
            continue;
        };
        if let Some(new_id) = id_map.get(s) {
            *item = serde_json::Value::String(new_id.clone());
        }
    }
}

/// 递归扫描 JSON 值，将旧 ID 引用替换为新 ID。
/// 处理以下模式：
/// - `"parentId"` 或 `"parent_id"` 字符串字段
/// - `"childrenIds"` 或 `"children_ids"` 数组字段
/// - RelationProperty 对象中的 `"targetId"` / `"id"` / `"objectId"`
pub(crate) fn rewrite_id_references(
    value: &mut serde_json::Value,
    id_map: &std::collections::HashMap<String, String>,
) {
    match value {
        serde_json::Value::Object(obj) => {
            // 检查是否为 RelationProperty
            let is_relation = obj
                .get("type")
                .or_else(|| obj.get("__type"))
                .or_else(|| obj.get("kind"))
                .and_then(|v| v.as_str())
                == Some("relation");
            if is_relation {
                for key in ["targetId", "id", "objectId"] {
                    rewrite_str_ref(obj, key, id_map);
                }
            }
            // 递归处理子对象
            let keys: Vec<String> = obj.keys().cloned().collect();
            for key in &keys {
                match key.as_str() {
                    "parent_id" | "parentId" => rewrite_str_ref(obj, key, id_map),
                    "children_ids" | "childrenIds" => rewrite_str_array_ref(obj, key, id_map),
                    _ => {
                        if let Some(val) = obj.get_mut(key) {
                            rewrite_id_references(val, id_map);
                        }
                    }
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                rewrite_id_references(item, id_map);
            }
        }
        _ => {}
    }
}

/// 流式读取 ZIP 条目并分块解密（用于 `payload.enc`），明文直接写入 `writer`。
///
/// 与 `read_file_from_zip` 的防 ZIP 炸弹上限（`MAX_ZIP_ENTRY_SIZE`）一致，
/// 但不在内存中同时驻留整份密文与明文——R2-15：导入峰值内存由约 3× payload
/// （密文 + 明文 + JSON 树）降至约 1×（仅 JSON 树，经临时文件流式解析）。
pub fn decrypt_zip_entry_streaming(
    file_path: &str,
    name: &str,
    key: &[u8; 32],
    writer: &mut impl std::io::Write,
) -> Result<(), String> {
    let path = std::path::Path::new(file_path);
    let file = File::open(path).map_err(|e| format!("Cannot open: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|_| "Invalid ZIP".to_string())?;
    let entry = archive
        .by_name(name)
        .map_err(|_| format!("File not found: {}", name))?;

    if entry.size() > MAX_ZIP_ENTRY_SIZE {
        return Err(format!(
            "ZIP entry '{}' is too large ({} bytes, max {} bytes)",
            name,
            entry.size(),
            MAX_ZIP_ENTRY_SIZE
        ));
    }

    let mut limited = entry.take(MAX_ZIP_ENTRY_SIZE + 1);
    solosoul_crypto::cipher::decrypt_chunked_stream(key, &mut limited, writer)
        .map_err(|_| import_err("DECRYPT_FAILED"))
}

pub fn read_file_from_zip(file_path: &str, name: &str) -> Result<Vec<u8>, String> {
    let path = std::path::Path::new(file_path);
    let file = File::open(path).map_err(|e| format!("Cannot open: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|_| "Invalid ZIP".to_string())?;
    let entry = archive
        .by_name(name)
        .map_err(|_| format!("File not found: {}", name))?;

    // 检查 ZIP 条目声明的未压缩大小
    if entry.size() > MAX_ZIP_ENTRY_SIZE {
        return Err(format!(
            "ZIP entry '{}' is too large ({} bytes, max {} bytes)",
            name,
            entry.size(),
            MAX_ZIP_ENTRY_SIZE
        ));
    }

    let mut buf = Vec::new();
    // 使用 take() 作为第二道防线（即使 entry.size() 不准确）
    entry
        .take(MAX_ZIP_ENTRY_SIZE + 1)
        .read_to_end(&mut buf)
        .map_err(|e| format!("Read {}: {}", name, e))?;

    Ok(buf)
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

//! 同目录临时 ZIP 的写入、收尾与原子发布；不依赖主机框架。
use std::fs::File;
use std::io::Write;
use zip::ZipWriter;

pub fn create_export_output(
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

pub fn write_export_output<W: Write + std::io::Seek>(
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

pub fn finish_export_output<W: Write + std::io::Seek>(
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

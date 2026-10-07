//! 已验收的公开媒体合同，原生 feature 与 CLI 共用，不复制校验策略。
use serde_json::Value;
use std::path::Path;
const PASSWORD: &str = "perf-baseline-only-password";
#[path = "../../../crates/solosoul-core/examples/perf_baseline/object.rs"]
mod object;
use object::make_object;
#[allow(dead_code)]
#[path = "../../../crates/solosoul-core/examples/perf_baseline/fixture.rs"]
mod fixture;
#[allow(dead_code)]
#[path = "../../../crates/solosoul-core/examples/perf_baseline/media.rs"]
mod media;
pub(super) fn marker(base: &Path) -> Result<Value, String> {
    let marker = media::native_manifest(base)?;
    if marker["baseFixture"]["buildProfile"] != "release"
        || marker["baseFixture"]["kdf"]
            != serde_json::json!({"memoryKiB":65536,"iterations":3,"parallelism":4})
    {
        return Err("native media fixture requires production KDF".into());
    }
    Ok(marker)
}
pub(super) fn copy(source: &Path, output: &Path) -> Result<(), String> {
    media::native_copy(source, output).map(|_| ())
}

#[cfg(test)]
pub(super) fn write_test_fixture(path: &Path) {
    media::native_test_fixture(path).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_marker_uses_persisted_kdf_after_environment_change() {
        let _guard = crate::VAULT_TEST_LOCK.lock().unwrap();
        struct RestoreSecure(Option<std::ffi::OsString>);
        impl Drop for RestoreSecure {
            fn drop(&mut self) {
                match self.0.take() {
                    Some(value) => std::env::set_var("SOLOSOUL_SECURE", value),
                    None => std::env::remove_var("SOLOSOUL_SECURE"),
                }
            }
        }
        let _restore = RestoreSecure(std::env::var_os("SOLOSOUL_SECURE"));
        // 显式覆盖两个方向，复现创建账户与发布标记之间的环境变化。
        for (before, after) in [("0", "1"), ("1", "0")] {
            std::env::set_var("SOLOSOUL_SECURE", before);
            let temporary = tempfile::tempdir().unwrap();
            let base = temporary.path().join("fixture");
            let (_, original) = fixture::populate(&base, 100).unwrap();
            let config_path = base.join("acc_rf312_100/config.json");
            let config_before = std::fs::read(&config_path).unwrap();
            let config: solosoul_core::vault_service::AccountConfig =
                serde_json::from_slice(&config_before).unwrap();
            let kdf = config.kdf_config();
            std::env::set_var("SOLOSOUL_SECURE", after);
            let marker = fixture::completed_manifest(&base, 100).unwrap();
            assert_eq!(
                marker, original,
                "KDF environment changed {before} -> {after}"
            );
            assert_eq!(
                marker["kdf"],
                serde_json::json!({"memoryKiB":kdf.memory_kb,"iterations":kdf.iterations,"parallelism":kdf.parallelism})
            );
            assert_eq!(std::fs::read(&config_path).unwrap(), config_before);
            assert_eq!(
                fixture::verify_data(&base, &marker).unwrap()["success"],
                true
            );
        }
    }
}

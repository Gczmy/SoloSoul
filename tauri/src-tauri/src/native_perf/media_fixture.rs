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

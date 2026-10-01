//! 主机错误契约映射；Core 只返回业务错误，不知道 IPC/i18n 前缀。
use solosoul_core::export_import::export::ExportFailure;
pub(crate) const EXPORT_ERR_PREFIX: &str = "__EXPORT_ERR__:";
pub(crate) fn map_export_failure(error: ExportFailure) -> String {
    let (code, detail) = match error {
        ExportFailure::PasswordEmpty => ("PASSWORD_EMPTY", None),
        ExportFailure::SameAsMasterPassword => ("SAME_AS_MASTER_PASSWORD", None),
        ExportFailure::MasterVerifyFailed(detail) => ("MASTER_VERIFY_FAILED", Some(detail)),
        ExportFailure::NoObjectsSelected => ("NO_OBJECTS_SELECTED", None),
        ExportFailure::AttachmentTooLarge(detail) => ("ATTACHMENT_TOO_LARGE", Some(detail)),
        ExportFailure::TotalSizeExceeded => ("TOTAL_SIZE_EXCEEDED", None),
        ExportFailure::Backend(error) => return error.to_string(),
    };
    match detail {
        Some(detail) => format!("{EXPORT_ERR_PREFIX}{code}:{detail}"),
        None => format!("{EXPORT_ERR_PREFIX}{code}"),
    }
}

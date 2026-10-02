//! 文档导出走实际批量解密 helper，缺对象不泄露 ID。
use super::*;
use crate::commands::export_import::tests::rf020::Fixture;
#[test]
fn rf318_document_scope_and_missing_object_are_distinct_safe_packets() {
    let f = Fixture::new();
    for (ids, code) in [
        (vec![], "EXPORT_SCOPE_EMPTY"),
        (
            vec!["RF318_PRIVATE_MISSING_ID".into()],
            "EXPORT_OBJECT_NOT_FOUND",
        ),
    ] {
        let error = load_records_in_order_safe(&f.vault, &ids).unwrap_err();
        let p = serde_json::to_value(BackendError::from(error)).unwrap();
        assert_eq!(p["code"], code);
        assert!(!p.to_string().contains("RF318_PRIVATE"));
    }
    assert_eq!(
        f.db.query_row("SELECT COUNT(*) FROM objects", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
}

use crate::commands::vault_handle;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn profile_load(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Option<solosoul_vault::Profile>, String> {
    let vault = vault_handle(&state)?;
    vault.load_profile(&account_id)
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use solosoul_vault::{Profile, VaultConfig, VaultStore};
    use tempfile::TempDir;

    #[derive(Serialize, Deserialize)]
    struct SectionData {
        section_type: String,
        fields: Vec<FieldValue>,
    }

    #[derive(Serialize, Deserialize)]
    struct FieldValue {
        key: String,
        label: String,
        value: serde_json::Value,
        sensitivity_level: Option<String>,
    }

    /// Parse a section from profile JSON data.
    fn parse_section_from_profile_data(
        data: &serde_json::Value,
        section_type: &str,
    ) -> Option<SectionData> {
        let sections = data.get("sections").and_then(|s| s.as_array())?;
        for sec in sections {
            if sec.get("type").and_then(|t| t.as_str()) == Some(section_type) {
                let st = sec
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                let fields: Vec<FieldValue> = sec
                    .get("fields")
                    .and_then(|f| f.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|f| FieldValue {
                                key: f
                                    .get("key")
                                    .and_then(|k| k.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                label: f
                                    .get("label")
                                    .and_then(|l| l.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                value: f.get("value").cloned().unwrap_or(serde_json::Value::Null),
                                sensitivity_level: f
                                    .get("sensitivityLevel")
                                    .and_then(|s| s.as_str())
                                    .map(|s| s.to_string()),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                return Some(SectionData {
                    section_type: st,
                    fields,
                });
            }
        }
        None
    }

    fn setup_vault() -> (VaultStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let config =
            VaultConfig::new("test_account", dir.path().to_path_buf()).with_data_key([0x42u8; 32]);
        let vault = VaultStore::open(config).unwrap();
        (vault, dir)
    }

    fn sample_profile_data() -> serde_json::Value {
        serde_json::json!({
            "sections": [
                {
                    "type": "identity",
                    "fields": [
                        {"key": "name", "label": "Name", "value": "Alice", "sensitivityLevel": "public"},
                        {"key": "email", "label": "Email", "value": "alice@example.com", "sensitivityLevel": "private"}
                    ]
                },
                {
                    "type": "travel",
                    "fields": [
                        {"key": "passport", "label": "Passport", "value": "P123456", "sensitivityLevel": "restricted"}
                    ]
                }
            ]
        })
    }

    #[test]
    fn test_parse_section_from_profile_data_found() {
        let data = sample_profile_data();
        let section = parse_section_from_profile_data(&data, "identity").unwrap();
        assert_eq!(section.section_type, "identity");
        assert_eq!(section.fields.len(), 2);
        assert_eq!(section.fields[0].key, "name");
        assert_eq!(section.fields[0].value, "Alice");
        assert_eq!(
            section.fields[0].sensitivity_level,
            Some("public".to_string())
        );
    }

    #[test]
    fn test_parse_section_from_profile_data_not_found() {
        let data = sample_profile_data();
        assert!(parse_section_from_profile_data(&data, "financial").is_none());
    }

    #[test]
    fn test_parse_section_from_profile_data_no_sections() {
        let data = serde_json::json!({"other": "value"});
        assert!(parse_section_from_profile_data(&data, "identity").is_none());
    }

    #[test]
    fn test_parse_section_missing_optional_fields() {
        let data = serde_json::json!({
            "sections": [{
                "type": "basic",
                "fields": [{"key": "k1", "label": "L1", "value": "v1"}]
            }]
        });
        let section = parse_section_from_profile_data(&data, "basic").unwrap();
        assert_eq!(section.fields[0].sensitivity_level, None);
    }

    #[test]
    fn test_vault_profile_save_and_load() {
        let (vault, _dir) = setup_vault();
        let data = serde_json::to_vec(&sample_profile_data()).unwrap();
        let profile = Profile::new_with_id("acc-1", "Alice", data);
        vault.save_profile(&profile).unwrap();

        let loaded = vault.load_profile("acc-1").unwrap().unwrap();
        assert_eq!(loaded.name, "Alice");
        let parsed: serde_json::Value = serde_json::from_slice(&loaded.data).unwrap();
        assert!(parsed.get("sections").is_some());
    }

    #[test]
    fn test_vault_profile_update_field_logic() {
        let (vault, _dir) = setup_vault();
        let data = serde_json::to_vec(&sample_profile_data()).unwrap();
        let mut profile = Profile::new_with_id("acc-1", "Alice", data);
        vault.save_profile(&profile).unwrap();

        // Simulate profile_update_field logic
        let mut parsed: serde_json::Value = serde_json::from_slice(&profile.data).unwrap();
        if let Some(sections) = parsed.get_mut("sections").and_then(|s| s.as_array_mut()) {
            for sec in sections.iter_mut() {
                if sec.get("type").and_then(|t| t.as_str()) == Some("identity") {
                    if let Some(fields) = sec.get_mut("fields").and_then(|f| f.as_array_mut()) {
                        for field in fields.iter_mut() {
                            if field.get("key").and_then(|k| k.as_str()) == Some("name") {
                                field["value"] = serde_json::json!("Bob");
                                break;
                            }
                        }
                    }
                    break;
                }
            }
        }
        profile.data = serde_json::to_vec(&parsed).unwrap();
        profile.version += 1;
        vault.save_profile(&profile).unwrap();

        let loaded = vault.load_profile("acc-1").unwrap().unwrap();
        let loaded_data: serde_json::Value = serde_json::from_slice(&loaded.data).unwrap();
        let section = parse_section_from_profile_data(&loaded_data, "identity").unwrap();
        let name_field = section.fields.iter().find(|f| f.key == "name").unwrap();
        assert_eq!(name_field.value, "Bob");
        assert_eq!(loaded.version, 2);
    }

    #[test]
    fn test_field_value_serde_roundtrip() {
        let original = FieldValue {
            key: "email".to_string(),
            label: "Email".to_string(),
            value: serde_json::json!("test@example.com"),
            sensitivity_level: Some("private".to_string()),
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: FieldValue = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.key, original.key);
        assert_eq!(restored.value, original.value);
        assert_eq!(restored.sensitivity_level, original.sensitivity_level);
    }
}

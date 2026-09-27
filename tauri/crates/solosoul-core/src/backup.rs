//! GUI / CLI 共用的 Profile 备份编解码；不读取 Vault 或写入文件。
//!
//! 1.0 / 2.0 使用相同元数据结构。编码器仅生成 2.0，宿主决定载荷编码。

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use solosoul_vault::Profile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfilePayloadEncoding {
    Base64,
    ByteArray,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileBackupHeader {
    pub version: String,
    pub created_at: String,
    pub profile_count: usize,
}

#[derive(Debug)]
pub struct DecodedProfileBackup {
    pub header: ProfileBackupHeader,
    pub profiles: Vec<Profile>,
}

/// 错误不包含 Profile 标识、名称或载荷；条目索引从 0 开始。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProfileBackupError {
    #[error("Invalid Profile backup JSON: {reason}")]
    Json { reason: String },
    #[error("Unsupported Profile backup version")]
    UnsupportedVersion,
    #[error("Profile backup count mismatch: declared {declared}, actual {actual}")]
    CountMismatch { declared: usize, actual: usize },
    #[error("Profile backup entry {index} is missing data")]
    MissingData { index: usize },
    #[error("Profile backup entry {index} has invalid Base64: {reason}")]
    InvalidBase64 { index: usize, reason: String },
}

#[derive(Serialize, Deserialize)]
struct WireManifest {
    version: String,
    created_at: String,
    profile_count: usize,
    profiles: Vec<WireProfile>,
}

#[derive(Serialize, Deserialize)]
struct WireProfile {
    id: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data_b64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Vec<u8>>,
    created_at: String,
    updated_at: String,
    version: u32,
}

/// 保留任意原始字节；每个条目只输出选定编码的一个载荷键。
pub fn encode_profile_backup(
    profiles: &[Profile],
    created_at: DateTime<Utc>,
    encoding: ProfilePayloadEncoding,
) -> Result<Vec<u8>, ProfileBackupError> {
    let profiles = profiles
        .iter()
        .map(|profile| {
            let (data_b64, data) = match encoding {
                ProfilePayloadEncoding::Base64 => (Some(BASE64.encode(&profile.data)), None),
                ProfilePayloadEncoding::ByteArray => (None, Some(profile.data.clone())),
            };
            WireProfile {
                id: profile.id.clone(),
                name: profile.name.clone(),
                data_b64,
                data,
                created_at: profile.created_at.to_rfc3339(),
                updated_at: profile.updated_at.to_rfc3339(),
                version: profile.version,
            }
        })
        .collect::<Vec<_>>();
    let manifest = WireManifest {
        version: "2.0".to_string(),
        created_at: created_at.to_rfc3339(),
        profile_count: profiles.len(),
        profiles,
    };
    serde_json::to_vec_pretty(&manifest).map_err(json_error)
}

/// 完整验证并解码后才返回；宿主可在成功后开始写入 Vault。
pub fn decode_profile_backup(
    bytes: &[u8],
    fallback_now: DateTime<Utc>,
) -> Result<DecodedProfileBackup, ProfileBackupError> {
    // 直接反序列化结构体：未被优先选中的载荷字段仍须满足 JSON 类型及字节范围。
    let manifest: WireManifest = serde_json::from_slice(bytes).map_err(json_error)?;
    if !matches!(manifest.version.as_str(), "1.0" | "2.0") {
        return Err(ProfileBackupError::UnsupportedVersion);
    }
    if manifest.profile_count != manifest.profiles.len() {
        return Err(ProfileBackupError::CountMismatch {
            declared: manifest.profile_count,
            actual: manifest.profiles.len(),
        });
    }
    let profiles = manifest
        .profiles
        .into_iter()
        .enumerate()
        .map(|(index, entry)| {
            let data = match (entry.data_b64, entry.data) {
                (Some(encoded), _) if !encoded.is_empty() => BASE64
                    .decode(encoded)
                    .map_err(|error| base64_error(index, error))?,
                (_, Some(data)) => data,
                (Some(_), None) => Vec::new(),
                (None, None) => return Err(ProfileBackupError::MissingData { index }),
            };
            Ok(Profile {
                id: entry.id,
                name: entry.name,
                data,
                created_at: parse_profile_date(&entry.created_at, fallback_now),
                updated_at: parse_profile_date(&entry.updated_at, fallback_now),
                version: entry.version,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(DecodedProfileBackup {
        header: ProfileBackupHeader {
            version: manifest.version,
            created_at: manifest.created_at,
            profile_count: manifest.profile_count,
        },
        profiles,
    })
}

fn parse_profile_date(value: &str, fallback_now: DateTime<Utc>) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .map(|date| date.with_timezone(&Utc))
        .unwrap_or(fallback_now)
}

fn json_error(error: serde_json::Error) -> ProfileBackupError {
    // serde 的原始 Data 错误可能包含输入值，不能直接携带到宿主日志或提示中。
    let category = match error.classify() {
        serde_json::error::Category::Io => "I/O error",
        serde_json::error::Category::Syntax => "invalid JSON syntax",
        serde_json::error::Category::Data => "invalid field type or missing field",
        serde_json::error::Category::Eof => "unexpected end of JSON",
    };
    ProfileBackupError::Json {
        reason: format!(
            "{category} at line {}, column {}",
            error.line(),
            error.column()
        ),
    }
}

fn base64_error(index: usize, error: base64::DecodeError) -> ProfileBackupError {
    let reason = match error {
        base64::DecodeError::InvalidByte(_, _) => "invalid Base64 symbol",
        base64::DecodeError::InvalidLength(_) => "invalid Base64 length",
        base64::DecodeError::InvalidLastSymbol(_, _) => "invalid Base64 trailing symbol",
        base64::DecodeError::InvalidPadding => "invalid Base64 padding",
    };
    ProfileBackupError::InvalidBase64 {
        index,
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests;

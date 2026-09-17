//! 发布方控制的更新源。配置随客户端编译，不接受 WebView 注入任意下载地址。

use serde::Deserialize;
use url::Url;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct UpdateSources {
    pub manifest_endpoints: Vec<String>,
    pub download_bases: Vec<String>,
}

impl UpdateSources {
    pub fn load() -> Result<Self, String> {
        Self::parse(include_str!("../../update-sources.json"))
    }

    fn parse(json: &str) -> Result<Self, String> {
        let sources: Self =
            serde_json::from_str(json).map_err(|e| format!("更新源配置格式错误: {e}"))?;
        if sources.manifest_endpoints.len() > 8 || sources.download_bases.len() > 8 {
            return Err("更新源配置最多允许 8 个清单源及 8 个下载源".into());
        }
        for value in sources
            .manifest_endpoints
            .iter()
            .chain(&sources.download_bases)
        {
            let url = secure_url(value)?;
            if url.query().is_some() || url.fragment().is_some() {
                return Err("更新源配置不能包含查询参数或片段".into());
            }
        }
        Ok(sources)
    }

    pub fn release_endpoints(&self, tag: Option<&str>) -> Result<Vec<String>, String> {
        let directory = match tag {
            Some(tag) => {
                let version = tag.strip_prefix('v').unwrap_or(tag);
                semver::Version::parse(version).map_err(|_| "无效的更新版本号")?;
                format!("v{version}")
            }
            None => "latest".into(),
        };
        self.download_bases
            .iter()
            .map(|base| append_path(base, &[&directory, "release.json"]))
            .collect()
    }
}

pub(super) fn secure_url(value: &str) -> Result<Url, String> {
    let url = Url::parse(value).map_err(|_| "更新地址无效")?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err("更新地址必须是无凭据的 HTTPS 地址".into());
    }
    Ok(url)
}

fn append_path(base: &str, segments: &[&str]) -> Result<String, String> {
    let mut url = secure_url(base)?;
    {
        let mut path = url.path_segments_mut().map_err(|_| "更新源路径无效")?;
        path.pop_if_empty();
        for segment in segments {
            path.push(segment);
        }
    }
    Ok(url.to_string())
}

fn decode_filename(encoded: &str) -> Result<String, String> {
    let mut bytes = Vec::new();
    let mut chars = encoded.as_bytes().iter().copied();
    while let Some(byte) = chars.next() {
        if byte == b'%' {
            let high = chars.next().and_then(|b| (b as char).to_digit(16));
            let low = chars.next().and_then(|b| (b as char).to_digit(16));
            bytes.push(match (high, low) {
                (Some(h), Some(l)) => (h * 16 + l) as u8,
                _ => return Err("更新文件名编码无效".into()),
            });
        } else {
            bytes.push(byte);
        }
    }
    let name = String::from_utf8(bytes).map_err(|_| "更新文件名不是 UTF-8")?;
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains(['/', '\\'])
        || name.chars().any(char::is_control)
    {
        return Err("更新文件名无效".into());
    }
    Ok(name)
}

/// 所有候选始终指向同一版本/同一文件；完整性由调用方的签名或已签名 SHA-256 保证。
pub(super) fn artifact_candidates(
    original: &str,
    version: &str,
    sources: &UpdateSources,
    proxies: &[String],
) -> Result<Vec<String>, String> {
    let version = version.strip_prefix('v').unwrap_or(version);
    semver::Version::parse(version).map_err(|_| "无效的更新版本号")?;
    let original_url = secure_url(original)?;
    let name = decode_filename(
        original_url
            .path_segments()
            .and_then(|mut p| p.next_back())
            .ok_or("更新地址缺少文件名")?,
    )?;
    let tag = format!("v{version}");
    let github = append_path(
        "https://github.com/Gczmy/SoloSoul/releases/download",
        &[&tag, &name],
    )?;
    let mut candidates = Vec::new();
    for base in &sources.download_bases {
        candidates.push(append_path(base, &[&tag, &name])?);
    }
    // 清单可能从公共代理读取，但包传输应先测试官方源/GitHub，而非固定锁在该代理。
    let is_wrapped_github =
        original.contains("/https://github.com/Gczmy/SoloSoul/releases/download/");
    if !is_wrapped_github {
        candidates.push(original.into());
    }
    candidates.push(github.clone());
    for prefix in proxies {
        secure_url(prefix)?;
        candidates.push(format!("{prefix}{github}"));
    }
    // 原地址若来自公共代理，仅在该代理仍获准使用时加入（置空代理环境变量须生效）。
    if is_wrapped_github && proxies.iter().any(|p| original.starts_with(p)) {
        candidates.push(original.into());
    }
    let mut seen = std::collections::HashSet::new();
    candidates.retain(|candidate| seen.insert(candidate.clone()));
    Ok(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuration_rejects_insecure_and_credential_urls() {
        for url in [
            "http://cdn.example/releases",
            "https://user:secret@cdn.example/",
            "https://cdn.example/?token=x",
            "https://cdn.example/#x",
        ] {
            let json = serde_json::json!({"manifestEndpoints":[], "downloadBases":[url]});
            assert!(UpdateSources::parse(&json.to_string()).is_err());
        }
        assert!(UpdateSources::load().is_ok());
    }

    #[test]
    fn candidates_prefer_official_and_do_not_double_wrap_proxy() {
        let sources = UpdateSources {
            manifest_endpoints: vec![],
            download_bases: vec!["https://cdn.example/releases/".into()],
        };
        let proxies = vec!["https://proxy.example/".into()];
        let original = "https://proxy.example/https://github.com/Gczmy/SoloSoul/releases/download/v2.13.0/app.tar.gz";
        let urls = artifact_candidates(original, "2.13.0", &sources, &proxies).unwrap();
        assert_eq!(
            urls,
            vec![
                "https://cdn.example/releases/v2.13.0/app.tar.gz",
                "https://github.com/Gczmy/SoloSoul/releases/download/v2.13.0/app.tar.gz",
                original
            ]
        );
        let direct = artifact_candidates(original, "2.13.0", &sources, &[]).unwrap();
        assert_eq!(direct.len(), 2);
    }

    #[test]
    fn version_and_filename_are_confined_to_path_segments() {
        let sources = UpdateSources::default();
        assert!(
            artifact_candidates("https://cdn.example/app.apk", "../../latest", &sources, &[])
                .is_err()
        );
        assert!(
            artifact_candidates("https://cdn.example/a%2Fb.apk", "2.13.0", &sources, &[]).is_err()
        );
        let urls = artifact_candidates("https://cdn.example/my%20app.apk", "2.13.0", &sources, &[])
            .unwrap();
        assert!(urls[1].ends_with("/v2.13.0/my%20app.apk"));
        assert!(!urls[1].contains("%2520"));
    }
}

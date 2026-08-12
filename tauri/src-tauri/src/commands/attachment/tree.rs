//! 附件树：全局附件列表的页面/对象分组与构建（P047 拆分）。

use super::*;

// ── Types for global attachment tree ────────────────────────────

/// One object in the attachment tree, containing its attachments.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentTreeObject {
    pub object_id: String,
    pub object_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<AttachmentMeta>,
}

/// One page (section type or custom page) in the attachment tree.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentTreePage {
    #[serde(default)]
    pub page_id: Option<String>,
    pub page_name: String,
    #[serde(default)]
    pub page_icon: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<AttachmentTreeObject>,
}

/// Result of listing all attachments across all objects, grouped by page.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentListAllResult {
    /// Pages with active (non-deleted) attachments.
    pub pages: Vec<AttachmentTreePage>,
    /// Pages with deleted attachments (for trash view).
    pub trash_pages: Vec<AttachmentTreePage>,
}

/// List all attachments across all objects, grouped by page.
/// Custom pages use parent_id to find child objects;
/// remaining objects are grouped by section_type (built-in sections).
#[tauri::command]
pub async fn attachment_list_all(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<AttachmentListAllResult, String> {
    let vault = vault_handle(&state)?;
    // P112: 单次 list_objects 已解密全部 properties 并返回 summary.properties，
    // 下方直接复用，不再 load_objects_batch / 逐页 list_objects 重复解密。
    // P114: 全表 AES 解密 + 附件树构建移入 spawn_blocking，避免阻塞 tokio worker。
    tokio::task::spawn_blocking(move || {
        let objects = vault.list_objects(&account_id, None, None, None, false, false)?;

        // Separate page objects from other objects
        let (page_objects, section_groups, children_by_parent) =
            group_objects_for_attachment_tree(&objects);

        let pages = build_attachment_tree_pages(
            &vault,
            &page_objects,
            &section_groups,
            &children_by_parent,
            false,
        )?;
        let trash_pages = build_attachment_tree_pages(
            &vault,
            &page_objects,
            &section_groups,
            &children_by_parent,
            true,
        )?;

        Ok(AttachmentListAllResult { pages, trash_pages })
    })
    .await
    .map_err(|e| format!("attachment_list_all task failed: {e}"))?
}

/// P112: 附件树分组结果——页面对象、按 section_type 分组的内置区段对象、
/// 按 parent_id 分组的子对象（单次 list_objects 解密后一次成型，替代每页面 N+1 次查询）。
pub(crate) type AttachmentTreeGroups = (
    Vec<solosoul_vault::ObjectSummary>,
    std::collections::BTreeMap<String, Vec<solosoul_vault::ObjectSummary>>,
    HashMap<String, Vec<solosoul_vault::ObjectSummary>>,
);

/// P112: 单次 list_objects 已解密全部 properties，这里按 parent_id 一次性预分组子对象
/// （替代每页面 N+1 次解密查询），并分离页面对象与按 section_type 分组的内置区段对象。
pub(crate) fn group_objects_for_attachment_tree(
    objects: &[solosoul_vault::ObjectSummary],
) -> AttachmentTreeGroups {
    let mut page_objects: Vec<solosoul_vault::ObjectSummary> = Vec::new();
    let mut section_groups: std::collections::BTreeMap<String, Vec<solosoul_vault::ObjectSummary>> =
        std::collections::BTreeMap::new();
    let mut children_by_parent: HashMap<String, Vec<solosoul_vault::ObjectSummary>> =
        HashMap::new();

    for obj in objects {
        if obj.collection_type == "page" {
            page_objects.push(obj.clone());
        } else {
            section_groups
                .entry(obj.section_type.clone())
                .or_default()
                .push(obj.clone());
        }
        if let Some(pid) = &obj.parent_id {
            children_by_parent
                .entry(pid.clone())
                .or_default()
                .push(obj.clone());
        }
    }

    (page_objects, section_groups, children_by_parent)
}

/// Build attachment tree pages for a given filter (active vs trash).
/// P112: 直接复用已解密的 `summary.properties` 解析附件（不再 load_objects_batch 重复解密）；
/// 子对象由调用方按 parent_id 一次性预分组传入（不再每页面 N+1 次解密查询）。
pub(crate) fn build_attachment_tree_pages(
    vault: &solosoul_vault::VaultStore,
    page_objects: &[solosoul_vault::ObjectSummary],
    section_groups: &std::collections::BTreeMap<String, Vec<solosoul_vault::ObjectSummary>>,
    children_by_parent: &HashMap<String, Vec<solosoul_vault::ObjectSummary>>,
    only_deleted: bool,
) -> Result<Vec<AttachmentTreePage>, String> {
    let template_cache: std::cell::RefCell<std::collections::HashMap<String, Option<String>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    let build_objects_with_attachments = |objs: &[solosoul_vault::ObjectSummary],
                                          only_del: bool|
     -> Vec<AttachmentTreeObject> {
        objs.iter()
            .filter_map(|summary| {
                let all_atts = load_attachments(&summary.properties);
                let filtered: Vec<AttachmentMeta> = all_atts
                    .into_iter()
                    .filter(|a| {
                        if only_del {
                            a.deleted_at.is_some()
                        } else {
                            a.deleted_at.is_none()
                        }
                    })
                    .collect();
                if filtered.is_empty() {
                    None
                } else {
                    let template_name = summary.template_id.as_ref().and_then(|tid| {
                        let mut cache = template_cache.borrow_mut();
                        cache.get(tid).cloned().unwrap_or_else(|| {
                            let name = vault.load_user_template(tid).ok().flatten().map(|t| t.name);
                            cache.insert(tid.clone(), name.clone());
                            name
                        })
                    });
                    Some(AttachmentTreeObject {
                        object_id: summary.id.clone(),
                        object_name: summary.name.clone(),
                        template_name,
                        attachments: filtered,
                    })
                }
            })
            .collect()
    };

    let mut pages: Vec<AttachmentTreePage> = Vec::new();
    let mut child_ids_assigned: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    // For custom pages: find children via pre-grouped parent map
    for page_obj in page_objects {
        let children = children_by_parent
            .get(&page_obj.id)
            .cloned()
            .unwrap_or_default();
        for child in &children {
            child_ids_assigned.insert(child.id.clone());
        }
        let objects_with_attachments = build_objects_with_attachments(&children, only_deleted);
        if !objects_with_attachments.is_empty() {
            pages.push(AttachmentTreePage {
                page_id: Some(page_obj.id.clone()),
                page_name: page_obj.name.clone(),
                page_icon: Some(page_obj.icon_name.clone()),
                objects: objects_with_attachments,
            });
        }
    }

    // For remaining objects: group by section_type (built-in sections)
    for (section, objs) in section_groups {
        let unassigned: Vec<_> = objs
            .iter()
            .filter(|o| !child_ids_assigned.contains(&o.id))
            .cloned()
            .collect();
        if unassigned.is_empty() {
            continue;
        }
        let objects_with_attachments = build_objects_with_attachments(&unassigned, only_deleted);
        if !objects_with_attachments.is_empty() {
            pages.push(AttachmentTreePage {
                page_id: None,
                page_name: section.clone(),
                page_icon: Some(section.clone()),
                objects: objects_with_attachments,
            });
        }
    }

    Ok(pages)
}

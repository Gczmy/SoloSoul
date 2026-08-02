/** 附件树相关类型（P024 拆分自 GlobalAttachmentManager）。 */

/** 附件元数据（与后端 attachment_list_all 返回结构一致）。 */
export interface AttachmentMeta {
  id: string;
  objectId: string;
  fileName: string;
  mimeType: string;
  sizeBytes: number;
  createdAt: string;
  deletedAt?: string | null;
  srcPath?: string | null;
  vaultPath?: string | null;
}

export interface AttachmentTreeObject {
  objectId: string;
  objectName: string;
  templateName?: string | null;
  attachments: AttachmentMeta[];
}

export interface AttachmentTreePage {
  pageId?: string | null;
  pageName: string;
  pageIcon?: string | null;
  objects: AttachmentTreeObject[];
}

export interface AttachmentListAllResult {
  pages: AttachmentTreePage[];
  trashPages: AttachmentTreePage[];
}

/** 需要永久删除的附件（携带来源对象 ID）。 */
export type AttachmentToPurge = AttachmentMeta & { _objectId: string };

import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { X } from 'lucide-react';
import { Dialog } from '@/components/ui/Dialog';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { useUiStore } from '@/stores/uiStore';

/** 保存成功后回传的最新元数据（父级据此更新本地列表 / 照片集）。 */
export interface AttachmentMetaEditResult {
  description?: string | null;
  tags?: string[];
}

interface AttachmentMetaEditDialogProps {
  /** 目标附件（仅使用 objectId / id / description / tags）。 */
  item: {
    objectId: string;
    id: string;
    fileName?: string;
    description?: string | null;
    tags?: string[];
  };
  onClose: () => void;
  onSaved: (updated: AttachmentMetaEditResult) => void;
}

/** 单个附件最多标签数（与后端 MAX_ATTACHMENT_TAGS 一致）。 */
const MAX_TAGS = 20;

/** 单个标签最大字符数（与后端 MAX_ATTACHMENT_TAG_LEN 一致）。 */
const MAX_TAG_LEN = 30;

/**
 * 把输入框原始文本合并进标签数组（幂等）：trim → 截断 → 空值/重复/超上限均不新增。
 *
 * 供「回车 / 失焦 / 保存」三条路径共用——尤其保存路径必须基于最新 tags 合并
 * 当前 tagInput，避免 blur 的 setTags 尚未提交时读取到过期闭包导致标签丢失。
 */
function mergeTagInput(base: string[], raw: string): string[] {
  const val = raw.trim().slice(0, MAX_TAG_LEN);
  if (!val) return base;
  if (base.length >= MAX_TAGS) return base;
  const exists = base.some((x) => x.toLowerCase() === val.toLowerCase());
  return exists ? base : [...base, val];
}

/**
 * 附件「描述 + 标签」编辑对话框。
 *
 * - 描述：多行文本域，空串保存时清除；
 * - 标签：chips 输入（回车 / 逗号 / 失焦 / 保存时自动生成，X 移除，去空去重，最多 20 个）；
 * - 保存调用 `attachment_update_meta`，成功后回调 onSaved 供父级同步本地状态。
 */
export function AttachmentMetaEditDialog({ item, onClose, onSaved }: AttachmentMetaEditDialogProps) {
  const { t } = useTranslation('common');
  const showToast = useUiStore((s) => s.showToast);

  const [description, setDescription] = useState(item.description ?? '');
  const [tags, setTags] = useState<string[]>(item.tags ?? []);
  const [tagInput, setTagInput] = useState('');
  const [saving, setSaving] = useState(false);

  // item 切换（如相册翻页后再次打开）时重置表单
  useEffect(() => {
    setDescription(item.description ?? '');
    setTags(item.tags ?? []);
    setTagInput('');
  }, [item.id]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleTagKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter' || e.key === ',') {
      e.preventDefault();
      setTags((prev) => mergeTagInput(prev, tagInput));
      setTagInput('');
    } else if (e.key === 'Backspace' && !tagInput && tags.length > 0) {
      setTags((prev) => prev.slice(0, -1));
    }
  };

  /** 失焦（点击对话框外部/其他区域）时，输入框有内容则直接生成标签。 */
  const handleTagBlur = () => {
    if (tagInput.trim()) {
      setTags((prev) => mergeTagInput(prev, tagInput));
      setTagInput('');
    }
  };

  const removeTag = (tag: string) => {
    setTags((prev) => prev.filter((x) => x !== tag));
  };

  const handleSave = async () => {
    if (saving) return;
    setSaving(true);
    try {
      const trimmedDesc = description.trim();
      // 保存时输入框仍有未回车内容 → 一并生成标签（基于最新 tags 合并，幂等）
      const finalTags = mergeTagInput(tags, tagInput);
      const updated: AttachmentMetaEditResult = {
        description: trimmedDesc ? trimmedDesc : null,
        tags: finalTags,
      };
      await invoke('attachment_update_meta', {
        objectId: item.objectId,
        attachmentId: item.id,
        description: updated.description,
        tags: updated.tags,
      });
      onSaved(updated);
      showToast({ type: 'success', message: t('common:meta_saved', { defaultValue: 'Saved' }) });
      onClose();
    } catch (e) {
      showToast({
        type: 'error',
        message: `${t('common:save_failed', { defaultValue: 'Save failed' })}: ${e}`,
      });
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog
      isOpen
      onClose={onClose}
      title={t('common:edit_meta', { defaultValue: 'Description & Tags' })}
      // 附件查看器在详情模态下可达 z-index 5100（ObjectDetailModal 传入），
      // 默认层级（--z-modal 4000）会被其背景遮住导致「点击无反应」；
      // 与附件确认对话框同用 auth 层级（8000），保证恒在查看器/预览/相册之上。
      priority="auth"
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
        {/* 描述 */}
        <div>
          <label
            style={{
              display: 'block',
              fontSize: 'var(--text-caption)',
              color: 'var(--text-secondary)',
              marginBottom: 6,
              fontWeight: 500,
            }}
          >
            {t('common:attachment_description', { defaultValue: 'Description' })}
          </label>
          <textarea
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder={t('common:attachment_description_placeholder', {
              defaultValue: 'Add a description…',
            })}
            rows={5}
            maxLength={500}
            style={{
              width: '100%',
              resize: 'vertical',
              padding: '8px 10px',
              borderRadius: 8,
              border: '1px solid var(--border-subtle)',
              background: 'var(--bg-elevated)',
              color: 'var(--text-primary)',
              fontSize: 'var(--text-body-sm)',
              outline: 'none',
              fontFamily: 'inherit',
              boxSizing: 'border-box',
            }}
          />
        </div>

        {/* 标签 */}
        <div>
          <label
            style={{
              display: 'block',
              fontSize: 'var(--text-caption)',
              color: 'var(--text-secondary)',
              marginBottom: 6,
              fontWeight: 500,
            }}
          >
            {t('common:attachment_tags', { defaultValue: 'Tags' })}
          </label>
          <div
            style={{
              display: 'flex',
              flexWrap: 'wrap',
              gap: 6,
              padding: '8px 10px',
              borderRadius: 8,
              border: '1px solid var(--border-subtle)',
              background: 'var(--bg-elevated)',
              minHeight: 40,
              boxSizing: 'border-box',
              alignItems: 'center',
            }}
          >
            {tags.map((tag) => (
              <span
                key={tag}
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: 4,
                  padding: '1px 8px',
                  borderRadius: 6,
                  background: 'var(--accent-subtle, rgba(94,129,244,0.12))',
                  border: '1px solid var(--accent-primary)',
                  color: 'var(--accent-primary)',
                  fontSize: 'var(--text-badge)',
                  fontWeight: 500,
                  lineHeight: 1,
                  whiteSpace: 'nowrap',
                }}
              >
                {tag}
                <button
                  type="button"
                  onClick={() => removeTag(tag)}
                  aria-label={t('common:remove_tag', { defaultValue: 'Remove tag' })}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    width: 16,
                    height: 16,
                    // 覆盖 global.css 移动端 button 触控基线（min-height/min-width: 44px）——
                    // 否则安卓端 X 移除按钮被撑成 44×44，chip 高度随之暴增（T003 同源）。
                    minWidth: 0,
                    minHeight: 0,
                    borderRadius: '50%',
                    border: 'none',
                    background: 'transparent',
                    color: 'inherit',
                    cursor: 'pointer',
                    padding: 0,
                    touchAction: 'manipulation',
                  }}
                >
                  <X size={12} />
                </button>
              </span>
            ))}
            <Input
              value={tagInput}
              onChange={(e) => setTagInput(e.target.value.slice(0, MAX_TAG_LEN))}
              onKeyDown={handleTagKeyDown}
              onBlur={handleTagBlur}
              placeholder={t('common:tag_input_placeholder', {
                defaultValue: 'Type a tag and press Enter',
              })}
              style={{ flex: 1, minWidth: 140 }}
              aria-label={t('common:attachment_tags', { defaultValue: 'Tags' })}
            />
          </div>
          <div
            style={{
              marginTop: 4,
              fontSize: 'var(--text-caption)',
              color: 'var(--text-tertiary)',
            }}
          >
            {tags.length}/{MAX_TAGS}
          </div>
        </div>

        {/* 操作 */}
        <div
          style={{
            display: 'flex',
            justifyContent: 'flex-end',
            gap: 8,
            marginTop: 4,
          }}
        >
          <Button variant="tertiary" size="sm" onClick={onClose} disabled={saving}>
            {t('common:cancel')}
          </Button>
          <Button variant="primary" size="sm" onClick={handleSave} loading={saving}>
            {t('common:save')}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}

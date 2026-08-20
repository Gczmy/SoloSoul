import type { CSSProperties, RefObject } from 'react';
import type { TFunction } from 'i18next';
import type { CustomIconId } from '@/lib/pageIcons';
import { IconCategoryPicker } from './IconCategoryPicker';
import styles from './SideNavigation.module.css';

export interface AddPagePopoverProps {
  style: CSSProperties;
  popoverRef: RefObject<HTMLDivElement | null>;
  inputRef: RefObject<HTMLInputElement | null>;
  name: string;
  onNameChange: (v: string) => void;
  description: string;
  onDescriptionChange: (v: string) => void;
  nameError: 'empty' | 'duplicate' | null;
  selectedIconId: CustomIconId;
  onSelectIcon: (id: CustomIconId) => void;
  /** 确认创建（isExplicit=true 时空名称展示错误而非静默取消）。 */
  onConfirm: (isExplicit?: boolean) => void;
  onCancel: () => void;
  showDescription: boolean;
  scrollMaxHeight?: number;
  isBottom: boolean;
  isSmallWindow: boolean;
  t: TFunction;
}

/**
 * AddPageButton 的「新建页面」弹层（portal 至 body 的创建表单）：
 * 名称/描述输入、错误行、图标选择、确认/取消。定位样式由父组件计算传入。
 */
export function AddPagePopover({
  style,
  popoverRef,
  inputRef,
  name,
  onNameChange,
  description,
  onDescriptionChange,
  nameError,
  selectedIconId,
  onSelectIcon,
  onConfirm,
  onCancel,
  showDescription,
  scrollMaxHeight,
  isBottom,
  isSmallWindow,
  t,
}: AddPagePopoverProps) {
  return (
    <div ref={popoverRef} className={styles.addPagePopover} style={style}>
      {/* Name input */}
      <input
        ref={inputRef}
        value={name}
        onChange={(e) => {
          onNameChange(e.target.value.slice(0, 20));
        }}
        onBlur={(e) => {
          // Only confirm if the blur is not caused by clicking inside the popover
          if (popoverRef.current && !popoverRef.current.contains(e.relatedTarget as Node)) {
            onConfirm(false);
          }
        }}
        onKeyDown={(e) => {
          if (e.key === 'Enter') onConfirm(true);
          if (e.key === 'Escape') onCancel();
        }}
        placeholder={t('add_page_placeholder')}
        maxLength={20}
        autoFocus
        aria-label={t('add_page_placeholder')}
        className={styles.addPageInput}
        data-error={nameError ? 'true' : undefined}
        style={{ flexShrink: 0 }}
      />
      {showDescription && (
        <input
          value={description}
          onChange={(e) => onDescriptionChange(e.target.value.slice(0, 30))}
          onBlur={(e) => {
            if (popoverRef.current && !popoverRef.current.contains(e.relatedTarget as Node)) {
              onConfirm(false);
            }
          }}
          onKeyDown={(e) => {
            if (e.key === 'Enter') onConfirm(true);
            if (e.key === 'Escape') onCancel();
          }}
          placeholder={t('add_page_description_placeholder')}
          maxLength={30}
          aria-label={t('add_page_description_placeholder')}
          className={styles.addPageInput}
          data-secondary
          style={{ flexShrink: 0 }}
        />
      )}
      {nameError && (
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            gap: 8,
            flexShrink: 0,
          }}
        >
          <span
            style={{
              fontSize: 'var(--text-badge)',
              color: '#e74c3c',
              whiteSpace: 'nowrap',
            }}
          >
            {nameError === 'empty' ? t('page_name_required') : t('page_name_exists')}
          </span>
          <button onClick={onCancel} className={styles.cancelTextBtn}>
            {t('common:cancel')}
          </button>
        </div>
      )}

      {/* Icon picker with category sections (scrollable) */}
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: 6,
          ...(isBottom && {
            flex: '1 1 auto',
            minHeight: isSmallWindow ? 80 : 120,
            overflow: 'hidden',
          }),
        }}
      >
        <span style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)' }}>
          {t('select_icon')}
        </span>
        <div
          style={{
            maxHeight: isBottom ? undefined : scrollMaxHeight,
            overflowY: 'auto',
            overflowX: 'hidden',
            display: 'flex',
            flexDirection: 'column',
            gap: 10,
            ...(isBottom && { flex: '1 1 auto', minHeight: 0 }),
          }}
        >
          <IconCategoryPicker selectedIconId={selectedIconId} onSelect={onSelectIcon} />
        </div>
      </div>

      {/* Cancel / Confirm buttons at bottom */}
      <div
        style={{
          display: 'flex',
          gap: 8,
          justifyContent: 'flex-end',
          paddingTop: 4,
          borderTop: '1px solid var(--border-subtle)',
          flexShrink: 0,
          marginTop: isBottom ? 'auto' : undefined,
        }}
      >
        <button
          onClick={onCancel}
          className={styles.cancelTextBtn}
          style={{
            padding: '6px 12px',
            borderRadius: 6,
            fontSize: 'var(--text-body-sm)',
            background: 'transparent',
            border: '1px solid var(--border-subtle)',
            color: 'var(--text-secondary)',
            cursor: 'pointer',
          }}
        >
          {t('common:cancel')}
        </button>{' '}
        <button
          onClick={() => onConfirm(true)}
          style={{
            padding: '6px 12px',
            borderRadius: 6,
            fontSize: 'var(--text-body-sm)',
            background: 'var(--accent-primary)',
            border: 'none',
            color: '#fff',
            cursor: 'pointer',
            fontWeight: 500,
          }}
        >
          {t('common:confirm')}
        </button>
      </div>
    </div>
  );
}

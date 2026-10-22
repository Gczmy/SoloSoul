import { useEffect, useId, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { ChevronDown } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { ToastOutlet } from '@/components/ui/ToastOutlet';
import { useOverlayBackGuard } from '@/hooks/useOverlayBackGuard';
import { useSettingsStore } from '@/stores/settingsStore';
import { deriveContractBindings } from '@/lib/plugin';
import { resolveCustomIcon } from '@/lib/pageIcons';
import { useTemplateEditorState } from '../useTemplateEditorState';
import { AndroidTemplateFields, type OpenTemplatePanel } from './AndroidTemplateFields';
import { AndroidTemplateSheet } from './AndroidTemplateSheet';
import { SYSTEM_PAGES, type TemplateEditorState, type TemplatePanel } from './types';
import styles from './AndroidTemplateEditor.module.css';

/** 安卓独立展示层：复用编辑草稿与保存/归档逻辑，桌面仍使用原 TemplateEditor。 */
export function AndroidTemplateEditor({ editor }: { editor: TemplateEditorState }) {
  const { t } = useTranslation(['settings', 'common', 'editor', 'navigation']);
  const titleId = useId();
  const screen = useRef<HTMLDivElement>(null);
  const body = useRef<HTMLDivElement>(null);
  const pendingFocus = useRef<string | null>(null);
  const trigger = useRef<HTMLElement | null>(null);
  const [panel, setPanel] = useState<TemplatePanel | null>(null);
  const customPages = useSettingsStore((state) => state.settings.customPages);
  const state = useTemplateEditorState(
    editor.editProperties,
    editor.editContractTypeId,
    editor.updatePropertyContractBindings,
  );
  const closeRef = useRef(editor.closeEdit);
  closeRef.current = editor.closeEdit;
  const confirmRef = useRef(editor.confirmOpen);
  confirmRef.current = editor.confirmOpen;
  useOverlayBackGuard({
    innerOpen: editor.confirmOpen,
    onCloseInner: editor.closeConfirm,
    onClose: editor.closeEdit,
  });

  useEffect(() => {
    const root = document.getElementById('root');
    const previousInert = root?.inert ?? false;
    const previousFocus = document.activeElement;
    const previousOverflow = document.body.style.overflow;
    if (root) root.inert = true;
    document.body.style.overflow = 'hidden';
    screen.current?.focus({ preventScroll: true });
    const keydown = (event: KeyboardEvent) => {
      const host = screen.current;
      // 子面板由 AndroidSheet 的捕获监听处理；确认弹窗由自身处理。
      if (
        !host ||
        event.defaultPrevented ||
        confirmRef.current ||
        !host.contains(event.target as Node)
      )
        return;
      if (event.key === 'Escape') {
        event.preventDefault();
        closeRef.current();
      } else if (event.key === 'Tab') {
        const nodes = [
          ...host.querySelectorAll<HTMLElement>(
            'button:not(:disabled), input:not(:disabled), textarea, [tabindex="0"]',
          ),
        ].filter((node) => node.getClientRects().length > 0);
        const first = nodes[0];
        const last = nodes[nodes.length - 1];
        if (
          event.shiftKey &&
          (document.activeElement === first || document.activeElement === host)
        ) {
          event.preventDefault();
          last?.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first?.focus();
        }
      }
    };
    document.addEventListener('keydown', keydown);
    return () => {
      document.removeEventListener('keydown', keydown);
      document.body.style.overflow = previousOverflow;
      if (root) root.inert = previousInert;
      if (previousFocus instanceof HTMLElement && previousFocus.isConnected)
        previousFocus.focus({ preventScroll: true });
    };
  }, []);

  useEffect(() => {
    const id = pendingFocus.current;
    if (!id || panel) return;
    const input = [
      ...(body.current?.querySelectorAll<HTMLInputElement>('[data-template-field-name]') ?? []),
    ].find((node) => node.dataset.templateFieldName === id);
    if (input) {
      pendingFocus.current = null;
      input.focus({ preventScroll: true });
      input.scrollIntoView({ block: 'nearest' });
      input.select();
    }
  }, [editor.editProperties, panel]);

  const openPanel: OpenTemplatePanel = (next, element) => {
    trigger.current = element;
    if (next.kind === 'binding') {
      const idx = editor.editProperties.findIndex((prop) => prop.id === next.fieldId);
      const prop = editor.editProperties[idx];
      if (prop?.contractField && !prop.contractBindings?.length && editor.editContractTypeId) {
        const bindings = deriveContractBindings(
          editor.editContractTypeId,
          prop.id,
          state.installedPlugins,
        );
        if (bindings.length) editor.updatePropertyContractBindings(idx, bindings);
      }
    }
    setPanel(next);
  };
  const Icon = resolveCustomIcon(editor.editIconId);
  const pageLabel = SYSTEM_PAGES.some((id) => id === editor.editCategory)
    ? t(`navigation:${editor.editCategory}`)
    : (customPages.find((page) => page.id === editor.editCategory)?.name ??
      t('settings:deleted_page'));

  return createPortal(
    <>
      <div
        ref={screen}
        className={styles.screen}
        data-android-template-editor
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        onClick={(event) => event.stopPropagation()}
        inert={panel !== null}
        aria-hidden={panel ? true : undefined}
      >
        <header className={styles.header}>
          <Button variant="tertiary" onClick={editor.closeEdit}>
            {t('common:cancel')}
          </Button>
          <h2 id={titleId}>
            {t(editor.isNewTemplate ? 'settings:new_template' : 'settings:edit_template')}
          </h2>
          <Button onClick={() => void editor.saveEdit()}>{t('common:save')}</Button>
        </header>
        <ToastOutlet priority={4000} />
        <div ref={body} className={styles.body} data-template-scroll>
          <div className={styles.form}>
            <Input
              label={t('common:name')}
              value={editor.editName}
              error={editor.showNameError ? t('common:name_required') : undefined}
              onChange={(event) => editor.setEditName(event.target.value)}
            />
            <button
              type="button"
              className={styles.choice}
              onClick={(event) => openPanel({ kind: 'page' }, event.currentTarget)}
            >
              <span>
                <small>{t('settings:template_category')}</small>
                {pageLabel}
              </span>
              <ChevronDown size={20} />
            </button>
            <button
              type="button"
              className={styles.choice}
              onClick={(event) => openPanel({ kind: 'icon' }, event.currentTarget)}
            >
              <Icon size={24} />
              <span>{t('settings:template_icon')}</span>
              <ChevronDown size={20} />
            </button>
            {editor.editContractTypeId && (
              <p className={styles.contractId}>
                {t('settings:contract_type_id')}: {editor.editContractTypeId}
              </p>
            )}
            <AndroidTemplateFields editor={editor} openPanel={openPanel} />
            <section className={styles.section}>
              <label className={styles.checkRow}>
                <SelectCheckbox
                  checked={editor.dynamicGroupEnabled}
                  onChange={editor.handleDynamicGroupEnabledChange}
                />
                <span>{t('editor:enable_dynamic_group')}</span>
              </label>
              {editor.dynamicGroupEnabled && (
                <Button
                  variant="secondary"
                  onClick={(event) => openPanel({ kind: 'dynamic' }, event.currentTarget)}
                >
                  {t('settings:template_mobile.dynamic_settings')}
                </Button>
              )}
            </section>
          </div>
        </div>
      </div>
      {panel && (
        <AndroidTemplateSheet
          panel={panel}
          setPanel={setPanel}
          editor={editor}
          state={state}
          trigger={trigger.current}
          onAdd={(type) => {
            pendingFocus.current = editor.addPropertyOfType(type);
            setPanel(null);
          }}
        />
      )}
    </>,
    document.body,
  );
}

import { useEffect, useMemo, useState } from 'react';
import { ChevronRight, FileText, Search } from 'lucide-react';
import { useMatch, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useAuthStore } from '@/stores/authStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useUiStore } from '@/stores/uiStore';
import { observeDesktopPageIndex, useDesktopPageIndexStore } from '@/stores/desktopPageIndexStore';
import { PAGE_ICON_MAP, resolveCustomIcon } from '@/lib/pageIcons';
import { SYSTEM_PAGE_KEYS } from './useNavigationItems';
import { AddPageButton } from './AddPageButton';
import { PagePaneRow } from './PagePaneRow';
import { ObjectEditLockBadge } from '@/components/object/ObjectEditLockBadge';
import { PagePaneObjectPreview, PagePanePagePreview } from './PagePanePreview';
import styles from './DesktopPagePane.module.css';

export function DesktopPagePane({
  open,
  animating = false,
}: {
  open: boolean;
  animating?: boolean;
}) {
  const { t } = useTranslation('navigation');
  const navigate = useNavigate();
  const editorMatch = useMatch('/editor/:objectId');
  const accountId = useAuthStore((state) => state.currentAccount?.id);
  const authenticated = useAuthStore((state) => state.isAuthenticated);
  const objects = useDesktopPageIndexStore((state) => state.objects);
  const loading = useDesktopPageIndexStore((state) => state.loading);
  const failed = useDesktopPageIndexStore((state) => state.failed);
  const customPages = useSettingsStore((state) => state.settings.customPages);
  const creationRequest = useUiStore((state) => state.desktopPageCreationRequest);
  const [query, setQuery] = useState('');
  const [activePreview, setActivePreview] = useState<string | null>(null);
  const [searchCollapsed, setSearchCollapsed] = useState(() => new Set<string>());
  const [expanded, setExpanded] = useState(() => new Set<string>(['identity']));
  const selectedId = editorMatch?.params.objectId;
  const openObject = (objectId: string) => {
    setActivePreview(null);
    // 重选当前对象不新增导航记录，也不重置编辑器及右侧面板的滚动和草稿。
    if (selectedId !== objectId) navigate(`/editor/${encodeURIComponent(objectId)}`);
  };
  useEffect(() => setActivePreview(null), [accountId, authenticated]);

  useEffect(() => {
    if (!accountId || !authenticated) return;
    return observeDesktopPageIndex();
  }, [accountId, authenticated]);

  const pages = useMemo(
    () => [
      ...SYSTEM_PAGE_KEYS.map((id) => ({ id, name: t(id), Icon: PAGE_ICON_MAP[id] })),
      ...customPages
        .filter((page) => !page.deletedAt)
        .map((page) => ({
          id: page.id,
          name: page.name,
          Icon: resolveCustomIcon(page.iconId),
          description: page.description,
        })),
    ],
    [customPages, t],
  );
  const grouped = useMemo(() => {
    const groups = new Map<string, typeof objects>();
    for (const object of objects) {
      const group = groups.get(object.sectionId) ?? [];
      group.push(object);
      groups.set(object.sectionId, group);
    }
    return groups;
  }, [objects]);
  const selectedSection = objects.find((object) => object.id === selectedId)?.sectionId;
  useEffect(() => {
    if (selectedSection) setExpanded((previous) => new Set([...previous, selectedSection]));
  }, [selectedSection]);
  const search = query.trim().toLocaleLowerCase();
  const visible = pages
    .map((page) => {
      const children = grouped.get(page.id) ?? [];
      return {
        ...page,
        count: children.length,
        children:
          !search || page.name.toLocaleLowerCase().includes(search)
            ? children
            : children.filter((object) => object.name.toLocaleLowerCase().includes(search)),
      };
    })
    .filter(
      (page) => !search || page.name.toLocaleLowerCase().includes(search) || page.children.length,
    );

  return (
    <section
      id="desktop-page-pane"
      className={styles.pane}
      aria-label={t('page_pane_title')}
      aria-hidden={!open}
      inert={!open}
      data-page-pane
    >
      <div className={styles.header}>
        <div className={styles.heading}>
          <strong>{t('page_pane_title')}</strong>
          <span>{objects.length}</span>
        </div>
        <label className={styles.search}>
          <Search size={16} aria-hidden="true" />
          <input
            type="search"
            value={query}
            onChange={(event) => {
              setQuery(event.target.value);
              setActivePreview(null);
              setSearchCollapsed(new Set());
            }}
            aria-label={t('page_pane_search')}
            placeholder={t('page_pane_search')}
          />
        </label>
        <AddPageButton
          enabled={open && !animating}
          openRequest={creationRequest}
          placement="below"
          className={styles.addPage}
          buttonClassName={styles.addPageButton}
          showLabel
          showDescription
          onCreate={(page) => {
            setQuery('');
            setExpanded((previous) => new Set([...previous, page.id]));
            navigate(`/workspace/custom/${encodeURIComponent(page.id)}`);
          }}
        />
      </div>
      <nav className={styles.scroller} aria-label={t('page_pane_title')} data-page-pane-scroll>
        {failed && (
          <div className={styles.message} role="status">
            {t('page_pane_load_failed')}
            <button
              type="button"
              onClick={() => void useDesktopPageIndexStore.getState().refresh()}
            >
              {t('page_pane_retry')}
            </button>
          </div>
        )}
        {loading && !objects.length && (
          <p className={styles.message} role="status">
            {t('page_pane_loading')}
          </p>
        )}
        <ul className={styles.list}>
          {visible.map((page) => {
            const { id, name, Icon, count, children } = page;
            const isExpanded = search ? !searchCollapsed.has(id) : expanded.has(id);
            const listId = `page-pane-children-${id}`;
            return (
              <li key={id}>
                <PagePaneRow
                  className={styles.pageRow}
                  name={name}
                  enabled={open && !animating}
                  aria-expanded={isExpanded}
                  aria-controls={listId}
                  data-page-id={id}
                  previewKey={`page:${id}`}
                  activePreview={activePreview}
                  onPreviewChange={setActivePreview}
                  previewLabel={t('page_pane_page_preview')}
                  preview={
                    <PagePanePagePreview
                      name={name}
                      description={'description' in page ? page.description : undefined}
                      count={count}
                      onNavigate={() => {
                        setActivePreview(null);
                        navigate(
                          SYSTEM_PAGE_KEYS.some((key) => key === id)
                            ? `/workspace?section=${encodeURIComponent(id)}`
                            : `/workspace/custom/${encodeURIComponent(id)}`,
                        );
                      }}
                    />
                  }
                  onClick={() => {
                    const update = (previous: Set<string>) => {
                      const next = new Set(previous);
                      if (next.has(id)) next.delete(id);
                      else next.add(id);
                      return next;
                    };
                    if (search) setSearchCollapsed(update);
                    else setExpanded(update);
                  }}
                  leading={
                    <>
                      <ChevronRight
                        size={14}
                        className={styles.chevron}
                        data-open={isExpanded}
                        aria-hidden="true"
                      />
                      <Icon size={17} aria-hidden="true" />
                    </>
                  }
                  trailing={
                    <span className={styles.count} aria-hidden="true">
                      {count}
                    </span>
                  }
                />
                <ul id={listId} className={styles.list} hidden={!isExpanded}>
                  {children.map((object, index) => (
                    <li key={object.id}>
                      <PagePaneRow
                        className={styles.objectRow}
                        name={object.name}
                        enabled={open && !animating && isExpanded}
                        aria-current={selectedId === object.id ? 'page' : undefined}
                        data-object-id={object.id}
                        previewKey={`object:${object.id}`}
                        activePreview={activePreview}
                        onPreviewChange={setActivePreview}
                        previewLabel={t('page_pane_object_preview')}
                        preview={
                          <PagePaneObjectPreview
                            objectId={object.id}
                            collectionLabel={name}
                            index={index}
                            total={children.length}
                            onNavigate={() => openObject(object.id)}
                          />
                        }
                        onClick={() => openObject(object.id)}
                        leading={<FileText size={15} aria-hidden="true" />}
                        trailing={
                          <ObjectEditLockBadge
                            objectId={object.id}
                            locked={object.editLocked}
                            compact
                          />
                        }
                      />
                    </li>
                  ))}
                  {!children.length && <li className={styles.empty}>{t('page_pane_empty')}</li>}
                </ul>
              </li>
            );
          })}
        </ul>
        {!visible.length && <p className={styles.message}>{t('page_pane_no_results')}</p>}
      </nav>
    </section>
  );
}

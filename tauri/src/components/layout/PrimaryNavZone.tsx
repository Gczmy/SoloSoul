import { useRef, useState, useCallback } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { RenameableNavButton } from './RenameableNavButton';
import { AddPageButton } from './AddPageButton';
import { NavButton } from './NavButton';
import { PAGE_ICON_MAP } from '@/lib/pageIcons';
import { useActiveCustomPages, primaryItems } from './useNavigationItems';
import styles from './SideNavigation.module.css';

import type { NavPosition } from './NavButton';
import type { CustomPage } from '@/stores/settingsStore';

interface PrimaryNavZoneProps {
  sidebarPosition: NavPosition;
  isHorizontal: boolean;
}

export function PrimaryNavZone({ sidebarPosition, isHorizontal }: PrimaryNavZoneProps) {
  const navigate = useNavigate();
  const location = useLocation();
  const { t } = useTranslation('navigation');
  const activeCustomPages = useActiveCustomPages();
  const horizontalNavRef = useRef<HTMLDivElement>(null);

  const [isDragging, setIsDragging] = useState(false);
  const dragStartX = useRef(0);
  const dragStartScroll = useRef(0);

  const handleHorizontalWheel = useCallback((e: React.WheelEvent<HTMLDivElement>) => {
    const el = horizontalNavRef.current;
    if (!el) return;
    const delta = e.deltaY + e.deltaX;
    if (delta === 0) return;
    e.preventDefault();
    el.scrollBy({ left: delta, behavior: 'auto' });
  }, []);

  const handlePointerDown = useCallback((e: React.PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0 || e.target !== e.currentTarget) return;
    setIsDragging(true);
    dragStartX.current = e.clientX;
    dragStartScroll.current = horizontalNavRef.current?.scrollLeft ?? 0;
    (e.currentTarget as HTMLDivElement).setPointerCapture?.(e.pointerId);
  }, []);

  const handlePointerMove = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      if (!isDragging || !horizontalNavRef.current) return;
      const dx = e.clientX - dragStartX.current;
      horizontalNavRef.current.scrollLeft = dragStartScroll.current - dx;
    },
    [isDragging],
  );

  const handlePointerUp = useCallback(() => setIsDragging(false), []);

  const isWorkspaceSectionActive = (sectionPath: string): boolean => {
    if (location.pathname.startsWith('/workspace/custom/')) return false;
    if (!location.pathname.startsWith('/workspace')) return false;
    const section = sectionPath.split('section=')[1];
    if (!section) return !location.search.includes('section=');
    return location.search.includes(`section=${section}`);
  };

  const isCustomPageActive = (pageId: string): boolean => {
    return location.pathname === `/workspace/custom/${pageId}`;
  };

  const handleCustomPageNavigate = (page: CustomPage) => {
    navigate(`/workspace/custom/${page.id}`);
  };

  const zoneStyle: React.CSSProperties = isHorizontal
    ? { flexDirection: 'row', width: 'auto', height: '100%', overflow: 'visible' }
    : { flexDirection: 'column', width: '100%', height: 'auto', overflow: 'visible' };

  return (
    <>
      {/* HOME — always visible */}
      <NavButton
        path={primaryItems[0].path}
        Icon={PAGE_ICON_MAP[primaryItems[0].iconKey]}
        label={t(primaryItems[0].labelKey)}
        isActive={location.pathname === '/'}
        onClick={() => navigate(primaryItems[0].path)}
        position={sidebarPosition}
      />

      {isHorizontal ? (
        <div className={styles.navPrimaryHorizontalWrapper} style={{ overflow: 'hidden', flex: 1 }}>
          <div
            ref={horizontalNavRef}
            onWheel={handleHorizontalWheel}
            onPointerDown={handlePointerDown}
            onPointerMove={handlePointerMove}
            onPointerUp={handlePointerUp}
            onPointerLeave={handlePointerUp}
            className={`${styles.navPrimary} ${styles.navPrimaryHorizontal}`}
            style={{
              display: 'flex',
              flexDirection: 'row',
              gap: 2,
              overflow: 'hidden',
              cursor: isDragging ? 'grabbing' : 'grab',
              userSelect: 'none',
            }}
          >
            {primaryItems.slice(1).map((item) => (
              <NavButton
                key={item.path}
                path={item.path}
                Icon={PAGE_ICON_MAP[item.iconKey]}
                label={t(item.labelKey)}
                isActive={isWorkspaceSectionActive(item.path)}
                onClick={() => navigate(item.path)}
                position={sidebarPosition}
              />
            ))}
            {activeCustomPages.map((page) => (
              <RenameableNavButton
                key={page.id}
                page={page}
                isActive={isCustomPageActive(page.id)}
                onClick={() => handleCustomPageNavigate(page)}
                position={sidebarPosition}
              />
            ))}
          </div>
        </div>
      ) : (
        <div
          className={styles.navPrimary}
          style={{ ...zoneStyle, flex: 1, overflowX: 'hidden', overflowY: 'auto' }}
        >
          {primaryItems.slice(1).map((item) => (
            <NavButton
              key={item.path}
              path={item.path}
              Icon={PAGE_ICON_MAP[item.iconKey]}
              label={t(item.labelKey)}
              isActive={isWorkspaceSectionActive(item.path)}
              onClick={() => navigate(item.path)}
              position={sidebarPosition}
            />
          ))}
          {activeCustomPages.map((page) => (
            <RenameableNavButton
              key={page.id}
              page={page}
              isActive={isCustomPageActive(page.id)}
              onClick={() => handleCustomPageNavigate(page)}
              position={sidebarPosition}
            />
          ))}
        </div>
      )}

      {/* AddPageButton — vertical mode only (horizontal has it in SecondaryActionBar) */}
      {!isHorizontal && (
        <AddPageButton
          onCreate={(page) => {
            navigate(`/workspace/custom/${page.id}`);
          }}
          position={sidebarPosition}
        />
      )}
    </>
  );
}

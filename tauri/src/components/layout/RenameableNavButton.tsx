import { useState, useRef } from 'react';
import { useTranslation } from 'react-i18next';

import type { CustomPage } from '@/stores/settingsStore';
import { NavButton } from './NavButton';
import { resolveCustomIcon } from '@/lib/pageIcons';
import { CustomPageEditPopover } from './CustomPageEditPopover';

// =============================================================================
// RenameableNavButton — custom page button with double-click rename
// =============================================================================

export function RenameableNavButton({
  page,
  isActive,
  onClick,
  position = 'left',
}: {
  page: CustomPage;
  isActive: boolean;
  onClick: () => void;
  position?: import('./NavButton').NavPosition;
}) {
  useTranslation(['navigation', 'common']);
  const [isEditing, setIsEditing] = useState(false);
  const [renameCardRect, setRenameCardRect] = useState<DOMRect | null>(null);
  const wrapperRef = useRef<HTMLDivElement>(null);

  const handleDoubleClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    setRenameCardRect(wrapperRef.current?.getBoundingClientRect() || null);
    setIsEditing(true);
  };

  const handleClose = () => {
    setIsEditing(false);
  };

  return (
    <div ref={wrapperRef} style={{ position: 'relative' }} onDoubleClick={handleDoubleClick}>
      <NavButton
        path={`/workspace/custom/${page.id}`}
        Icon={resolveCustomIcon(page.iconId)}
        label={page.name}
        isActive={isActive}
        onClick={onClick}
        position={position}
      />
      <CustomPageEditPopover
        page={page}
        isOpen={isEditing}
        onClose={handleClose}
        triggerRect={renameCardRect}
        position={position}
      />
    </div>
  );
}

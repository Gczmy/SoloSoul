import {
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
  type RefObject,
} from 'react';
import { useTranslation } from 'react-i18next';
import { supportsHover } from '@/lib/platform';
import type { ObjectSummary } from '@/stores/objectStore';
import type { UserTemplate } from '@/types/template';
import { ObjectRulerPreview } from './ObjectRulerPreview';
import { useObjectRulerPosition } from './useObjectRulerPosition';
import { useRulerRail } from './useRulerRail';
import { shellInset } from './objectRuler';
import styles from './WorkspaceObjectRuler.module.css';

interface WorkspaceObjectRulerProps {
  objects: ObjectSummary[];
  renderedCount: number;
  listRef: RefObject<HTMLDivElement | null>;
  revealObject: (index: number) => void;
  userTemplates: UserTemplate[];
  resolveCollectionLabel: (typeId: string) => string;
  attachmentCounts: Record<string, number>;
}

export function WorkspaceObjectRuler(props: WorkspaceObjectRulerProps) {
  const { objects, userTemplates, resolveCollectionLabel, attachmentCounts } = props;
  const { t } = useTranslation('common');
  const { activeId, navigateTo } = useObjectRulerPosition(props);
  const [hoveredId, setHoveredId] = useState<string | null>(null);
  const [previewTop, setPreviewTop] = useState<number | null>(null);
  const [focusIndex, setFocusIndex] = useState(0);
  const containerRef = useRef<HTMLElement>(null);
  const railRef = useRef<HTMLDivElement>(null);
  const previewRef = useRef<HTMLDivElement>(null);
  const closeTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const previewId = useId();
  const hintId = useId();
  const hoveredIndex = objects.findIndex((object) => object.id === hoveredId);
  const hovered = objects[hoveredIndex];
  const { step, scrollToIndex } = useRulerRail(containerRef, railRef, objects.length);

  const cancelClose = () => {
    if (closeTimer.current) clearTimeout(closeTimer.current);
    closeTimer.current = null;
  };
  const closePreview = () => {
    cancelClose();
    setHoveredId(null);
  };
  useEffect(
    () => () => {
      if (closeTimer.current) clearTimeout(closeTimer.current);
    },
    [],
  );
  useEffect(() => {
    setHoveredId(null);
    setFocusIndex(0);
  }, [objects]);

  useLayoutEffect(() => {
    if (!hovered) {
      setPreviewTop(null);
      return;
    }
    const update = () => {
      const button = railRef.current?.querySelector<HTMLButtonElement>(
        `[data-ruler-index="${hoveredIndex}"]`,
      );
      const preview = previewRef.current;
      if (!button || !preview) return;
      const anchor = button.getBoundingClientRect();
      const railBounds = railRef.current?.getBoundingClientRect();
      if (railBounds && (anchor.bottom < railBounds.top || anchor.top > railBounds.bottom)) {
        setHoveredId(null);
        return;
      }
      const top = Math.max(
        shellInset('top') + 8,
        Math.min(
          anchor.top + anchor.height / 2 - preview.offsetHeight / 2,
          window.innerHeight - shellInset('bottom') - preview.offsetHeight - 8,
        ),
      );
      setPreviewTop(top);
    };
    update();
    const observer = new ResizeObserver(update);
    if (previewRef.current) observer.observe(previewRef.current);
    const rail = railRef.current;
    if (rail) observer.observe(rail);
    rail?.addEventListener('scroll', update, { passive: true });
    window.addEventListener('resize', update);
    return () => {
      observer.disconnect();
      rail?.removeEventListener('scroll', update);
      window.removeEventListener('resize', update);
    };
  }, [hovered, hoveredIndex]);

  useEffect(() => {
    if (
      hoveredId ||
      containerRef.current?.matches(':hover') ||
      containerRef.current?.contains(document.activeElement)
    )
      return;
    const index = objects.findIndex((object) => object.id === activeId);
    if (index < 0) return;
    scrollToIndex(index);
    setFocusIndex(index);
  }, [activeId, hoveredId, objects, scrollToIndex]);

  const jump = (index: number, keyboard = false) => {
    closePreview();
    navigateTo(index, keyboard);
  };
  const onKeyDown = (event: KeyboardEvent<HTMLButtonElement>, index: number) => {
    let next: number;
    switch (event.key) {
      case 'ArrowDown':
        next = Math.min(objects.length - 1, index + 1);
        break;
      case 'ArrowUp':
        next = Math.max(0, index - 1);
        break;
      case 'Home':
        next = 0;
        break;
      case 'End':
        next = objects.length - 1;
        break;
      case 'Escape':
        closePreview();
        event.preventDefault();
        return;
      default:
        return;
    }
    event.preventDefault();
    setFocusIndex(next);
    const rail = railRef.current;
    const button = rail?.querySelector<HTMLButtonElement>(`[data-ruler-index="${next}"]`);
    scrollToIndex(next);
    button?.focus({ preventScroll: true });
  };

  if (objects.length < 2) return null;
  return (
    <nav
      ref={containerRef}
      className={styles.ruler}
      aria-label={t('object_ruler_label')}
      aria-describedby={hintId}
      onMouseEnter={cancelClose}
      onMouseLeave={() => {
        cancelClose();
        closeTimer.current = setTimeout(() => setHoveredId(null), 120);
      }}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node | null)) closePreview();
      }}
    >
      <span id={hintId} className={styles.srOnly}>
        {t('object_ruler_keyboard_hint')}
      </span>
      <div className={styles.railFrame}>
        <div
          ref={railRef}
          className={styles.rail}
          style={
            {
              '--ruler-step': `${step}px`,
            } as CSSProperties
          }
        >
          {objects.map((object, index) => {
            const distance = hoveredIndex < 0 ? Infinity : Math.abs(index - hoveredIndex);
            const width = distance === 0 ? 28 : distance === 1 ? 23 : distance === 2 ? 18 : 12;
            return (
              <button
                key={object.id}
                type="button"
                className={styles.tick}
                data-ruler-index={index}
                aria-label={t('object_ruler_go_to', {
                  name: object.name,
                  index: index + 1,
                  total: objects.length,
                })}
                aria-current={object.id === activeId ? 'location' : undefined}
                aria-describedby={hoveredId === object.id ? previewId : undefined}
                tabIndex={focusIndex === index ? 0 : -1}
                onMouseEnter={() => {
                  if (supportsHover()) {
                    cancelClose();
                    setHoveredId(object.id);
                  }
                }}
                onFocus={() => {
                  cancelClose();
                  setFocusIndex(index);
                  setHoveredId(object.id);
                }}
                onClick={(event) => jump(index, event.detail === 0)}
                onKeyDown={(event) => onKeyDown(event, index)}
                data-preview={hoveredId === object.id || undefined}
              >
                <span className={styles.tickLine} style={{ transform: `scaleX(${width / 28})` }} />
              </button>
            );
          })}
        </div>
      </div>
      {hovered && (
        <div
          data-macos-glass="panel"
          ref={previewRef}
          id={previewId}
          role="region"
          aria-label={t('object_ruler_preview')}
          className={styles.preview}
          style={{ top: previewTop ?? 0, visibility: previewTop === null ? 'hidden' : undefined }}
          onKeyDown={(event) => {
            if (event.key === 'Escape') closePreview();
          }}
        >
          <ObjectRulerPreview
            object={hovered}
            template={userTemplates.find((template) => template.id === hovered.templateId)}
            collectionLabel={resolveCollectionLabel(hovered.typeId)}
            index={hoveredIndex}
            total={objects.length}
            attachmentCount={attachmentCounts[hovered.id]}
            onNavigate={() => jump(hoveredIndex)}
          />
        </div>
      )}
    </nav>
  );
}

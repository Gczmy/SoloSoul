import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Image } from 'lucide-react';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { isMobilePlatformSync } from '@/lib/platform';
import { ICON_SIZE } from '@/lib/constants';
import type { AttachmentItem } from '@/lib/attachmentUtils';
import { loadThumbnailUrl } from '@/lib/photoAlbumPreview';

export interface PhotoAlbumGridProps {
  items: AttachmentItem[];
  onSelect: (item: AttachmentItem, index: number) => void;
}

interface ThumbCellProps {
  item: AttachmentItem;
  index: number;
  onSelect: (item: AttachmentItem, index: number) => void;
}

/**
 * 照片集网格单元：IntersectionObserver 懒加载缩略图（仅可视区附近加载，
 * 内存中只保留可视区附近的 data URL）；加载失败显示占位块。
 * 测试环境（jsdom 无 IntersectionObserver）回退为直接加载。
 */
function ThumbCell({ item, index, onSelect }: ThumbCellProps) {
  const { t } = useTranslation('common');
  const ref = useRef<HTMLDivElement>(null);
  const [url, setUrl] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const load = () => {
      loadThumbnailUrl(item)
        .then((u) => {
          if (!cancelled) setUrl(u);
        })
        .catch(() => {
          if (!cancelled) setFailed(true);
        });
    };

    const el = ref.current;
    if (el && typeof IntersectionObserver !== 'undefined') {
      const io = new IntersectionObserver(
        (entries) => {
          for (const entry of entries) {
            if (entry.isIntersecting) {
              io.disconnect();
              load();
            }
          }
        },
        { rootMargin: '240px' },
      );
      io.observe(el);
      return () => {
        cancelled = true;
        io.disconnect();
      };
    }
    load();
    return () => {
      cancelled = true;
    };
  }, [item]);

  return (
    <div
      ref={ref}
      role="button"
      tabIndex={0}
      title={item.fileName}
      aria-label={item.fileName}
      onClick={() => onSelect(item, index)}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          onSelect(item, index);
        }
      }}
      className="interactive-toolbar"
      style={{
        position: 'relative',
        aspectRatio: '1 / 1',
        borderRadius: 10,
        overflow: 'hidden',
        border: '1px solid var(--border-subtle)',
        cursor: 'pointer',
        background: 'var(--bg-toolbar)',
      }}
    >
      {url ? (
        <img
          src={url}
          alt={item.fileName}
          loading="lazy"
          draggable={false}
          style={{ width: '100%', height: '100%', objectFit: 'cover', display: 'block' }}
        />
      ) : failed ? (
        <div
          style={{
            position: 'absolute',
            inset: 0,
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            gap: 6,
          }}
        >
          <Image size={ICON_SIZE.xl} style={{ color: 'var(--text-tertiary)', opacity: 0.6 }} />
          <span
            style={{
              fontSize: 'var(--text-badge)',
              color: 'var(--text-tertiary)',
              padding: '0 4px',
              textAlign: 'center',
            }}
          >
            {t('common:photo_thumbnail_failed', 'Thumbnail failed')}
          </span>
        </div>
      ) : (
        <LoadingPlaceholder
          variant="toolbar"
          style={{ position: 'absolute', inset: 0, minHeight: 0 }}
        />
      )}
    </div>
  );
}

/**
 * 照片集网格：移动端固定 4 列；桌面端 `auto-fill + minmax` 自适应列数铺满可用宽度。
 * 网格项统一 `aspect-ratio: 1/1` + `object-fit: cover`。
 */
export function PhotoAlbumGrid({ items, onSelect }: PhotoAlbumGridProps) {
  const isMobile = isMobilePlatformSync();
  return (
    <div
      data-testid="photo-album-grid"
      style={{
        display: 'grid',
        gap: 8,
        gridTemplateColumns: isMobile
          ? 'repeat(4, 1fr)'
          : 'repeat(auto-fill, minmax(160px, 1fr))',
      }}
    >
      {items.map((item, index) => (
        <ThumbCell key={item.id} item={item} index={index} onSelect={onSelect} />
      ))}
    </div>
  );
}

import { useState } from 'react';
import { X } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';

type GuideImageProps = React.ImgHTMLAttributes<HTMLImageElement>;

export function GuideImage(props: GuideImageProps) {
  const [open, setOpen] = useState(false);

  return (
    <>
      <img
        {...props}
        onClick={() => setOpen(true)}
        style={{
          maxWidth: '100%',
          borderRadius: 8,
          cursor: 'zoom-in',
          border: '1px solid var(--border-subtle)',
          ...(props.style as React.CSSProperties),
        }}
      />
      {open && (
        <div
          onClick={() => setOpen(false)}
          style={{
            position: 'fixed',
            inset: 0,
            zIndex: 'var(--z-preview-overlay)',
            background: 'rgba(0,0,0,0.9)',
            backdropFilter: 'blur(4px)',
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            cursor: 'zoom-out',
          }}
        >
          <div
            style={{
              position: 'absolute',
              top: 16,
              right: 16,
              padding: 8,
              borderRadius: 8,
              background: 'rgba(255,255,255,0.1)',
              cursor: 'pointer',
              color: 'white',
            }}
          >
            <X size={ICON_SIZE.xl} />
          </div>
          <img
            {...props}
            style={{
              maxWidth: '90vw',
              maxHeight: '90vh',
              objectFit: 'contain',
              borderRadius: 8,
            }}
          />
        </div>
      )}
    </>
  );
}

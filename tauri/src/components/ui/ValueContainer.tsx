import { useState, useRef, useLayoutEffect } from 'react';

type WrapState = 'inline' | 'full' | 'full-wrapped';

function useFieldWrapState(value: string) {
  const ref = useRef<HTMLDivElement>(null);
  const stateRef = useRef<WrapState>('inline');
  const [state, setState] = useState<WrapState>('inline');

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const measure = () => {
      const rect = el.getBoundingClientRect();
      const computed = window.getComputedStyle(el);
      const lineHeight = parseFloat(computed.lineHeight) || parseFloat(computed.fontSize) * 1.2;
      const current = stateRef.current;
      const wrapped = rect.height > lineHeight * 1.5;
      let next = current;
      if (current === 'inline' && wrapped) {
        next = 'full';
      } else if (current === 'full' && wrapped) {
        next = 'full-wrapped';
      }
      if (next !== current) {
        stateRef.current = next;
        setState(next);
      }
    };
    measure();
    window.addEventListener('resize', measure);
    return () => window.removeEventListener('resize', measure);
  }, [value, state]);

  return { ref, state };
}

export function ValueContainer({
  value,
  children,
  action,
}: {
  value: string;
  children: React.ReactNode;
  action?: React.ReactNode;
}) {
  const { ref, state } = useFieldWrapState(value);
  const isFull = state === 'full' || state === 'full-wrapped';
  return (
    <div
      data-field-value
      style={{
        flex: isFull ? '0 0 100%' : '1 1 0%',
        minWidth: 0,
        maxWidth: '100%',
        textAlign: state === 'full-wrapped' ? 'left' : 'right',
        whiteSpace: 'normal',
        wordBreak: 'break-word',
        overflowWrap: 'break-word',
        display: 'flex',
        alignItems: 'center',
        justifyContent: state === 'full-wrapped' ? 'flex-start' : 'flex-end',
        gap: 6,
      }}
    >
      {/* 只测量文本是否换行，避免把移动端按钮的 48px 触控区误判为多行。 */}
      <div ref={ref} data-field-value-text style={{ minWidth: 0, maxWidth: '100%' }}>
        {children}
      </div>
      {action}
    </div>
  );
}

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

export function ValueContainer({ value, children }: { value: string; children: React.ReactNode }) {
  const { ref, state } = useFieldWrapState(value);
  const isFull = state === 'full' || state === 'full-wrapped';
  return (
    <div
      ref={ref}
      style={{
        flex: isFull ? '0 0 100%' : '1 1 0%',
        minWidth: 0,
        maxWidth: '100%',
        textAlign: state === 'full-wrapped' ? 'left' : 'right',
        whiteSpace: 'normal',
        wordBreak: 'break-word',
        overflowWrap: 'break-word',
      }}
    >
      {children}
    </div>
  );
}

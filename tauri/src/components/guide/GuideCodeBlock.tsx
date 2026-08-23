import { useRef } from 'react';
import { Copy, Check } from 'lucide-react';
import { SensitivityBadge, type SensitivityLevel } from '@/components/ui/SensitivityBadge';
import { ICON_SIZE } from '@/lib/constants';
import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';

interface GuideCodeBlockProps {
  children?: React.ReactNode;
  className?: string;
}

export function GuideCodeBlock({ children, className }: GuideCodeBlockProps) {
  // P025：复制逻辑收敛至共享 hook（降级行为改为选中文本，保留原 UX）
  const { copy, isCopied } = useCopyToClipboard(2000);
  const copied = isCopied();
  const ref = useRef<HTMLPreElement>(null);

  const handleCopy = async () => {
    const text = String(children).replace(/\n$/, '');
    const ok = await copy(text);
    if (!ok && ref.current) {
      // 降级：选中文本
      const selection = window.getSelection();
      const range = document.createRange();
      range.selectNodeContents(ref.current);
      selection?.removeAllRanges();
      selection?.addRange(range);
    }
  };

  const text = String(children).trim();
  const isInline = !className;

  const sensitivityLevels = ['public', 'internal', 'sensitive', 'critical'] as const;
  const isSensitivity =
    isInline && sensitivityLevels.includes(text as (typeof sensitivityLevels)[number]);

  if (isSensitivity) {
    const level = text as (typeof sensitivityLevels)[number];
    return (
      <span
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          verticalAlign: 'middle',
          transform: 'translateY(-1px)',
        }}
      >
        <SensitivityBadge level={level as SensitivityLevel} />
      </span>
    );
  }

  if (isInline) {
    return (
      <code
        style={{
          background: 'var(--bg-toolbar)',
          padding: '2px 5px',
          borderRadius: 4,
          fontSize: '0.9em',
          fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
          color: 'var(--accent-primary)',
        }}
      >
        {children}
      </code>
    );
  }

  return (
    <div style={{ position: 'relative', margin: '12px 0' }}>
      <button
        onClick={handleCopy}
        style={{
          position: 'absolute',
          top: 8,
          right: 8,
          padding: '4px 8px',
          borderRadius: 6,
          border: '1px solid var(--border-subtle)',
          background: 'var(--bg-elevated)',
          color: copied ? '#27ae60' : 'var(--text-secondary)',
          fontSize: 'var(--text-caption)',
          cursor: 'pointer',
          display: 'flex',
          alignItems: 'center',
          gap: 4,
          zIndex: 2,
        }}
        title={copied ? '已复制' : '复制'}
      >
        {copied ? <Check size={ICON_SIZE.sm} /> : <Copy size={ICON_SIZE.sm} />}
        {copied ? '已复制' : '复制'}
      </button>
      <pre
        ref={ref}
        className={className}
        style={{
          background: 'var(--bg-toolbar)',
          border: '1px solid var(--border-subtle)',
          borderRadius: 10,
          padding: '14px 16px',
          paddingTop: 36,
          overflow: 'auto',
          fontSize: 'var(--text-body-sm)',
          lineHeight: 1.6,
          fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
          color: 'var(--text-primary)',
          margin: 0,
        }}
      >
        <code>{children}</code>
      </pre>
    </div>
  );
}

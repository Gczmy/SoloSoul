import { useState, useCallback } from 'react';
import { Check } from 'lucide-react';
import styles from './CopyButton.module.css';

interface CopyButtonProps {
  getContent: () => string;
  label: string;
  copiedLabel?: string;
  icon: React.ReactNode;
  size?: 'sm' | 'md';
}

export function CopyButton({ getContent, label, copiedLabel = 'Copied', icon, size = 'sm' }: CopyButtonProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(getContent());
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // 静默忽略
    }
  }, [getContent]);

  return (
    <button
      className={`${styles.copyBtn} ${styles[size]} ${copied ? styles.glow : ''}`}
      onClick={handleCopy}
      title={label}
    >
      {copied ? <Check size={size === 'sm' ? 10 : 12} /> : icon}
      {copied ? copiedLabel : label}
    </button>
  );
}

import { useState } from 'react';
import { ChevronDown } from 'lucide-react';
import styles from './ExpandableSection.module.css';
import { ICON_SIZE } from '@/lib/iconSizes';

interface ExpandableSectionProps {
  title: string;
  defaultExpanded?: boolean;
  count?: number;
  actions?: React.ReactNode;
  children: React.ReactNode;
}

export function ExpandableSection({
  title,
  defaultExpanded = false,
  count,
  actions,
  children,
}: ExpandableSectionProps) {
  const [expanded, setExpanded] = useState(defaultExpanded);

  return (
    <>
      <button
        className={styles.inlineHeader}
        onClick={() => setExpanded(!expanded)}
        aria-expanded={expanded}
      >
        <div className={styles.inlineTitleRow}>
          <ChevronDown
            size={ICON_SIZE.xs}
            className={`${styles.chevron} ${expanded ? styles.chevronOpen : ''}`}
          />
          <span className={styles.inlineTitle}>{title}</span>
          {count !== undefined && <span className={styles.count}>{count}</span>}
        </div>
        {actions && (
          <div className={styles.inlineActions} onClick={(e) => e.stopPropagation()}>
            {actions}
          </div>
        )}
      </button>
      <div className={`${styles.collapsible} ${expanded ? styles.collapsibleOpen : ''}`}>
        {children}
      </div>
    </>
  );
}

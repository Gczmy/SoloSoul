import { ChevronDown } from 'lucide-react';
import styles from './ExpandableSection.module.css';
import { ICON_SIZE } from '@/lib/constants';

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
  return (
    <details className={styles.details} open={defaultExpanded}>
      <summary
        className={styles.summary}
        onClick={(e) => {
          // Prevent toggle when clicking on action buttons inside summary
          if ((e.target as HTMLElement).closest('[data-actions]')) {
            e.preventDefault();
          }
        }}
      >
        <div className={styles.headerRow}>
          <ChevronDown size={ICON_SIZE.xs} className={styles.chevron} />
          <span className={styles.title}>{title}</span>
          {count !== undefined && <span className={styles.count}>{count}</span>}
        </div>
        {actions && (
          <div className={styles.actions} data-actions>
            {actions}
          </div>
        )}
      </summary>
      <div className={styles.content}>{children}</div>
    </details>
  );
}

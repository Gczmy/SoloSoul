import React from 'react';
import { XCircle, AlertTriangle, Clock, Info, CheckCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import styles from './ExpiryGuardianView.module.css';

interface ExpiryItem {
  objectId: string;
  objectName: string;
  kind: string;
  expiryDate: string;
  daysRemaining: number;
  urgency: 'expired' | 'critical' | 'warning' | 'notice' | 'safe';
}

interface ExpirySummary {
  total: number;
  expired: number;
  critical: number;
  warning: number;
  notice: number;
  safe: number;
}

interface ExpiryGuardianPayload {
  type: string;
  title: string;
  locale: string;
  items: ExpiryItem[];
  summary: ExpirySummary;
}

interface Props {
  payload: ExpiryGuardianPayload;
}

const URGENCY_LABELS: Record<
  string,
  { zh: string; en: string }
> = {
  expired: { zh: '已过期', en: 'Expired' },
  critical: { zh: '紧急', en: 'Critical' },
  warning: { zh: '警告', en: 'Warning' },
  notice: { zh: '注意', en: 'Notice' },
  safe: { zh: '安全', en: 'Safe' },
};

const URGENCY_ICON: Record<string, React.ReactNode> = {
  expired: <XCircle size={14} />,
  critical: <AlertTriangle size={14} />,
  warning: <Clock size={14} />,
  notice: <Info size={14} />,
  safe: <CheckCircle size={14} />,
};

const URGENCY_CLASS: Record<string, string> = {
  expired: styles.urgencyExpired,
  critical: styles.urgencyCritical,
  warning: styles.urgencyWarning,
  notice: styles.urgencyNotice,
  safe: styles.urgencySafe,
};

export const ExpiryGuardianView: React.FC<Props> = ({ payload }) => {
  useTranslation();
  const isZh = payload.locale.startsWith('zh');

  return (
    <div className={styles.container}>
      <h3 className={styles.title}>{payload.title}</h3>

      <div className={styles.summary}>
        {(Object.keys(URGENCY_LABELS) as Array<keyof typeof URGENCY_LABELS>).map((key) => {
          const count = payload.summary[key as keyof ExpirySummary];
          if (count === 0) return null;
          const label = URGENCY_LABELS[key];
          return (
            <div key={key} className={`${styles.summaryItem} ${URGENCY_CLASS[key]}`}>
              <span className={styles.summaryIcon}>{URGENCY_ICON[key]}</span>
              <strong className={styles.summaryCount}>{count}</strong>
              <span className={styles.summaryLabel}>
                {isZh ? label.zh : label.en}
              </span>
            </div>
          );
        })}
      </div>

      {payload.items.length > 0 ? (
        <ul className={styles.list}>
          {payload.items.map((item) => (
            <li
              key={item.objectId}
              className={`${styles.item} ${URGENCY_CLASS[item.urgency]}`}
            >
              <div className={styles.itemHeader}>
                <span className={styles.itemKind}>{item.kind}</span>
                <span className={styles.itemName}>{item.objectName}</span>
                <span className={styles.itemBadge}>
                  <span className={styles.itemBadgeIcon}>{URGENCY_ICON[item.urgency]}</span>
                  {isZh
                    ? URGENCY_LABELS[item.urgency]?.zh
                    : URGENCY_LABELS[item.urgency]?.en}
                </span>
              </div>
              <div className={styles.itemMeta}>
                {item.expiryDate} ·{' '}
                {isZh
                  ? `剩余 ${item.daysRemaining} 天`
                  : `${item.daysRemaining} days remaining`}
              </div>
            </li>
          ))}
        </ul>
      ) : (
        <p className={styles.empty}>
          {isZh ? '没有找到到期证件' : 'No expiring documents found'}
        </p>
      )}
    </div>
  );
};

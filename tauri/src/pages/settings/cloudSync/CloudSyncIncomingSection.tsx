/**
 * P007：云同步设置页——下行待导入快照列表 section。
 */
import { DownloadCloud } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { TransferButton } from '@/components/transfer/TransferButton';
import styles from '../CloudSyncPage.module.css';

interface CloudSyncIncomingSectionProps {
  incomingFiles: string[];
  importingFile: string | null;
  onImport: (file: string) => void;
}

export function CloudSyncIncomingSection({
  incomingFiles,
  importingFile,
  onImport,
}: CloudSyncIncomingSectionProps) {
  const { t } = useTranslation(['settings']);
  return (
    <Card className={styles.card}>
      <h2 className={styles.sectionTitle}>
        <DownloadCloud size={20} style={{ marginRight: 8 }} />
        {t('settings:cloud_sync_incoming_title')}
      </h2>
      <p className={styles.hint}>{t('settings:cloud_sync_incoming_hint')}</p>
      <div className={styles.incomingList}>
        {incomingFiles.map((file) => {
          const nameParts = file.split('/');
          const fileName = nameParts[nameParts.length - 1] || file;
          return (
            <div key={file} className={styles.incomingItem}>
              <span className={styles.incomingName}>{fileName}</span>
              <TransferButton
                variant="accent"
                onClick={() => onImport(file)}
                busy={importingFile === file}
                disabled={!!importingFile}
              >
                {t('settings:cloud_sync_import')}
              </TransferButton>
            </div>
          );
        })}
      </div>
    </Card>
  );
}

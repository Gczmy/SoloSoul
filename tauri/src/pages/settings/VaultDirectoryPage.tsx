import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { PageShell } from '@/components/layout/PageShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { VaultDirectorySection } from './VaultDirectorySection';

export function VaultDirectoryPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['settings', 'common']);

  return (
    <PageShell title={t('settings:vault_directory')} onBack={() => navigate('/settings')}>
      <PageContainer variant="form" gap="default">
        <VaultDirectorySection />
      </PageContainer>
    </PageShell>
  );
}

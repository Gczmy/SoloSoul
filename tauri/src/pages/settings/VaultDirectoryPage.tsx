import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { VaultDirectorySection } from './VaultDirectorySection';

export function VaultDirectoryPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['settings', 'common']);

  return (
    <AppShell title={t('settings:vault_directory')} onBack={() => navigate('/settings')}>
      <PageContainer variant="form" gap="default">
        <VaultDirectorySection />
      </PageContainer>
    </AppShell>
  );
}

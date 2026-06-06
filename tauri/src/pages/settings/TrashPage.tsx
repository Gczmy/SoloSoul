import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { useAuthStore } from '@/stores/authStore';
import { useObjectStore } from '@/stores/objectStore';

export function TrashPage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['settings', 'common']);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const { trashObjects, loadTrashObjects, restoreObject, purgeObject, isLoading } = useObjectStore();

  useEffect(() => {
    if (accountId) {
      loadTrashObjects(accountId);
    }
  }, [accountId]);

  return (
    <AppShell title={t('settings:trash')} onBack={() => navigate('/settings')}>
      <div style={{ maxWidth: 560, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 8 }}>
        {isLoading ? (
          <Card>
            <p style={{ textAlign: 'center', color: 'var(--text-tertiary)', padding: '24px 0' }}>
              {t('common:loading')}
            </p>
          </Card>
        ) : trashObjects.length === 0 ? (
          <Card>
            <p style={{ textAlign: 'center', color: 'var(--text-secondary)', padding: '24px 0', fontSize: 14 }}>
              {t('settings:trash_empty')}
            </p>
          </Card>
        ) : (
          trashObjects.map((obj) => (
            <Card key={obj.id}>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                <div>
                  <div style={{ fontSize: 14, fontWeight: 500 }}>{obj.name}</div>
                  <div style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
                    {obj.collectionType}
                  </div>
                </div>
                <div style={{ display: 'flex', gap: 8 }}>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => restoreObject(obj.id)}
                  >
                    {t('common:restore')}
                  </Button>
                  <button
                    onClick={() => purgeObject(obj.id)}
                    style={{
                      padding: '6px 12px', borderRadius: 8, border: 'none',
                      background: '#e74c3c', color: 'white',
                      fontSize: 13, fontWeight: 500, cursor: 'pointer',
                    }}
                  >
                    {t('common:delete_permanently')}
                  </button>
                </div>
              </div>
            </Card>
          ))
        )}
      </div>
    </AppShell>
  );
}

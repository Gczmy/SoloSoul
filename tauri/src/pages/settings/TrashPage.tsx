import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';

export function TrashPage() {
  return (
    <AppShell title="Trash" onBack={() => window.history.back()}>
      <div style={{ maxWidth: 480, margin: '0 auto' }}>
        <Card>
          <p
            style={{
              fontSize: 14,
              color: 'var(--text-secondary)',
              textAlign: 'center',
              padding: '24px 0',
            }}
          >
            Trash is empty. Deleted items appear here and are automatically cleaned after 30 days.
          </p>
        </Card>
      </div>
    </AppShell>
  );
}

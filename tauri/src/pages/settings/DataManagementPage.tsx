import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';

export function DataManagementPage() {
  return (
    <AppShell title="Data Management" onBack={() => window.history.back()}>
      <div
        style={{
          maxWidth: 480,
          margin: '0 auto',
          display: 'flex',
          flexDirection: 'column',
          gap: 12,
        }}
      >
        <Card>
          <h3 style={{ fontSize: 15, fontWeight: 600, marginBottom: 4 }}>Backup</h3>
          <p style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 12 }}>
            Create encrypted backups of your vault data.
          </p>
          <div style={{ display: 'flex', gap: 8 }}>
            <Button size="sm" variant="primary">
              Create Backup
            </Button>
            <Button size="sm" variant="secondary">
              Restore
            </Button>
          </div>
        </Card>
        <Card>
          <h3 style={{ fontSize: 15, fontWeight: 600, marginBottom: 4 }}>Export & Import</h3>
          <p style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 12 }}>
            Export or import your data in .solosoul format.
          </p>
          <div style={{ display: 'flex', gap: 8 }}>
            <Button size="sm" variant="primary">
              Export
            </Button>
            <Button size="sm" variant="secondary">
              Import
            </Button>
          </div>
        </Card>
      </div>
    </AppShell>
  );
}

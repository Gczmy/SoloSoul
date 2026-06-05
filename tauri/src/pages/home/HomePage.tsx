import { useNavigate } from 'react-router-dom';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';

const sections = [
  { type: 'identity', label: 'Identity', icon: '🆔', desc: 'Personal info, ID cards, contacts' },
  { type: 'travel', label: 'Travel', icon: '🛂', desc: 'Passports, visas, travel history' },
  { type: 'financial', label: 'Financial', icon: '💰', desc: 'Bank accounts, cards, tax info' },
  {
    type: 'professional',
    label: 'Professional',
    icon: '💼',
    desc: 'Education, employment, skills',
  },
];

export function HomePage() {
  const navigate = useNavigate();

  return (
    <AppShell title="Home">
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: 20,
          maxWidth: 720,
          margin: '0 auto',
        }}
      >
        <Card>
          <h2 style={{ fontSize: 20, fontWeight: 600, marginBottom: 4 }}>Welcome back</h2>
          <p style={{ fontSize: 14, color: 'var(--text-secondary)' }}>
            Your personal data vault. All data encrypted and stored locally.
          </p>
        </Card>

        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(auto-fill, minmax(240px, 1fr))',
            gap: 12,
          }}
        >
          {sections.map((s) => (
            <Card key={s.type} interactive onClick={() => navigate(`/workspace?section=${s.type}`)}>
              <div style={{ fontSize: 28, marginBottom: 8 }}>{s.icon}</div>
              <h3 style={{ fontSize: 15, fontWeight: 600, marginBottom: 4 }}>{s.label}</h3>
              <p style={{ fontSize: 13, color: 'var(--text-secondary)' }}>{s.desc}</p>
            </Card>
          ))}
        </div>
      </div>
    </AppShell>
  );
}

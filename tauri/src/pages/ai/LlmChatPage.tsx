import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { Card } from '@/components/ui/Card';
import { MessageSquare } from 'lucide-react';

/** P3 — AI Chat page, wireframe for future LLM chat integration */
export function LlmChatPage() {
  const { t } = useTranslation(['settings', 'common']);
  return (
    <AppShell title={t('settings:items.ai_chat', { defaultValue: 'AI Chat' })}>
      <div style={{ maxWidth: 600, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 16 }}>
        <Card>
          <div style={{ textAlign: 'center', padding: '48px 24px' }}>
            <MessageSquare size={48} style={{ marginBottom: 16, opacity: 0.3, color: 'var(--text-tertiary)' }} />
            <h2 style={{ fontSize: 18, fontWeight: 600, margin: '0 0 8px' }}>
              {t('settings:ai_chat_title', { defaultValue: 'AI Chat' })}
            </h2>
            <p style={{ fontSize: 14, color: 'var(--text-secondary)', margin: 0 }}>
              {t('settings:ai_chat_description', { defaultValue: 'AI features are under development.' })}
            </p>
          </div>
        </Card>
      </div>
    </AppShell>
  );
}

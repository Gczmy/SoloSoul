import { memo } from 'react';
import { Card } from '@/components/ui/Card';
import { formatTokens } from '@/lib/llm/statsApi';
import type { TFunction } from 'i18next';

interface TokenBreakdownCardProps {
  sessionPrompt: number;
  sessionCompletion: number;
  accountPrompt: number;
  accountCompletion: number;
  t: TFunction;
}

export function TokenBreakdownCard({
  sessionPrompt,
  sessionCompletion,
  accountPrompt,
  accountCompletion,
  t,
}: TokenBreakdownCardProps) {
  const sessionTotal = sessionPrompt + sessionCompletion;
  const accountTotal = accountPrompt + accountCompletion;

  return (
    <Card>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 16, padding: '4px 0' }}>
        {/* Session */}
        <div>
          <div
            style={{
              fontSize: 'var(--text-caption)',
              fontWeight: 600,
              color: 'var(--text-primary)',
              marginBottom: 8,
            }}
          >
            {t('settings:llm_this_session')}
          </div>
          <TokenBar
            prompt={sessionPrompt}
            completion={sessionCompletion}
            total={sessionTotal}
            t={t}
          />
        </div>
        {/* Account */}
        <div>
          <div
            style={{
              fontSize: 'var(--text-caption)',
              fontWeight: 600,
              color: 'var(--text-primary)',
              marginBottom: 8,
            }}
          >
            {t('settings:llm_account_total')}
          </div>
          <TokenBar
            prompt={accountPrompt}
            completion={accountCompletion}
            total={accountTotal}
            t={t}
          />
        </div>
      </div>
    </Card>
  );
}

const TokenBar = memo(function TokenBar({
  prompt,
  completion,
  total,
  t,
}: {
  prompt: number;
  completion: number;
  total: number;
  t: TFunction;
}) {
  const promptRatio = total === 0 ? 0 : prompt / total;
  const completionRatio = total === 0 ? 0 : completion / total;

  return (
    <div>
      <div style={{ display: 'flex', height: 12, borderRadius: 4, overflow: 'hidden' }}>
        <div
          style={{
            flex: promptRatio * 1000,
            background: 'var(--accent-primary)',
            minWidth: prompt > 0 ? 2 : 0,
          }}
        />
        <div
          style={{
            flex: completionRatio * 1000,
            background: 'var(--accent-warm)',
            minWidth: completion > 0 ? 2 : 0,
          }}
        />
      </div>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 16,
          marginTop: 6,
          fontSize: 'var(--text-caption)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
          <div
            style={{ width: 8, height: 8, borderRadius: 2, background: 'var(--accent-primary)' }}
          />
          <span>Prompt {formatTokens(prompt)}</span>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
          <div style={{ width: 8, height: 8, borderRadius: 2, background: 'var(--accent-warm)' }} />
          <span>Completion {formatTokens(completion)}</span>
        </div>
        <span style={{ marginLeft: 'auto', color: 'var(--text-tertiary)' }}>
          {t('settings:llm_total_label')} {formatTokens(total)}
        </span>
      </div>
    </div>
  );
});

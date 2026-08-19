import type { AccountInfo } from '@/lib/ipc';

/**
 * P013/2: 登录页账户选择器 — 多账户显示下拉框，单账户显示名称卡片。
 * 始终预留空间，避免切换登录方式时下方内容位移。
 */
export function LoginAccountSelector({
  accounts,
  selectedAccountId,
  onSelect,
}: {
  accounts: AccountInfo[];
  selectedAccountId: string;
  onSelect: (id: string) => void;
}) {
  return (
    <div style={{ marginBottom: 20, width: '100%', minHeight: 62 }}>
      {accounts.length > 0 &&
        (accounts.length > 1 ? (
          <select
            value={selectedAccountId}
            onChange={(e) => onSelect(e.target.value)}
            style={{
              width: '100%',
              padding: '10px 14px',
              borderRadius: 8,
              border: '1px solid var(--border-subtle)',
              background: 'var(--bg-elevated)',
              color: 'var(--text-primary)',
              fontSize: 'var(--text-body)',
              fontFamily: 'inherit',
              outline: 'none',
              textAlign: 'left',
            }}
          >
            {accounts.map((acc) => (
              <option key={acc.id} value={acc.id}>
                {acc.name} · {acc.id}
              </option>
            ))}
          </select>
        ) : (
          <div
            style={{
              width: '100%',
              padding: '10px 14px',
              borderRadius: 8,
              border: '1px solid var(--border-subtle)',
              background: 'var(--bg-elevated)',
              color: 'var(--text-primary)',
              fontSize: 'var(--text-body)',
              textAlign: 'left',
            }}
          >
            <div>{accounts[0]?.name}</div>
            <div
              style={{
                fontSize: 'var(--text-badge)',
                color: 'var(--text-tertiary)',
                marginTop: 2,
                fontFamily: 'monospace',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
              }}
            >
              {accounts[0]?.id}
            </div>
          </div>
        ))}
    </div>
  );
}

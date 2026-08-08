/**
 * P013/2: 登录页底部图标栏 — 切换解锁方式。
 * 两阶段悬停（边框/颜色立即高亮，文字/展开延迟 300ms）与选中态由父组件状态驱动，
 * 本组件仅做展示与事件转发。
 */
export interface LoginMethodOption {
  id: 'faceId' | 'touchId' | 'windowsHello' | 'pin' | 'password';
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
}

export function LoginIconBar({
  loginMethod,
  iconMethods,
  hoveredIcon,
  committedIcon,
  onIconEnter,
  onIconLeave,
  onIconClick,
}: {
  loginMethod: 'faceId' | 'touchId' | 'windowsHello' | 'pin' | 'password';
  iconMethods: LoginMethodOption[];
  hoveredIcon: string | null;
  committedIcon: string | null;
  onIconEnter: (id: string) => void;
  onIconLeave: () => void;
  onIconClick: (method: LoginMethodOption) => void;
}) {
  return (
    <div
      style={{
        display: 'flex',
        gap: 6,
        paddingTop: 12,
        marginTop: 'auto',
        borderTop: '1px solid var(--border-subtle)',
        justifyContent: 'flex-start',
        overflow: 'hidden',
        maxWidth: '100%',
      }}
    >
      {iconMethods.map((method) => {
        const isActive = loginMethod === method.id;
        const isHovered = hoveredIcon === method.id;
        const isExpanded = committedIcon === method.id;

        return (
          <button
            key={method.id}
            aria-label={method.label}
            onClick={() => onIconClick(method)}
            onMouseEnter={() => onIconEnter(method.id)}
            onMouseLeave={onIconLeave}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              padding: '6px 10px',
              borderRadius: 8,
              border: `1px solid ${
                isHovered
                  ? 'var(--accent-primary)'
                  : isActive
                    ? 'color-mix(in srgb, var(--accent-primary) 40%, transparent)'
                    : 'transparent'
              }`,
              background: isActive
                ? 'color-mix(in srgb, var(--accent-primary) 6%, transparent)'
                : 'transparent',
              cursor: 'pointer',
              fontFamily: 'inherit',
              fontSize: 'var(--text-body-sm)',
              color: isHovered
                ? 'var(--accent-primary)'
                : isActive
                  ? 'var(--text-primary)'
                  : 'var(--text-tertiary)',
              whiteSpace: 'nowrap',
              overflow: 'hidden',
              maxWidth: isExpanded ? 200 : 40,
              transition:
                isExpanded || (!isHovered && !isExpanded)
                  ? 'all 0.25s ease'
                  : 'all 0.25s ease, max-width 0.01s linear 0.2s',
              flexShrink: 0,
              outline: 'none',
            }}
          >
            <span style={{ flexShrink: 0, display: 'flex', alignItems: 'center' }}>
              {method.icon}
            </span>
            <span
              style={{
                opacity: isExpanded ? 1 : 0,
                transition: 'opacity 0.2s ease 0.05s',
                overflow: 'hidden',
                whiteSpace: 'nowrap',
              }}
            >
              {method.label}
            </span>
          </button>
        );
      })}
    </div>
  );
}

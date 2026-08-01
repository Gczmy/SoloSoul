interface WarningCancelButtonProps {
  onClick: () => void;
  children: string;
}

/**
 * 弱密码/提示弹窗的「取消」按钮：warning 描边按钮。
 * hover 由 `interactive-warning-cancel` 工具类（CSS :hover）表达，
 * 替代此前 state 版手写 hover（useState hovered 仅切换 background）。
 */
export function WarningCancelButton({ onClick, children }: WarningCancelButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="interactive-warning-cancel"
      style={{
        padding: '6px 12px',
        fontSize: 'var(--text-body-sm)',
        borderRadius: 6,
        borderStyle: 'solid',
        borderWidth: 1,
        cursor: 'pointer',
        fontWeight: 500,
        transition: 'background 0.15s',
        fontFamily: 'inherit',
      }}
    >
      {children}
    </button>
  );
}

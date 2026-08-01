/**
 * ToggleSwitch — 共享开关滑块组件（44×24，checkbox + 两个 span，0.2s 过渡）。
 *
 * 从 BiometricSection 抽取为共享组件，避免多份拷贝；
 * 设备同步自动同步开关（SyncStatusCard）与生物识别开关共用。
 */
export function ToggleSwitch({
  checked,
  onChange,
  disabled = false,
}: {
  checked: boolean;
  onChange: () => void;
  disabled?: boolean;
}) {
  return (
    <label
      style={{
        position: 'relative',
        display: 'inline-block',
        width: 44,
        height: 24,
        cursor: disabled ? 'not-allowed' : 'pointer',
        flexShrink: 0,
        opacity: 1,
      }}
    >
      <input
        type="checkbox"
        checked={checked}
        onChange={disabled ? () => {} : onChange}
        style={{ opacity: 0, width: 0, height: 0 }}
      />
      <span
        style={{
          position: 'absolute',
          inset: 0,
          background: checked ? 'var(--accent-primary)' : 'var(--border-subtle)',
          borderRadius: 12,
          transition: '0.2s',
        }}
      />
      <span
        style={{
          position: 'absolute',
          top: 2,
          left: checked ? 22 : 2,
          width: 20,
          height: 20,
          borderRadius: '50%',
          background: 'white',
          transition: '0.2s',
          boxShadow: '0 1px 3px rgba(0,0,0,0.2)',
        }}
      />
    </label>
  );
}

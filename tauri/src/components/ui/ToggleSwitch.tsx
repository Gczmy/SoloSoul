import styles from './ToggleSwitch.module.css';

/** 轨道与命中区分离；平台仅提供尺寸和颜色 token。 */
export function ToggleSwitch({
  checked,
  onChange,
  disabled = false,
  ariaLabel,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  ariaLabel?: string;
}) {
  return (
    <label data-ui-switch data-disabled={disabled} className={styles.root}>
      <input
        className={styles.input}
        type="checkbox"
        role="switch"
        aria-label={ariaLabel}
        checked={checked}
        disabled={disabled}
        onChange={(event) => onChange(event.target.checked)}
      />
      <span aria-hidden="true" data-switch-track className={styles.track}>
        <span className={styles.thumb} />
      </span>
    </label>
  );
}

import React, { InputHTMLAttributes, forwardRef, useState, useEffect, useRef } from 'react';
import { X } from 'lucide-react';
import styles from './Input.module.css';
import { ICON_SIZE } from '@/lib/constants';

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
  /** 错误抖动重触发计数器：error 字符串相同但 tick 变化时也重新抖动（可选）。 */
  errorTick?: number;
  /** 为错误行预留固定高度，error 出现/消失不改变表单布局（可选，防闪烁）。 */
  reserveErrorSpace?: boolean;
  /** Icon displayed inside the input field on the left side (e.g. search icon). */
  prefixIcon?: React.ReactNode;
  icon?: React.ReactNode;
  badge?: React.ReactNode;
  /** When provided, shows a clear (X) button at the right side of the input. */
  onClear?: () => void;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  (
    { label, error, errorTick = 0, reserveErrorSpace = false, prefixIcon, icon, badge, onClear, className, ...props },
    ref,
  ) => {
    const [shouldShake, setShouldShake] = useState(false);
    const prevErrorRef = useRef(error);
    const prevTickRef = useRef(errorTick);

    useEffect(() => {
      if (error && (error !== prevErrorRef.current || errorTick !== prevTickRef.current)) {
        setShouldShake(true);
        const timer = setTimeout(() => setShouldShake(false), 300);
        prevErrorRef.current = error;
        prevTickRef.current = errorTick;
        return () => clearTimeout(timer);
      }
      prevErrorRef.current = error;
      prevTickRef.current = errorTick;
    }, [error, errorTick]);

    const hasValue = props.value !== undefined && props.value !== null && props.value !== '';
    const showClear = onClear && hasValue;

    return (
      <div className={styles.wrapper}>
        {label && (
          <label className={styles.label}>
            <span className={styles.labelRow}>
              {icon}
              <span className={styles.labelText}>{label}</span>
              {badge}
            </span>
          </label>
        )}
        <div className={styles.inputWrap}>
          {prefixIcon && <span className={styles.prefixIcon}>{prefixIcon}</span>}
          <input
            ref={ref}
            className={[
              styles.input,
              prefixIcon ? styles.hasPrefix : '',
              showClear ? styles.hasClear : '',
              error ? styles.hasError : '',
              shouldShake ? styles.shake : '',
              className || '',
            ]
              .filter(Boolean)
              .join(' ')}
            {...props}
          />
          {showClear && (
            <button
              type="button"
              className={styles.clearBtn}
              onClick={onClear}
              tabIndex={-1}
              aria-label="Clear"
            >
              <X size={ICON_SIZE.sm} />
            </button>
          )}
        </div>
        {reserveErrorSpace ? (
          <div style={{ minHeight: 16 }}>
            {error && (
              <span className={styles.error} key={`${error}-${errorTick}`}>
                {error}
              </span>
            )}
          </div>
        ) : (
          error && (
            <span className={styles.error} key={error}>
              {error}
            </span>
          )
        )}
      </div>
    );
  },
);
Input.displayName = 'Input';

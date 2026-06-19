import React, { InputHTMLAttributes, forwardRef, useState, useEffect, useRef } from 'react';
import { X } from 'lucide-react';
import styles from './Input.module.css';

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
  icon?: React.ReactNode;
  badge?: React.ReactNode;
  /** When provided, shows a clear (X) button at the right side of the input. */
  onClear?: () => void;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ label, error, icon, badge, onClear, className, ...props }, ref) => {
    const [shouldShake, setShouldShake] = useState(false);
    const prevErrorRef = useRef(error);

    useEffect(() => {
      if (error && error !== prevErrorRef.current) {
        setShouldShake(true);
        const timer = setTimeout(() => setShouldShake(false), 300);
        prevErrorRef.current = error;
        return () => clearTimeout(timer);
      }
      prevErrorRef.current = error;
    }, [error]);

    const hasValue =
      props.value !== undefined && props.value !== null && props.value !== '';
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
          <input
            ref={ref}
            className={[
              styles.input,
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
              <X size={14} />
            </button>
          )}
        </div>
        {error && <span className={styles.error} key={error}>{error}</span>}
      </div>
    );
  },
);
Input.displayName = 'Input';

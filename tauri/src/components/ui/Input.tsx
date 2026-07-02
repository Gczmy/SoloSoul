import React, { InputHTMLAttributes, forwardRef, useState, useEffect, useRef } from 'react';
import { X } from 'lucide-react';
import styles from './Input.module.css';
import { ICON_SIZE } from '@/lib/constants';

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
  /** Icon displayed inside the input field on the left side (e.g. search icon). */
  prefixIcon?: React.ReactNode;
  icon?: React.ReactNode;
  badge?: React.ReactNode;
  /** When provided, shows a clear (X) button at the right side of the input. */
  onClear?: () => void;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ label, error, prefixIcon, icon, badge, onClear, className, ...props }, ref) => {
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
        {error && (
          <span className={styles.error} key={error}>
            {error}
          </span>
        )}
      </div>
    );
  },
);
Input.displayName = 'Input';

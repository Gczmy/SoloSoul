import React, { InputHTMLAttributes, forwardRef, useState, useEffect, useRef } from 'react';
import styles from './Input.module.css';

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
  icon?: React.ReactNode;
  badge?: React.ReactNode;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ label, error, icon, badge, className, ...props }, ref) => {
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
        <input
          ref={ref}
          className={[
            styles.input,
            error ? styles.hasError : '',
            shouldShake ? styles.shake : '',
            className || '',
          ]
            .filter(Boolean)
            .join(' ')}
          {...props}
        />
        {error && <span className={styles.error} key={error}>{error}</span>}
      </div>
    );
  },
);
Input.displayName = 'Input';

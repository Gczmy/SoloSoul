import React, { InputHTMLAttributes, forwardRef } from 'react';
import styles from './Input.module.css';

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
  icon?: React.ReactNode;
  badge?: React.ReactNode;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ label, error, icon, badge, className, ...props }, ref) => {
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
          className={`${styles.input} ${error ? styles.hasError : ''} ${className || ''}`}
          {...props}
        />
        {error && <span className={styles.error}>{error}</span>}
      </div>
    );
  },
);
Input.displayName = 'Input';

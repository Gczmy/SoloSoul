import { ButtonHTMLAttributes, forwardRef } from 'react';
import styles from './Button.module.css';

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?:
    | 'primary'
    | 'secondary'
    | 'tertiary'
    | 'glass'
    | 'danger'
    | 'danger-outline'
    | 'warning';
  size?: 'sm' | 'md' | 'lg';
  loading?: boolean;
}

const variantIntent = {
  primary: 'primary',
  secondary: 'neutral',
  tertiary: 'text',
  glass: 'quiet',
  danger: 'danger',
  'danger-outline': 'danger-soft',
  warning: 'warning',
} as const;

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ variant = 'primary', size = 'md', loading, children, className, disabled, ...props }, ref) => {
    return (
      <button
        ref={ref}
        data-ui-button={variant}
        data-ui-intent={variantIntent[variant]}
        data-ui-size={size}
        className={`${styles.button} ${className || ''}`}
        disabled={disabled || loading}
        {...props}
      >
        {children}
      </button>
    );
  },
);

Button.displayName = 'Button';

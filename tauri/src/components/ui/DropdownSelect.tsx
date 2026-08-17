import { useState, useRef, useEffect, useCallback, useId } from 'react';
import { ChevronDown } from 'lucide-react';
import styles from './DropdownSelect.module.css';
import { ICON_SIZE } from '@/lib/constants';

export interface DropdownSelectOption {
  value: string | number;
  label: string;
}

interface DropdownSelectProps {
  /** Currently selected value */
  value: string | number;
  /** Called when user selects an option */
  onChange: (value: string | number) => void;
  /** Available options */
  options: DropdownSelectOption[];
  /** Text displayed on the trigger button */
  triggerLabel: string;
  /** aria-label for accessibility */
  ariaLabel?: string;
  /** Width of the popover in px (default 90) */
  width?: number;
}

export function DropdownSelect({
  value,
  onChange,
  options,
  triggerLabel,
  ariaLabel,
  width = 90,
}: DropdownSelectProps) {
  const [open, setOpen] = useState(false);
  const listRef = useRef<HTMLDivElement>(null);
  const areaId = useId();

  // Close on click outside the component
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      const area = document.querySelector(`[data-dropdown-area="${areaId}"]`);
      if (area && !area.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    const blurHandler = () => setOpen(false);
    document.addEventListener('mousedown', handler);
    window.addEventListener('blur', blurHandler);
    return () => {
      document.removeEventListener('mousedown', handler);
      window.removeEventListener('blur', blurHandler);
    };
  }, [open, areaId]);

  // Auto-scroll to the currently selected item when opened
  useEffect(() => {
    if (open && listRef.current) {
      const item = listRef.current.querySelector(`[data-dd-value="${value}"]`);
      if (item) {
        item.scrollIntoView({ block: 'center' });
      }
    }
  }, [open, value]);

  const handleSelect = useCallback(
    (newValue: string | number) => {
      onChange(newValue);
      setOpen(false);
    },
    [onChange],
  );

  return (
    <div data-dropdown-area={areaId} style={{ position: 'relative' }}>
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className={styles.trigger}
        aria-label={ariaLabel}
        aria-expanded={open}
      >
        {triggerLabel}
        <ChevronDown size={ICON_SIZE.xs} className={styles.chevron} />
      </button>
      {open && (
        <div className={styles.popover} ref={listRef} style={{ width }}>
            {options.map((opt) => {
              const isActive = opt.value === value;
              return (
                <button
                  key={opt.value}
                  type="button"
                  data-dd-value={opt.value}
                  className={`${styles.item} ${isActive ? styles.itemActive : ''}`}
                  onClick={(e) => {
                    e.stopPropagation();
                    handleSelect(opt.value);
                  }}
                >
                  {opt.label}
                </button>
              );
            })}
        </div>
      )}
    </div>
  );
}

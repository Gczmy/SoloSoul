import { useState, useRef, useEffect, useCallback } from 'react';

interface PinInputProps {
  length: number;
  onComplete: (pin: string) => void;
  disabled?: boolean;
  error?: boolean;
}

/**
 * 数字 PIN 码输入组件。
 * 每个字符独立小框，自动焦点推进，Backspace 回退。
 */
export function PinInput({ length, onComplete, disabled, error }: PinInputProps) {
  const [digits, setDigits] = useState<string[]>(Array(length).fill(''));
  const inputRefs = useRef<(HTMLInputElement | null)[]>([]);
  const [activeIdx, setActiveIdx] = useState(0);

  // 确保 ref 数组长度与 length 一致
  useEffect(() => {
    inputRefs.current = inputRefs.current.slice(0, length);
    while (inputRefs.current.length < length) {
      inputRefs.current.push(null);
    }
  }, [length]);

  // 重置时聚焦第一个框
  useEffect(() => {
    setDigits(Array(length).fill(''));
    setActiveIdx(0);
    inputRefs.current[0]?.focus();
  }, [length]);

  const focusInput = useCallback((idx: number) => {
    if (idx >= 0 && idx < length) {
      inputRefs.current[idx]?.focus();
      setActiveIdx(idx);
    }
  }, [length]);

  const handleChange = useCallback((idx: number, value: string) => {
    // 只允许数字
    const digit = value.replace(/\D/g, '').slice(-1);
    const newDigits = [...digits];
    newDigits[idx] = digit;
    setDigits(newDigits);

    if (digit) {
      // 如果有下一个输入框，自动聚焦
      if (idx < length - 1) {
        focusInput(idx + 1);
      } else {
        // 最后一位输入完成，触发 onComplete
        const pin = newDigits.join('');
        if (pin.length === length) {
          onComplete(pin);
        }
      }
    }
  }, [digits, length, focusInput, onComplete]);

  const handleKeyDown = useCallback((idx: number, e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Backspace') {
      if (digits[idx]) {
        // 当前位有值 → 清除
        const newDigits = [...digits];
        newDigits[idx] = '';
        setDigits(newDigits);
      } else if (idx > 0) {
        // 当前位无值 → 回退到前一位并清除
        const newDigits = [...digits];
        newDigits[idx - 1] = '';
        setDigits(newDigits);
        focusInput(idx - 1);
      }
    } else if (e.key === 'ArrowLeft') {
      focusInput(idx - 1);
    } else if (e.key === 'ArrowRight') {
      focusInput(idx + 1);
    }
  }, [digits, focusInput]);

  const handlePaste = useCallback((e: React.ClipboardEvent) => {
    e.preventDefault();
    const pasted = e.clipboardData.getData('text').replace(/\D/g, '').slice(0, length);
    if (!pasted) return;
    const newDigits = [...digits];
    for (let i = 0; i < pasted.length; i++) {
      newDigits[i] = pasted[i];
    }
    setDigits(newDigits);
    const nextIdx = Math.min(pasted.length, length - 1);
    if (pasted.length === length) {
      onComplete(pasted);
    } else {
      focusInput(nextIdx);
    }
  }, [digits, length, focusInput, onComplete]);

  return (
    <div
      style={{
        display: 'flex',
        gap: 8,
        justifyContent: 'center',
        alignItems: 'center',
      }}
    >
      {Array.from({ length }).map((_, idx) => (
        <input
          key={idx}
          ref={(el) => { inputRefs.current[idx] = el; }}
          type="password"
          inputMode="numeric"
          autoComplete="one-time-code"
          value={digits[idx]}
          onChange={(e) => handleChange(idx, e.target.value)}
          onKeyDown={(e) => handleKeyDown(idx, e)}
          onPaste={idx === 0 ? handlePaste : undefined}
          onFocus={() => setActiveIdx(idx)}
          disabled={disabled}
          maxLength={1}
          style={{
            width: 48,
            height: 56,
            textAlign: 'center',
            fontSize: 'var(--text-card-title, 20px)',
            fontFamily: 'monospace',
            fontWeight: 600,
            borderRadius: 10,
            border: error
              ? '2px solid #dc2626'
              : activeIdx === idx
                ? '2px solid var(--accent-primary)'
                : '1px solid var(--border-subtle)',
            background: error
              ? 'rgba(220, 38, 38, 0.06)'
              : 'var(--bg-toolbar)',
            color: 'var(--text-primary)',
            outline: 'none',
            transition: 'border-color 0.15s ease, background 0.15s ease',
            caretColor: 'var(--accent-primary)',
          }}
        />
      ))}
    </div>
  );
}

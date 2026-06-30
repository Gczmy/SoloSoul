import { useState, useRef, useEffect, useCallback } from 'react';

interface PinInputProps {
  length: number;
  onComplete: (pin: string) => void;
  disabled?: boolean;
  error?: boolean;
  verifying?: boolean;
}

/**
 * 数字 PIN 码输入组件。
 * 每个字符独立小框，自动焦点推进，Backspace 回退。
 */
export function PinInput({ length, onComplete, disabled, error, verifying }: PinInputProps) {
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

  const totalWidth = length * 40 + (length - 1) * 8;

  return (
    <div
      style={{
        position: 'relative',
        display: 'flex',
        gap: 8,
        justifyContent: 'center',
        alignItems: 'center',
        height: 48,
      }}
    >
      {/* PIN 输入框 — verifying 时淡出 */}
      <div
        style={{
          display: 'flex',
          gap: 8,
          opacity: verifying ? 0 : 1,
          transition: 'opacity 0.25s ease',
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
            onMouseDown={(e) => e.preventDefault()}
            disabled={disabled || verifying}
            maxLength={1}
            style={{
              width: 40,
              height: 48,
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

      {/* 验证中动画 — 流动渐变横线 */}
      <div
        style={{
          position: 'absolute',
          top: '50%',
          left: '50%',
          transform: 'translate(-50%, -50%)',
          width: totalWidth,
          height: 4,
          borderRadius: 2,
          opacity: verifying ? 1 : 0,
          pointerEvents: 'none',
          transition: 'opacity 0.25s ease',
          background:
            'linear-gradient(90deg, transparent 0%, var(--accent-primary) 25%, transparent 50%, var(--accent-primary) 75%, transparent 100%)',
          backgroundSize: '200% 100%',
          animation: verifying ? 'pin-flow 2.8s linear infinite' : 'none',
          boxShadow: verifying
            ? '0 0 6px 2px color-mix(in srgb, var(--accent-primary) 40%, transparent), 0 0 16px 4px color-mix(in srgb, var(--accent-primary) 20%, transparent)'
            : 'none',
        }}
      />

      {/* 注入关键帧动画 */}
      <style>{`
        @keyframes pin-flow {
          0% { background-position: 200% 0; }
          100% { background-position: -200% 0; }
        }
      `}</style>
    </div>
  );
}

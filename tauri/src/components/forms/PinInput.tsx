import { useState, useRef, useEffect, forwardRef, useImperativeHandle } from 'react';

import { IndeterminateProgressBar } from '@/components/ui/IndeterminateProgressBar';

interface PinInputProps {
  length: number;
  onComplete: (pin: string) => void;
  disabled?: boolean;
  error?: boolean;
  verifying?: boolean;
}

export interface PinInputHandle {
  focus: () => void;
}

/**
 * 数字 PIN 码输入组件。
 * 一个隐藏 input 统一处理键盘/粘贴，纯视觉方框展示掩码 + 边框高亮指示当前输入位。
 * 额外通过全局 keydown 监听器确保点击卡片外部空白区域后仍能正常输入。
 */
export const PinInput = forwardRef<PinInputHandle, PinInputProps>(function PinInput(
  { length, onComplete, disabled, error, verifying },
  ref,
) {
  const [value, setValue] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);
  const valueRef = useRef('');
  // 保持 ref 与 state 同步，供全局 keydown 闭包读取最新值
  valueRef.current = value;

  useImperativeHandle(
    ref,
    () => ({
      focus: () => inputRef.current?.focus(),
    }),
    [],
  );

  const activeIndex = Math.min(value.length, length - 1);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (disabled || verifying) return;
    const digits = e.target.value.replace(/\D/g, '').slice(0, length);
    setValue(digits);
    if (digits.length === length) {
      onComplete(digits);
    }
  };

  const handleContainerClick = () => {
    inputRef.current?.focus();
  };

  // 全局 keydown 监听器：当隐藏 input 失去焦点时，仍然能捕获数字键输入
  useEffect(() => {
    if (disabled || verifying) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      // 如果隐藏 input 已有焦点，让它的 onChange 处理（避免双重处理）
      if (document.activeElement === inputRef.current) return;
      // 如果焦点在其它输入框/文本区中，不干扰
      const tag = (document.activeElement?.tagName || '').toLowerCase();
      if (tag === 'input' || tag === 'textarea' || tag === 'select') return;

      if (e.key >= '0' && e.key <= '9') {
        e.preventDefault();
        const next = (valueRef.current + e.key).slice(0, length);
        setValue(next);
        if (next.length === length) {
          onComplete(next);
        }
      } else if (e.key === 'Backspace') {
        e.preventDefault();
        setValue(valueRef.current.slice(0, -1));
      } else if (e.key === 'Enter' && valueRef.current.length > 0) {
        e.preventDefault();
        onComplete(valueRef.current);
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [length, onComplete, disabled, verifying]);

  const totalWidth = length * 40 + (length - 1) * 8;

  return (
    <div
      onClick={handleContainerClick}
      style={{
        position: 'relative',
        display: 'flex',
        gap: 8,
        justifyContent: 'center',
        alignItems: 'center',
        height: 48,
        cursor: 'default',
      }}
    >
      {/* 隐藏 input — 统一接收所有键盘/粘贴输入 */}
      <input
        ref={inputRef}
        autoFocus
        type="text"
        inputMode="numeric"
        pattern="[0-9]*"
        autoComplete="one-time-code"
        maxLength={length}
        value={value}
        onChange={handleChange}
        disabled={disabled || verifying}
        aria-label="PIN 输入"
        style={{
          position: 'absolute',
          top: 0,
          right: 0,
          bottom: 0,
          left: 0,
          opacity: 0,
          zIndex: 1,
          cursor: 'default',
          width: '100%',
          height: '100%',
        }}
      />

      {/* 纯视觉方框 — verifying 时淡出 */}
      <div
        style={{
          display: 'flex',
          gap: 8,
          opacity: verifying ? 0 : 1,
          transition: 'opacity 0.25s ease',
          pointerEvents: 'none',
        }}
      >
        {Array.from({ length }).map((_, i) => (
          <div
            key={i}
            style={{
              width: 40,
              height: 48,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              borderRadius: 10,
              border: error
                ? '2px solid #dc2626'
                : i === activeIndex
                  ? '2px solid var(--accent-primary)'
                  : '1px solid var(--border-subtle)',
              background: error ? 'rgba(220, 38, 38, 0.06)' : 'var(--bg-toolbar)',
              color: 'var(--text-primary)',
              fontFamily: 'monospace',
              fontSize: 'var(--text-card-title, 20px)',
              fontWeight: 600,
              transition: 'border-color 0.15s ease, background 0.15s ease',
            }}
          >
            {value[i] ? '●' : ''}
          </div>
        ))}
      </div>

      {/* 验证中动画 — 与外部存储选择一致的渐变进度条 */}
      <div
        style={{
          position: 'absolute',
          top: 0,
          right: 0,
          bottom: 0,
          left: 0,
          opacity: verifying ? 1 : 0,
          pointerEvents: 'none',
          transition: 'opacity 0.25s ease',
        }}
      >
        <div
          style={{
            position: 'absolute',
            top: '50%',
            left: '50%',
            transform: 'translate(-50%, -50%)',
            width: totalWidth,
          }}
        >
          <IndeterminateProgressBar height={4} />
        </div>
      </div>
    </div>
  );
});

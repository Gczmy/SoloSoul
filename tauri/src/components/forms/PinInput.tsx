import { useState, useRef } from 'react';

interface PinInputProps {
  length: number;
  onComplete: (pin: string) => void;
  disabled?: boolean;
  error?: boolean;
  verifying?: boolean;
}

/** 共用渐变横线 CSS 值 */
const GRADIENT_LINE =
  'linear-gradient(90deg, ' +
  'color-mix(in srgb, var(--accent-primary) 10%, var(--accent-warm)) 0%, ' +
  'color-mix(in srgb, var(--accent-primary) 80%, var(--accent-warm)) 15%, ' +
  'color-mix(in srgb, var(--accent-primary) 20%, var(--accent-warm)) 50%, ' +
  'color-mix(in srgb, var(--accent-primary) 80%, var(--accent-warm)) 85%, ' +
  'color-mix(in srgb, var(--accent-primary) 10%, var(--accent-warm)) 100%)';

/**
 * 数字 PIN 码输入组件。
 * 一个隐藏 input 统一处理键盘/粘贴，纯视觉方框展示掩码 + 边框高亮指示当前输入位。
 */
export function PinInput({ length, onComplete, disabled, error, verifying }: PinInputProps) {
  const [value, setValue] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);

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
              background: error
                ? 'rgba(220, 38, 38, 0.06)'
                : 'var(--bg-toolbar)',
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

      {/* 验证中动画 — 梯度 + 叠加移动光斑 */}
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
        <div style={{ position: 'absolute', top: '50%', left: '50%', transform: 'translate(-50%, -50%)', width: totalWidth, height: 8, borderRadius: 4, background: GRADIENT_LINE, backgroundSize: '150% 100%', animation: 'pin-flow 4s linear infinite' }}>
          <div style={{ position: 'absolute', top: 0, right: 0, bottom: 0, left: 0, borderRadius: 4, background: 'repeating-linear-gradient(-45deg, transparent 0px, transparent 10px, rgba(255,255,255,0.25) 12px, transparent 14px)', animation: 'pin-ripple 0.8s linear infinite', mixBlendMode: 'overlay' }} />
        </div>
      </div>

      {/* 注入关键帧动画 */}
      <style>{`
        @keyframes pin-flow {
          0% { background-position: 200% 0; }
          100% { background-position: -200% 0; }
        }
        @keyframes pin-ripple {
          0% { background-position: 0 0; }
          100% { background-position: 19.8px 0; }
        }

      `}</style>
    </div>
  );
}

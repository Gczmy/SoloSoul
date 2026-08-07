import { useState, useCallback, useRef, useEffect } from 'react';
import type { SensitivityLevel } from '@/types/template';
import { MASK_PLACEHOLDER, shouldMaskSensitivity } from '@/lib/masking';

interface RevealEntry {
  expiresAt: number; // Date.now() + TTL
}

const REVEAL_TTL_MS = 60_000; // 1 minute per spec §7

/**
 * §7 — Temporary reveal state for sensitive fields.
 * Pure frontend in-memory state, never persisted to vault.
 * Revealed fields auto-expire after TTL (1 minute).
 */
export function useRevealState() {
  const [revealed, setRevealed] = useState<Record<string, RevealEntry>>({});
  const timersRef = useRef<Record<string, ReturnType<typeof setTimeout>>>({});

  // Clear all pending timers on unmount to avoid setState on unmounted components
  useEffect(() => {
    return () => {
      Object.values(timersRef.current).forEach((timer) => clearTimeout(timer));
      timersRef.current = {};
    };
  }, []);

  /** Reveal a field for the TTL duration. */
  const reveal = useCallback((fieldId: string) => {
    const expiresAt = Date.now() + REVEAL_TTL_MS;
    setRevealed((prev) => ({ ...prev, [fieldId]: { expiresAt } }));

    // Clear previous timer
    if (timersRef.current[fieldId]) {
      clearTimeout(timersRef.current[fieldId]);
    }
    // Auto-hide after TTL
    timersRef.current[fieldId] = setTimeout(() => {
      setRevealed((prev) => {
        const next = { ...prev };
        delete next[fieldId];
        return next;
      });
      delete timersRef.current[fieldId];
    }, REVEAL_TTL_MS);
  }, []);

  /** Hide a field immediately. */
  const hide = useCallback((fieldId: string) => {
    if (timersRef.current[fieldId]) {
      clearTimeout(timersRef.current[fieldId]);
      delete timersRef.current[fieldId];
    }
    setRevealed((prev) => {
      const next = { ...prev };
      delete next[fieldId];
      return next;
    });
  }, []);

  /** Check if a field should be masked based on its sensitivity level. */
  const shouldMask = useCallback(
    (fieldId: string, sensitivity: SensitivityLevel): boolean => {
      // P036: 规则统一——仅 public 不掩码，internal 同样自动掩码（AGENTS.md 约定）
      if (!shouldMaskSensitivity(sensitivity)) return false;
      // Sensitive/critical (and internal) are masked unless revealed
      if (revealed[fieldId]) {
        if (Date.now() < revealed[fieldId].expiresAt) return false;
        // Expired — clean up
        hide(fieldId);
      }
      return true;
    },
    [revealed, hide],
  );

  /** Check if a field is currently revealed. */
  const isRevealed = useCallback(
    (fieldId: string): boolean => {
      if (!revealed[fieldId]) return false;
      return Date.now() < revealed[fieldId].expiresAt;
    },
    [revealed],
  );

  /** Mask a value for display. */
  const maskValue = useCallback(
    (value: string, fieldId: string, sensitivity: SensitivityLevel): string => {
      if (!shouldMask(fieldId, sensitivity)) return value;
      // TODO(产品决策): 字段类型感知的部分掩码未实现（如银行卡只显后 4 位、日期只显年份）。
      // 需字段类型注册表 + 掩码规则 DSL（见 §09 对象规范）；当前统一 8 圆点占位（P036 已收敛）。
      return MASK_PLACEHOLDER;
    },
    [shouldMask],
  );

  return { reveal, hide, shouldMask, isRevealed, maskValue };
}

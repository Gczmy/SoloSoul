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

  // P026: 过期清理移到 effect——渲染期（shouldMask）不再触发 setState，
  // 保证渲染纯净性；监听 revealed 变化，发现过期条目即 hide（触发新一轮
  // render 后无过期条目，effect 空转停止）。
  useEffect(() => {
    const now = Date.now();
    const expired: string[] = [];
    for (const [fieldId, entry] of Object.entries(revealed)) {
      if (now >= entry.expiresAt) expired.push(fieldId);
    }
    for (const fieldId of expired) hide(fieldId);
  }, [revealed, hide]);

  /** Check if a field should be masked based on its sensitivity level. */
  const shouldMask = useCallback(
    (fieldId: string, sensitivity: SensitivityLevel): boolean => {
      // P036: 规则统一——仅 public 不掩码，internal 同样自动掩码（AGENTS.md 约定）
      if (!shouldMaskSensitivity(sensitivity)) return false;
      // Sensitive/critical (and internal) are masked unless revealed.
      // 已过期条目视为未 reveal（纯判断，清理交由上述 effect）。
      if (revealed[fieldId] && Date.now() < revealed[fieldId].expiresAt) return false;
      return true;
    },
    [revealed],
  );

  /** Check if a field is currently revealed. */
  const isRevealed = useCallback(
    (fieldId: string): boolean => {
      if (!revealed[fieldId]) return false;
      return Date.now() < revealed[fieldId].expiresAt;
    },
    [revealed],
  );

  /** 剩余揭示时长（ms）：未揭示/已过期返回 0，供倒计时展示。 */
  const revealRemainingMs = useCallback(
    (fieldId: string): number => {
      const entry = revealed[fieldId];
      if (!entry) return 0;
      return Math.max(0, entry.expiresAt - Date.now());
    },
    [revealed],
  );

  /** Mask a value for display. */
  const maskValue = useCallback(
    (value: string, fieldId: string, sensitivity: SensitivityLevel): string => {
      if (!shouldMask(fieldId, sensitivity)) return value;
      // 有意保留的产品决策（P045 后续迭代项，见 docs/design_map/08_对象与模板规范.md §2.5
      // 「字段类型感知的部分掩码（规划）」——该文档为决策载体）：字段类型感知的部分掩码
      // 未实现（如银行卡只显后 4 位、日期只显年份）；当前统一 8 圆点占位。
      // 掩码逻辑一律走共享组件/本 hook，禁止在别处自行实现。
      return MASK_PLACEHOLDER;
    },
    [shouldMask],
  );

  return { reveal, hide, shouldMask, isRevealed, revealRemainingMs, maskValue };
}

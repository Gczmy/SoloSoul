import { useState, useCallback, useRef } from 'react';
import type { SensitivityLevel } from '@/stores/sensitivityStore';

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
      // Public/internal are never masked
      if (sensitivity === 'public' || sensitivity === 'internal') return false;
      // Sensitive/critical are masked unless revealed
      if (revealed[fieldId]) {
        if (Date.now() < revealed[fieldId].expiresAt) return false;
        // Expired — clean up
        hide(fieldId);
      }
      return true;
    },
    [revealed, hide]
  );

  /** Check if a field is currently revealed. */
  const isRevealed = useCallback(
    (fieldId: string): boolean => {
      if (!revealed[fieldId]) return false;
      return Date.now() < revealed[fieldId].expiresAt;
    },
    [revealed]
  );

  /** Mask a value for display. */
  const maskValue = useCallback(
    (value: string, fieldId: string, sensitivity: SensitivityLevel): string => {
      if (!shouldMask(fieldId, sensitivity)) return value;
      // Full mask for all non-public levels. TODO: support field-type-aware
      // partial masking (e.g. bank card: show last 4 digits, date: show year only).
      return '••••••••';
    },
    [shouldMask]
  );

  return { reveal, hide, shouldMask, isRevealed, maskValue };
}

import { describe, it, expect } from 'vitest';
import { isFieldValueValid } from './fieldValidators';

describe('isFieldValueValid', () => {
  it('accepts valid dates and datetimes', () => {
    expect(isFieldValueValid('date', '2024-12-31')).toBe(true);
    expect(isFieldValueValid('date', '2024-02-29')).toBe(true);
    expect(isFieldValueValid('date', '2026-08-22')).toBe(true);
    expect(isFieldValueValid('datetime', '2024-12-31T23:59')).toBe(true);
    expect(isFieldValueValid('datetime', '2026-08-22T15:05')).toBe(true);
  });

  it('rejects impossible dates', () => {
    expect(isFieldValueValid('date', '2024-02-30')).toBe(false);
    expect(isFieldValueValid('date', '2024-13-01')).toBe(false);
    expect(isFieldValueValid('date', '2024-00-10')).toBe(false);
    expect(isFieldValueValid('datetime', '2024-02-30T10:00')).toBe(false);
    expect(isFieldValueValid('datetime', '2024-12-31T24:00')).toBe(false);
  });

  it('rejects loose formats the picker never produces', () => {
    expect(isFieldValueValid('date', '2024/12/31')).toBe(false);
    expect(isFieldValueValid('date', 'December 31, 2024')).toBe(false);
    expect(isFieldValueValid('datetime', '2024/12/31 23:59')).toBe(false);
  });

  it('accepts space-separated and second-precision datetimes (parseISO displays them)', () => {
    expect(isFieldValueValid('datetime', '2024-12-31 23:59')).toBe(true);
    expect(isFieldValueValid('datetime', '2024-12-31T23:59:30')).toBe(true);
    expect(isFieldValueValid('datetime', '2024-08-22 10:30:00')).toBe(true);
  });

  it('still rejects impossible dates in every separator form', () => {
    expect(isFieldValueValid('datetime', '2024-02-30 10:00')).toBe(false);
    expect(isFieldValueValid('datetime', '2024-02-30T10:00:00')).toBe(false);
    expect(isFieldValueValid('datetime', '2024-13-01 00:00')).toBe(false);
  });

  it('passes through types without a validator', () => {
    expect(isFieldValueValid('text', 'anything')).toBe(true);
    expect(isFieldValueValid('multiline', '2024-12-31')).toBe(true);
  });
});

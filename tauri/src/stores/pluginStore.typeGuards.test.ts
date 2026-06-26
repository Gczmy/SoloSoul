import { describe, it, expect } from 'vitest';

// Type guard tests for pluginStore.ts — in a separate file to avoid
// interfering with the persist middleware's localStorage mocking in
// pluginStore.test.ts.

describe('isPluginLogLine', () => {
  it('returns false for null', async () => {
    const { isPluginLogLine } = await import('./pluginStore');
    expect(isPluginLogLine(null)).toBe(false);
  });

  it('returns false for non-object types', async () => {
    const { isPluginLogLine } = await import('./pluginStore');
    expect(isPluginLogLine('string')).toBe(false);
    expect(isPluginLogLine(42)).toBe(false);
    expect(isPluginLogLine(undefined)).toBe(false);
  });

  it('returns true for a valid PluginLogLine', async () => {
    const { isPluginLogLine } = await import('./pluginStore');
    const line = { id: 'log1', level: 'info', message: 'hello', timestamp: 1000 };
    expect(isPluginLogLine(line)).toBe(true);
  });

  it('returns false when id is missing', async () => {
    const { isPluginLogLine } = await import('./pluginStore');
    expect(isPluginLogLine({ level: 'info', message: 'hello', timestamp: 1000 })).toBe(false);
  });

  it('returns false when message is not a string', async () => {
    const { isPluginLogLine } = await import('./pluginStore');
    expect(isPluginLogLine({ id: 'log1', level: 'info', message: 123, timestamp: 1000 })).toBe(false);
  });

  it('returns false when timestamp is not a number', async () => {
    const { isPluginLogLine } = await import('./pluginStore');
    expect(isPluginLogLine({ id: 'log1', level: 'info', message: 'hello', timestamp: '1000' })).toBe(false);
  });

  it('returns false for an invalid level string', async () => {
    const { isPluginLogLine } = await import('./pluginStore');
    expect(isPluginLogLine({ id: 'log1', level: 'critical', message: 'hello', timestamp: 1000 })).toBe(false);
  });

  it('returns true with extra properties', async () => {
    const { isPluginLogLine } = await import('./pluginStore');
    const line = { id: 'log1', level: 'warn', message: 'warning', timestamp: 2000, extra: 'field' };
    expect(isPluginLogLine(line)).toBe(true);
  });
});

describe('isPluginResultPayload', () => {
  it('returns false for null', async () => {
    const { isPluginResultPayload } = await import('./pluginStore');
    expect(isPluginResultPayload(null)).toBe(false);
  });

  it('returns false for non-object', async () => {
    const { isPluginResultPayload } = await import('./pluginStore');
    expect(isPluginResultPayload('string')).toBe(false);
  });

  it('accepts type "text" with content string', async () => {
    const { isPluginResultPayload } = await import('./pluginStore');
    expect(isPluginResultPayload({ type: 'text', content: 'Hello' })).toBe(true);
  });

  it('rejects type "text" without content', async () => {
    const { isPluginResultPayload } = await import('./pluginStore');
    expect(isPluginResultPayload({ type: 'text' })).toBe(false);
  });

  it('accepts type "markdown" with content string', async () => {
    const { isPluginResultPayload } = await import('./pluginStore');
    expect(isPluginResultPayload({ type: 'markdown', content: '# Title' })).toBe(true);
  });

  it('rejects type "markdown" without content', async () => {
    const { isPluginResultPayload } = await import('./pluginStore');
    expect(isPluginResultPayload({ type: 'markdown' })).toBe(false);
  });

  it('accepts type "key_value" with title + pairs array', async () => {
    const { isPluginResultPayload } = await import('./pluginStore');
    expect(
      isPluginResultPayload({
        type: 'key_value',
        title: 'Info',
        pairs: [{ key: 'k', value: 'v' }],
      }),
    ).toBe(true);
  });

  it('rejects type "key_value" without pairs', async () => {
    const { isPluginResultPayload } = await import('./pluginStore');
    expect(isPluginResultPayload({ type: 'key_value', title: 'Info' })).toBe(false);
  });

  it('rejects type "key_value" without title', async () => {
    const { isPluginResultPayload } = await import('./pluginStore');
    expect(isPluginResultPayload({ type: 'key_value', pairs: [{ key: 'k', value: 'v' }] })).toBe(false);
  });

  it('accepts type "table" with headers + rows arrays', async () => {
    const { isPluginResultPayload } = await import('./pluginStore');
    expect(
      isPluginResultPayload({ type: 'table', headers: ['A', 'B'], rows: [['1', '2']] }),
    ).toBe(true);
  });

  it('rejects type "table" without rows', async () => {
    const { isPluginResultPayload } = await import('./pluginStore');
    expect(isPluginResultPayload({ type: 'table', headers: ['A', 'B'] })).toBe(false);
  });

  it('rejects type "table" without headers', async () => {
    const { isPluginResultPayload } = await import('./pluginStore');
    expect(isPluginResultPayload({ type: 'table', rows: [['1', '2']] })).toBe(false);
  });

  it('returns false for unknown type', async () => {
    const { isPluginResultPayload } = await import('./pluginStore');
    expect(isPluginResultPayload({ type: 'chart' })).toBe(false);
  });
});

describe('isConsentRequestEvent', () => {
  it('returns false for null/undefined', async () => {
    const { isConsentRequestEvent } = await import('./pluginStore');
    expect(isConsentRequestEvent(null)).toBe(false);
    expect(isConsentRequestEvent(undefined)).toBe(false);
  });

  it('accepts a valid consent_request event with fieldId string', async () => {
    const { isConsentRequestEvent } = await import('./pluginStore');
    const event = {
      eventType: 'consent_request',
      requestId: 'r1',
      pluginId: 'p1',
      pluginName: 'Test',
      fieldId: 'f1',
      fieldLabel: 'Field',
      sensitivityLevel: 'private',
    };
    expect(isConsentRequestEvent(event)).toBe(true);
  });

  it('rejects wrong eventType', async () => {
    const { isConsentRequestEvent } = await import('./pluginStore');
    expect(isConsentRequestEvent({ eventType: 'dialog_request', fieldId: 'f1' })).toBe(false);
  });

  it('rejects missing fieldId', async () => {
    const { isConsentRequestEvent } = await import('./pluginStore');
    expect(isConsentRequestEvent({ eventType: 'consent_request' })).toBe(false);
  });

  it('rejects fieldId that is not a string', async () => {
    const { isConsentRequestEvent } = await import('./pluginStore');
    expect(isConsentRequestEvent({ eventType: 'consent_request', fieldId: 42 })).toBe(false);
  });
});

describe('isPluginCompletedEvent', () => {
  it('returns false for null', async () => {
    const { isPluginCompletedEvent } = await import('./pluginStore');
    expect(isPluginCompletedEvent(null)).toBe(false);
  });

  it('returns false for non-object types', async () => {
    const { isPluginCompletedEvent } = await import('./pluginStore');
    expect(isPluginCompletedEvent('string')).toBe(false);
    expect(isPluginCompletedEvent(42)).toBe(false);
    expect(isPluginCompletedEvent(undefined)).toBe(false);
  });

  it('accepts an object with a numeric exitCode', async () => {
    const { isPluginCompletedEvent } = await import('./pluginStore');
    expect(isPluginCompletedEvent({ exitCode: 0 })).toBe(true);
    expect(isPluginCompletedEvent({ exitCode: -1 })).toBe(true);
    expect(isPluginCompletedEvent({ exitCode: 255 })).toBe(true);
  });

  it('rejects an object without exitCode', async () => {
    const { isPluginCompletedEvent } = await import('./pluginStore');
    expect(isPluginCompletedEvent({})).toBe(false);
  });

  it('rejects an object with non-numeric exitCode', async () => {
    const { isPluginCompletedEvent } = await import('./pluginStore');
    expect(isPluginCompletedEvent({ exitCode: '0' })).toBe(false);
    expect(isPluginCompletedEvent({ exitCode: null })).toBe(false);
    expect(isPluginCompletedEvent({ exitCode: true })).toBe(false);
  });

  it('accepts objects with extra properties', async () => {
    const { isPluginCompletedEvent } = await import('./pluginStore');
    expect(isPluginCompletedEvent({ exitCode: 0, extra: 'data', nested: { a: 1 } })).toBe(true);
  });

  it('rejects objects with no exitCode but other props', async () => {
    const { isPluginCompletedEvent } = await import('./pluginStore');
    expect(isPluginCompletedEvent({ result: 'ok' })).toBe(false);
  });
});

describe('isDialogRequestEvent', () => {
  it('returns false for null/undefined', async () => {
    const { isDialogRequestEvent } = await import('./pluginStore');
    expect(isDialogRequestEvent(null)).toBe(false);
    expect(isDialogRequestEvent(undefined)).toBe(false);
  });

  it('accepts a valid dialog_request event with requestId string', async () => {
    const { isDialogRequestEvent } = await import('./pluginStore');
    const event = {
      eventType: 'dialog_request',
      requestId: 'r1',
      pluginId: 'p1',
      pluginName: 'Test',
      jsonData: '{}',
    };
    expect(isDialogRequestEvent(event)).toBe(true);
  });

  it('rejects wrong eventType', async () => {
    const { isDialogRequestEvent } = await import('./pluginStore');
    expect(isDialogRequestEvent({ eventType: 'consent_request', requestId: 'r1' })).toBe(false);
  });

  it('rejects missing requestId', async () => {
    const { isDialogRequestEvent } = await import('./pluginStore');
    expect(isDialogRequestEvent({ eventType: 'dialog_request' })).toBe(false);
  });

  it('rejects requestId that is not a string', async () => {
    const { isDialogRequestEvent } = await import('./pluginStore');
    expect(isDialogRequestEvent({ eventType: 'dialog_request', requestId: true })).toBe(false);
  });
});

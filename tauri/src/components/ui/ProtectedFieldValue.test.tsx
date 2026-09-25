import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ProtectedFieldValue } from './ProtectedFieldValue';
import { fieldPresentationPolicy } from '@/lib/fieldPresentationPolicy';
import { setRequestSession } from '@/lib/sessionRequests';

function Harness({
  value = 'secret',
  objectId = 'one',
  accountId = 'a',
  authorize,
  copy,
}: {
  value?: string;
  objectId?: string;
  accountId?: string;
  authorize: () => Promise<boolean>;
  copy: (value: string, key: string) => void;
}) {
  return (
    <ProtectedFieldValue
      accountId={accountId}
      objectId={objectId}
      fieldId="field"
      value={value}
      policy={fieldPresentationPolicy({
        fieldId: 'field',
        definition: { sensitivityLevel: 'critical' },
      })}
      authorize={authorize}
      onCopy={copy}
    >
      {(control) => (
        <>
          <span>{control.displayValue}</span>
          <button onClick={() => void control.reveal()}>Reveal</button>
          <button onClick={() => void control.copy()}>Copy</button>
        </>
      )}
    </ProtectedFieldValue>
  );
}
beforeEach(() => setRequestSession('a'));
afterEach(() => {
  cleanup();
  setRequestSession(null);
  vi.useRealTimers();
});
describe('protected field identity and access', () => {
  it.each(['value', 'object', 'account', 'lock', 'unmount'] as const)(
    '%s change rejects late verification and copying',
    async (change) => {
      let finish!: (ok: boolean) => void;
      const authorize = vi.fn(
        () =>
          new Promise<boolean>((resolve) => {
            finish = resolve;
          }),
      );
      const copy = vi.fn();
      const { rerender, unmount } = render(<Harness authorize={authorize} copy={copy} />);
      fireEvent.click(screen.getByText('Copy'));
      if (change === 'unmount') unmount();
      else if (change === 'lock')
        act(() => {
          setRequestSession(null);
          setRequestSession('a');
        });
      else
        rerender(
          <Harness
            authorize={authorize}
            copy={copy}
            value={change === 'value' ? 'changed' : 'secret'}
            objectId={change === 'object' ? 'two' : 'one'}
            accountId={change === 'account' ? 'b' : 'a'}
          />,
        );
      await act(async () => finish(true));
      expect(copy).not.toHaveBeenCalled();
      expect(document.body.innerHTML).not.toContain('secret');
      expect(document.body.innerHTML).not.toContain('changed');
    },
  );
  it('copy/reveal share authorization, cancellation and TTL; returning to an old value does not restore access', async () => {
    vi.useFakeTimers();
    const authorize = vi.fn().mockResolvedValueOnce(false).mockResolvedValue(true),
      copy = vi.fn();
    const { rerender } = render(<Harness authorize={authorize} copy={copy} />);
    await act(async () => fireEvent.click(screen.getByText('Copy')));
    expect(copy).not.toHaveBeenCalled();
    await act(async () => fireEvent.click(screen.getByText('Reveal')));
    expect(screen.getByText('secret')).toBeInTheDocument();
    await act(async () => fireEvent.click(screen.getByText('Copy')));
    expect(copy).toHaveBeenCalledExactlyOnceWith('secret', 'field');
    expect(authorize).toHaveBeenCalledTimes(2);
    act(() => vi.advanceTimersByTime(60_001));
    expect(screen.queryByText('secret')).toBeNull();
    await act(async () => fireEvent.click(screen.getByText('Reveal')));
    rerender(<Harness authorize={authorize} copy={copy} value="other" />);
    rerender(<Harness authorize={authorize} copy={copy} />);
    expect(screen.queryByText('secret')).toBeNull();
  });
});

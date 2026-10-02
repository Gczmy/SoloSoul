import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, it, vi } from 'vitest';
vi.unmock('react-i18next');
import { I18nextProvider } from 'react-i18next';
import { PluginLogSection } from './PluginLogSection';
import i18n from '@/lib/i18n';
afterEach(async () => {
  cleanup();
  await i18n.changeLanguage('en-US');
});
it.each(['page', 'sidebar'] as const)(
  'RF320 %s renders a translated failure with no logs, and updates after language change',
  async (variant) => {
    await i18n.changeLanguage('zh-CN');
    const view = render(
      <I18nextProvider i18n={i18n}>
        <PluginLogSection logs={[]} error="PLUGIN_CONSENT_DENIED" completed variant={variant} />
      </I18nextProvider>,
    );
    expect(view.container.textContent).toContain(i18n.t('common:backend_plugin_consent_denied'));
    expect(view.container.textContent).not.toContain('PLUGIN_CONSENT_DENIED');
    if (variant === 'page') fireEvent.click(view.container.querySelector('summary')!);
    const chinese = i18n.t('common:backend_plugin_consent_denied');
    await act(() => i18n.changeLanguage('en-US'));
    const english = i18n.t('common:backend_plugin_consent_denied');
    expect(chinese).not.toBe(english);
    expect(screen.getByText(english)).toBeInTheDocument();
  },
);
it('RF320 old private execution body cannot appear in either detail variant', () => {
  for (const variant of ['page', 'sidebar'] as const) {
    const view = render(
      <PluginLogSection
        logs={[]}
        error="RF320_PRIVATE_FIELD_KEY_PATH"
        completed
        variant={variant}
      />,
    );
    expect(view.container.textContent).not.toContain('RF320_PRIVATE');
    expect(view.container.textContent).toContain(i18n.t('common:backend_plugin_execution_failed'));
    view.unmount();
  }
});

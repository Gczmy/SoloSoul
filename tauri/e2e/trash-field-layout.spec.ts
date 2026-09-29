import { expect, test, type Locator } from '@playwright/test';
import { setupTauriMock, login } from './fixtures/auth';

const longSensitiveValue =
  'RF109 synthetic sensitive value / 合成敏感值🌟 / '.repeat(6) +
  'synthetic-unbroken-value-'.repeat(5);
const responsiveValue = 'Responsive value fits wide';

for (const platform of ['android', 'ios', 'macos', 'windows']) {
  test(`${platform}: protected trash values align with reveal controls`, async ({
    page,
    hasTouch,
  }) => {
    const activate = async (target: Locator) => {
      if (hasTouch) await target.tap();
      else await target.click();
    };
    await page.setViewportSize({
      width: ['android', 'ios'].includes(platform) ? 390 : 1100,
      height: 844,
    });
    await setupTauriMock(page);
    await page.addInitScript(
      ({ platform, longSensitiveValue }) => {
        const prefs = {
          theme: 'dark',
          language: 'en-US',
          hasSeenOnboarding: true,
          autoLockTimeoutMinutes: 0,
        };
        const item = {
          id: 'trash-layout',
          originalId: 'object-layout',
          itemType: 'object',
          name: 'Layout fixture',
          deletedAt: Date.now(),
        };
        Object.assign(window, {
          __MOCK_PLATFORM__: platform,
          __E2E_MOCKS__: {
            ui_get_preferences: () => prefs,
            user_data_get_preferences: () => prefs,
            vault_check_directory: () => true,
            ocr_get_model_status: () => ({ installed: true, bundled: true }),
            sync_list_conflicts: () => [],
            object_trash_list: () => [item],
            trash_get_detail: () => ({
              ...item,
              deletedBy: 'user',
              originalLocation: 'identity',
              previewProperties: [
                ...['public', 'internal', 'sensitive', 'critical'].map((sensitivityLevel) => ({
                  key: sensitivityLevel,
                  value: 'Short test value',
                  type: 'text',
                  sensitivityLevel,
                })),
                {
                  key: 'long-sensitive',
                  value: longSensitiveValue,
                  type: 'text',
                  sensitivityLevel: 'sensitive',
                },
                {
                  key: 'responsive',
                  value: 'Responsive value fits wide',
                  type: 'text',
                  sensitivityLevel: 'public',
                },
              ],
              attachments: [],
              deletedAttachments: [],
              snapshots: [],
              childItems: [],
            }),
          },
        });
        localStorage.setItem('i18nextLng', 'en-US');
      },
      { platform, longSensitiveValue },
    );
    await login(page);
    await page.evaluate(() => {
      history.pushState(
        { ...history.state, idx: (history.state?.idx ?? 0) + 1 },
        '',
        '/settings/trash',
      );
      dispatchEvent(new PopStateEvent('popstate', { state: history.state }));
    });
    await activate(page.getByText('Layout fixture', { exact: true }));
    for (const level of ['internal', 'sensitive', 'critical']) {
      const button = page.getByRole('button', { name: new RegExp(`^${level}:`) });
      await expect(button).toBeVisible();
      const geometry = await button.evaluate((control) => {
        const text = control.parentElement!.querySelector('[data-field-value-text]')!;
        const a = text.getBoundingClientRect();
        const b = control.getBoundingClientRect();
        return {
          offset: Math.abs(a.y + a.height / 2 - b.y - b.height / 2),
          height: b.height,
          basis: getComputedStyle(control.parentElement!).flexBasis,
        };
      });
      expect(geometry.offset).toBeLessThan(1);
      expect(geometry.basis).toBe('0%');
      if (platform === 'android') expect(geometry.height).toBeGreaterThanOrEqual(48);
    }
    await expect(page.getByRole('button', { name: /^public:/ })).toHaveCount(0);
    await activate(page.getByRole('button', { name: /^sensitive:/ }));
    await expect(
      page.locator('[data-field-value-text]').filter({ hasText: 'Short test value' }),
    ).toHaveCount(2);

    const longButton = page.getByRole('button', { name: /^long-sensitive:/ });
    const longContainer = longButton.locator('..');
    const longText = longContainer.locator('[data-field-value-text]');
    await expect(longText).toHaveText('••••••••');
    const revealLabel = await longButton.getAttribute('aria-label');
    expect(revealLabel).toBeTruthy();
    await activate(longButton);
    await expect(longText).toHaveText(longSensitiveValue);
    await expect(longButton).not.toHaveAttribute('aria-label', revealLabel!);
    await expect
      .poll(() =>
        longText.evaluate((text) => {
          const style = getComputedStyle(text);
          const lineHeight = parseFloat(style.lineHeight) || parseFloat(style.fontSize) * 1.2;
          return text.getBoundingClientRect().height > lineHeight * 1.5;
        }),
      )
      .toBe(true);

    await longButton.scrollIntoViewIfNeeded();
    await expect(longButton).toBeInViewport();
    await expect(longButton).toBeEnabled();
    const longGeometry = await longButton.evaluate((control) => {
      const container = control.parentElement!;
      const text = container.querySelector('[data-field-value-text]')!;
      const row = container.parentElement!;
      const panel = container.closest('[data-macos-glass="panel"]')!;
      const buttonRect = control.getBoundingClientRect();
      const containerRect = container.getBoundingClientRect();
      const panelRect = panel.getBoundingClientRect();
      return {
        horizontalOverflow: [text, container, row, panel, document.documentElement].map(
          (element) => element.scrollWidth - element.clientWidth,
        ),
        left: buttonRect.left,
        right: buttonRect.right,
        height: buttonRect.height,
        containerLeft: containerRect.left,
        containerRight: containerRect.right,
        panelLeft: panelRect.left,
        panelRight: panelRect.right,
        viewportWidth: window.innerWidth,
      };
    });
    for (const overflow of longGeometry.horizontalOverflow) {
      expect(overflow).toBeLessThanOrEqual(1);
    }
    expect(longGeometry.left).toBeGreaterThanOrEqual(0);
    expect(longGeometry.right).toBeLessThanOrEqual(longGeometry.viewportWidth);
    expect(longGeometry.left).toBeGreaterThanOrEqual(longGeometry.containerLeft - 1);
    expect(longGeometry.right).toBeLessThanOrEqual(longGeometry.containerRight + 1);
    expect(longGeometry.left).toBeGreaterThanOrEqual(longGeometry.panelLeft - 1);
    expect(longGeometry.right).toBeLessThanOrEqual(longGeometry.panelRight + 1);
    if (platform === 'android') expect(longGeometry.height).toBeGreaterThanOrEqual(48);
    await page.screenshot({ path: test.info().outputPath(`${platform}-trash-long-value.png`) });

    // 长值重新掩码后必须回到紧凑行，不能永久停留在扩展布局。
    await activate(longButton);
    await expect(longText).toHaveText('••••••••');
    await expect(longContainer).toHaveAttribute('data-value-layout', 'inline');
    await expect(longButton).toHaveAttribute('aria-label', revealLabel!);
    expect(await longContainer.innerHTML()).not.toContain('RF109 synthetic sensitive value');

    const responsiveText = page
      .locator('[data-field-value-text]')
      .filter({ hasText: responsiveValue });
    const responsiveContainer = responsiveText.locator('..');
    const responsiveRow = responsiveContainer.locator('..');
    await responsiveRow.evaluate((row) => {
      (row as HTMLElement).style.width = '260px';
    });
    await expect(responsiveContainer).not.toHaveAttribute('data-value-layout', 'inline');
    await responsiveRow.evaluate((row) => {
      (row as HTMLElement).style.removeProperty('width');
    });
    await expect(responsiveContainer).toHaveAttribute('data-value-layout', 'inline');
    await page.screenshot({ path: test.info().outputPath(`${platform}-trash-values.png`) });
  });
}

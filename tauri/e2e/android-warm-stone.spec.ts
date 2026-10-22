import { expect, test } from '@playwright/test';
import { mountForm } from './fixtures/android-form';
for (const platform of ['android', 'windows', 'macos'])
  test(`${platform}: warm stone uses its platform surfaces`, async ({ page }) => {
    await page.setViewportSize({ width: platform === 'android' ? 390 : 1000, height: 750 });
    await mountForm(
      page,
      '/src/components/ui/Card.tsx',
      'Card',
      {
        children: 'Public card content · Warm stone surface preview',
        style: { minHeight: 220, padding: 24 },
      },
      platform,
    );
    const colors = await page.evaluate(() => {
      const s = getComputedStyle(document.documentElement);
      return {
        base: s.getPropertyValue('--bg-base').trim(),
        card: s.getPropertyValue('--bg-elevated').trim(),
        startup: s.getPropertyValue('--startup-background').trim(),
      };
    });
    expect(colors).toEqual(
      platform === 'android'
        ? { base: '#eeeae2', card: '#fffdf9', startup: '#eeeae2' }
        : { base: '#ffffff', card: '#f7f7f7', startup: '#ffffff' },
    );
    await expect(page.locator('[data-form-fixture] [data-ui-card]')).toHaveCSS(
      'background-color',
      platform === 'android' ? 'rgb(255, 253, 249)' : 'rgb(247, 247, 247)',
    );
    await page.screenshot({ path: test.info().outputPath(`${platform}-warm-stone.png`) });
  });

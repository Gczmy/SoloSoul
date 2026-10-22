import { expect, test } from '@playwright/test';
import { mountForm, noOverflow } from './fixtures/android-form';

for (const platform of ['android', 'windows', 'macos']) {
  test(`${platform}: global attachment expanded content has no extra labels or background`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: platform === 'android' ? 320 : 900, height: 700 });
    const fileName = 'PublicLongFileName_'.repeat(24) + '.txt';
    const description = 'PublicDescription_'.repeat(24) + '\nSecond line';
    const tags = ['PublicLongTag_'.repeat(24), 'b', 'c', 'd', 'e'];
    await mountForm(
      page,
      '/src/components/attachment/AttachmentFileNameBlock.tsx',
      'AttachmentFileNameBlock',
      {
        plainExpanded: true,
        fileName,
        description,
        tags,
        sizeBytes: 1234,
        createdAt: '2026-10-22',
        showTrash: false,
      },
      platform,
    );
    const host = page.locator('[data-form-fixture]');
    await expect(host.getByRole('button', { expanded: false })).toHaveCount(3);
    for (let i = 0; i < 3; i++) await host.getByRole('button', { expanded: false }).first().click();
    await expect(host.getByRole('button', { expanded: true })).toHaveCount(3);
    for (const label of ['Name', 'Description', 'Tags'])
      await expect(host.getByText(label, { exact: true })).toHaveCount(0);
    await expect(host.getByText(description, { exact: true })).toBeVisible();
    await noOverflow(page, '[data-form-fixture], [data-form-fixture-ready]');
    const backgrounds = await host
      .getByRole('button', { expanded: true })
      .evaluateAll((els) =>
        els.map((el) => [
          getComputedStyle(el).backgroundColor,
          getComputedStyle(el.parentElement!.parentElement!).backgroundColor,
        ]),
      );
    for (const pair of backgrounds) expect(pair).toEqual(['rgba(0, 0, 0, 0)', 'rgba(0, 0, 0, 0)']);
    await host.getByRole('button', { expanded: true }).first().click();
    await expect(host.getByRole('button', { expanded: false })).toHaveCount(1);
  });
}

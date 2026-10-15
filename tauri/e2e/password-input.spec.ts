import { expect, test } from '@playwright/test';
import { setupTauriMock } from './fixtures/auth';

test('password stays protected until revealed, hides on blur, and still unlocks with Enter', async ({
  page,
}) => {
  await setupTauriMock(page);
  await page.goto('/login');
  const password = page.locator('[data-login-method-region="password"] input');
  await expect(password).toHaveAttribute('type', 'password');
  await password.fill('public-test-password');
  await page.getByRole('button', { name: 'Show password', exact: true }).click();
  await expect(password).toHaveAttribute('type', 'text');
  await expect(password).toBeFocused();
  await expect(password).toHaveValue('public-test-password');

  await page.getByText('Unlock your vault to continue', { exact: true }).click();
  await expect(password).toHaveAttribute('type', 'password');
  await expect(password).toHaveValue('public-test-password');

  await password.fill('any-password');
  await password.press('Enter');
  await page.waitForURL('/');
});

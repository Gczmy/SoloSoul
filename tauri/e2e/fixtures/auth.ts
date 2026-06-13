import type { Page } from '@playwright/test';

export async function setupTauriMock(page: Page) {
  await page.addInitScript({ path: 'e2e/fixtures/tauriMock.js' });
}

export async function login(page: Page) {
  await page.goto('/login');
  await page.waitForSelector('input[type="text"]', { timeout: 10000 });
  await page.locator('input[type="text"]').fill('any-password');
  await page.locator('button[type="submit"]').click();
  await page.waitForURL('/', { timeout: 10000 });
}

export async function navigateToPlugins(page: Page) {
  await page.locator('[aria-label="Plugins"]').click();
  await page.waitForURL('/plugins', { timeout: 10000 });
}

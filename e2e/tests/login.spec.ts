import { test, expect } from '@playwright/test';

const EMAIL = process.env.E2E_USER_EMAIL || 'admin@test.com';
const PASSWORD = process.env.E2E_USER_PASSWORD || 'test123456';

test.describe('Login Flow', () => {
  test('navigates to login, enters credentials, submits, and redirects to dashboard', async ({ page }) => {
    await page.goto('/login');

    // Verify login form is visible
    await expect(page.locator('#login-email')).toBeVisible();
    await expect(page.locator('#login-password')).toBeVisible();

    // Enter credentials
    await page.locator('#login-email').fill(EMAIL);
    await page.locator('#login-password').fill(PASSWORD);

    // Submit
    await page.locator('button[type="submit"]').click();

    // Verify redirect to dashboard
    await page.waitForURL('**/dashboard', { timeout: 10_000 });
    await expect(page).toHaveURL(/\/dashboard/);
  });

  test('shows validation error on empty email', async ({ page }) => {
    await page.goto('/login');
    await page.locator('#login-email').focus();
    await page.locator('#login-email').blur();
    await expect(page.locator('#email-error')).toBeVisible();
  });

  test('shows error banner on invalid credentials', async ({ page }) => {
    await page.goto('/login');
    await page.locator('#login-email').fill('invalid@example.com');
    await page.locator('#login-password').fill('wrongpass123');
    await page.locator('button[type="submit"]').click();
    await expect(page.locator('.gi-error-banner')).toBeVisible({ timeout: 10_000 });
  });
});

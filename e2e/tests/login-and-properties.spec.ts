import { test, expect, Page } from '@playwright/test';

const CREDENTIALS = {
  email: process.env.E2E_USER_EMAIL || 'admin@test.com',
  password: process.env.E2E_USER_PASSWORD || 'test123456',
};

async function login(page: Page) {
  await page.goto('/login');
  await page.locator('#login-email').fill(CREDENTIALS.email);
  await page.locator('#login-password').fill(CREDENTIALS.password);
  await page.locator('button[type="submit"]').click();
  await page.waitForURL('**/dashboard');
}

test.describe('Login flow', () => {
  test('shows login form', async ({ page }) => {
    await page.goto('/login');
    await expect(page.locator('h1')).toContainText('Gestión Inmobiliaria');
    await expect(page.locator('#login-email')).toBeVisible();
    await expect(page.locator('#login-password')).toBeVisible();
  });

  test('validates empty fields', async ({ page }) => {
    await page.goto('/login');
    await page.locator('#login-email').focus();
    await page.locator('#login-password').focus();
    await page.locator('#login-email').blur();
    await expect(page.locator('#email-error')).toBeVisible();
  });

  test('shows error for invalid credentials', async ({ page }) => {
    await page.goto('/login');
    await page.locator('#login-email').fill('wrong@example.com');
    await page.locator('#login-password').fill('wrongpassword');
    await page.locator('button[type="submit"]').click();
    await expect(page.locator('.gi-error-banner')).toBeVisible();
  });

  test('successful login redirects to dashboard', async ({ page }) => {
    await login(page);
    await expect(page).toHaveURL(/dashboard/);
  });
});

test.describe('Property creation', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('opens property form and validates required fields', async ({ page }) => {
    await page.goto('/propiedades');
    await page.getByRole('button', { name: /nueva propiedad/i }).click();
    // Submit empty form
    await page.getByRole('button', { name: /guardar/i }).click();
    await expect(page.locator('.gi-field-error').first()).toBeVisible();
  });

  test('creates a property successfully', async ({ page }) => {
    await page.goto('/propiedades');
    await page.getByRole('button', { name: /nueva propiedad/i }).click();

    const ts = Date.now();
    await page.locator('input').filter({ has: page.locator('~ label', { hasText: 'Título' }).or(page.locator('')) }).first().waitFor();

    // Fill required fields using label associations
    const form = page.locator('form');
    const inputs = form.locator('input[type="text"]');
    // Título
    await inputs.nth(0).fill(`E2E Propiedad ${ts}`);
    // Dirección
    await inputs.nth(1).fill('Calle Prueba #123');
    // Ciudad
    await inputs.nth(2).fill('Santo Domingo');
    // Provincia
    await inputs.nth(3).fill('Distrito Nacional');
    // Precio
    await form.locator('input[type="number"]').first().fill('50000');

    await page.getByRole('button', { name: /guardar/i }).click();

    // Verify property appears in list
    await expect(page.getByText(`E2E Propiedad ${ts}`)).toBeVisible({ timeout: 10_000 });
  });
});

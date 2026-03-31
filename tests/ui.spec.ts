import { test, expect } from '@playwright/test';

test('navigation to samantha', async ({ page }) => {
  await page.goto('/');
  // The default title is often "dioxus | ⛺"
  await expect(page).toHaveTitle(/dioxus/);

  // Click the Samantha link in the navbar
  await page.getByRole('link', { name: 'Samantha' }).click();

  // Expect the URL to be /samantha
  await expect(page).toHaveURL(/\/samantha/);

  // Expect the heading to be "Samantha Editor"
  await expect(page.getByRole('heading', { name: 'Samantha Editor' })).toBeVisible();
});

test('editor layout', async ({ page }) => {
  await page.goto('/samantha');

  // Verify presence of editor and preview areas
  const editorArea = page.locator('.editor-area');
  const previewArea = page.locator('.preview-area');

  await expect(editorArea).toBeVisible();
  await expect(previewArea).toBeVisible();

  // Verify the python-editor textarea exists
  await expect(page.locator('.python-editor')).toBeVisible();
});

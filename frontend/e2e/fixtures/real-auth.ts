import type { Page } from '@playwright/test';

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000';

const TEST_USERNAME = process.env.TEST_USERNAME;
const TEST_PASSWORD = process.env.TEST_PASSWORD;

export async function login(page: Page) {
  if (!TEST_USERNAME || !TEST_PASSWORD) {
    throw new Error(
      'E2E 测试需要环境变量 TEST_USERNAME / TEST_PASSWORD'
    );
  }
  await page.goto(`${BASE_URL}/login`);
  await page.waitForSelector('input[name="username"]', { state: 'visible' });
  await page.waitForSelector('input[name="password"]', { state: 'visible' });

  await page.fill('input[name="username"]', TEST_USERNAME);
  await page.fill('input[name="password"]', TEST_PASSWORD);

  const checkbox = page.locator('.el-checkbox').first();
  const isChecked = await checkbox.locator('input').isChecked();
  if (!isChecked) {
    await checkbox.click();
  }

  await page.click('button[type="submit"]');
  await page.waitForURL(/\/(dashboard|$)/, { timeout: 15_000 });
}
import type { Page } from '@playwright/test';

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000';

export async function gotoWithRetry(page: Page, path: string): Promise<void> {
  const url = path.startsWith('http') ? path : `${BASE_URL}${path}`;
  await page.goto(url, { waitUntil: 'domcontentloaded' });

  // 检测 Vite 504（首次加载触发依赖预构建时出现）
  const bodyText = await page.locator('body').textContent({ timeout: 5_000 }).catch(() => '');
  if (bodyText && bodyText.includes('504')) {
    // 等 Vite 预构建完成后重新加载
    await page.waitForTimeout(5_000);
    await page.goto(url, { waitUntil: 'domcontentloaded' });
    // 再检查一次
    const retryText = await page.locator('body').textContent({ timeout: 5_000 }).catch(() => '');
    if (retryText && retryText.includes('504')) {
      await page.waitForTimeout(5_000);
      await page.goto(url, { waitUntil: 'networkidle' });
    }
  }
}

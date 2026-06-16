import { defineConfig, devices } from '@playwright/test'

/**
 * Playwright 配置 - P2-3 V2Table 冒烟测试
 *
 * 说明：
 * - baseURL: 沙箱 dev server 使用 vite.config.ts 的 port: 3000
 * - timeout: 单测试 30s（API mock 响应 + 页面渲染）
 * - retries: 0（冒烟测试需要一次性通过，避免无意义重试）
 * - workers: 1（共享一个 dev server，避免并发）
 * - use.headless: true（沙箱无 X server）
 */
export default defineConfig({
  testDir: './tests/views',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 0,
  workers: 1,
  reporter: 'line',
  timeout: 30_000,
  use: {
    baseURL: 'http://localhost:3000',
    headless: true,
    trace: 'off',
    screenshot: 'only-on-failure',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
})

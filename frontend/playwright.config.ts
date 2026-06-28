import { defineConfig, devices } from '@playwright/test'

/**
 * Playwright 配置 - E2E 业务流程测试套件
 *
 * TS-T-4 修复（2026-06-26 第二次审计第二优先级）：
 * 原 testDir 指向 './tests/views'（仅 5 个冒烟测试），但 17 个完整业务流程
 * E2E spec 位于 './e2e/' 目录下，导致这些 spec 完全无法被发现和运行。
 * 改为 testDir: './e2e'，让 Playwright 扫描真正的 E2E 套件目录。
 *
 * v5 批次 21 修复（2026-06-28 11-P0-2）：
 * 新增 webServer 配置，让 Playwright 自动启动 vite dev server 并等待端口就绪。
 * 原配置无 webServer，CI 中无人预先启动 dev server，导致所有 E2E 测试
 * 因 net::ERR_CONNECTION_REFUSED 而失败。本地开发者通常已手动启动 dev server，
 * 但 CI 环境必须由 Playwright 自动托管。
 *
 * 说明：
 * - testDir: ./e2e（业务流程 E2E 测试套件）
 * - baseURL: 沙箱 dev server 使用 vite.config.ts 的 port: 3000
 * - timeout: 单测试 30s（API mock 响应 + 页面渲染）
 * - retries: 0（冒烟测试需要一次性通过，避免无意义重试）
 * - workers: 1（共享一个 dev server，避免并发）
 * - use.headless: true（沙箱无 X server）
 * - webServer: CI 中自动启动 dev server（reuseExistingServer: !CI）
 *
 * 注：原 tests/views/ 下的 5 个冒烟测试（inventory/production/quality/quotation/sales）
 * 已迁移至 e2e/smoke/ 子目录统一管理；如未迁移，可通过 projects 配置额外覆盖。
 */
export default defineConfig({
  testDir: './e2e',
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
  // 自动启动 vite dev server 并等待端口就绪
  // CI 中 reuseExistingServer: false 确保使用全新实例
  // 本地开发时 reuseExistingServer: true 复用已启动的 dev server
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:3000',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
    stdout: 'pipe',
    stderr: 'pipe',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
})

import { defineConfig, devices } from '@playwright/test'

/**
 * Playwright 配置 - E2E 业务流程测试套件
 *
 * 批次 190 规则 5 修复（2026-07-08）：
 * 移除"前端独立冒烟测试"占位符策略，改为真实 E2E 测试。
 * - reporter: [['html'], ['line']] 生成可下载的 HTML 报告（规则 5）
 * - timeout: 60_000 增加单测试超时（真实后端 API 响应）
 * - webServer: 仅启动前端 dev server（后端由 CI 独立启动进程）
 */
export default defineConfig({
  testDir: './e2e',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 0,
  workers: 1,
  /// 同时生成 HTML 报告（可下载 artifact）和命令行输出
  reporter: [['html'], ['line']],
  /// 单测试 60s（真实后端 API 响应 + 页面渲染）
  timeout: 60_000,
  use: {
    baseURL: 'http://localhost:3000',
    headless: true,
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },
  /// CI 中自动启动前端 dev server（后端由 CI job 独立启动进程）
  /// 本地开发时 reuseExistingServer: true 复用已启动的 dev server
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

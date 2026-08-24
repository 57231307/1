import { defineConfig, devices } from '@playwright/test'

/**
 * Playwright 配置 - E2E 业务流程测试套件
 *
 * 批次 190 规则 5 修复（2026-07-08）：
 * 移除"前端独立冒烟测试"占位符策略，改为真实 E2E 测试。
 * - reporter: [['html'], ['line']] 生成可下载的 HTML 报告（规则 5）
 * - timeout: 60_000 增加单测试超时（真实后端 API 响应）
 *
 * 批次 262 增强（2026-07-10）：多浏览器支持
 * - 新增 firefox + webkit 项目（本地运行覆盖跨浏览器兼容性）
 * - CI 仅安装 chromium，通过 --project=chromium 限定单浏览器运行（控制 CI 时长）
 * - 本地 `npx playwright test` 默认运行所有浏览器项目
 * - 多上下文隔离 / 网络拦截 / RPA 工具见 e2e/fixtures/
 *
 * V15 Batch 487 P0-T05 修复（规则 5）：webServer 改为数组
 * - 数组配置同时启动前端 dev server + 后端二进制，实现本地+CI 一致启动
 * - 前端 webServer：reuseExistingServer: !process.env.CI（CI 中启动，本地复用）
 * - 后端 webServer：reuseExistingServer: true（总是复用）
 *   - CI 中后端由 e2e-batch.yml 独立启动（带健康检查 + 系统初始化），
 *     Playwright 复用该实例，避免端口冲突
 *   - 本地若后端未启动，Playwright 启动后端二进制；若已启动则复用
 * - 后端健康检查端点：GET /health（与 e2e-batch.yml 一致）
 */
export default defineConfig({
  testDir: './e2e',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 0,
  workers: 1,
  // 同时生成 HTML 报告（可下载 artifact）和命令行输出
  reporter: [['html'], ['line']],
  // 单测试 120s（CI 环境页面渲染 + API 响应较慢）
  timeout: 120_000,
  use: {
    baseURL: 'http://localhost:3000',
    headless: true,
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    // CI 环境 actionTimeout 30s（单个操作超时）
    actionTimeout: 30_000,
    // CI 环境导航超时 30s
    navigationTimeout: 30_000,
  },
  // webServer 数组：同时启动前端 dev server + 后端二进制
  webServer: [
    {
      command: 'npm run dev',
      url: 'http://localhost:3000',
      reuseExistingServer: true,
      timeout: 120_000,
      stdout: 'pipe',
      stderr: 'pipe',
    },
    {
      command: 'cd ../backend && ./target/release/server',
      url: 'http://localhost:8082/health',
      reuseExistingServer: true,
      timeout: 60_000,
      stdout: 'pipe',
      stderr: 'pipe',
    },
  ],
  // 项目级覆盖：smoke 可并行，flow 串行
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'firefox',
      use: { ...devices['Desktop Firefox'], fullyParallel: true, workers: 2 },
      testMatch: /smoke\/.*\.spec\.ts/,
    },
    {
      name: 'webkit',
      use: { ...devices['Desktop Safari'] },
    },
  ],
})

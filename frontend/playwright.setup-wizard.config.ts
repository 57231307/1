import { defineConfig, devices } from '@playwright/test';

/**
 * Setup 向导真实链路 E2E 专用配置（零 mock，用户指令）
 *
 * 与 CI 主配置（playwright.config.ts）的差异：
 * - baseURL 指向 3100（Setup 专用前端 dev server，VITE_PROXY_TARGET 代理到
 *   8083 独立 Setup 后端）；主配置 3000/8082 供 34 分片共享库流程测试
 * - storageState 关闭：测试对象是"未初始化系统的引导流程"，不存在可复用的
 *   登录态；登录验证在测试内真实执行（UI 填表 + API 登录）
 * - globalSetup 关闭：主配置的 globalSetup 依赖已初始化后端（登录创建分片
 *   账号），Setup 模式后端无用户体系
 * - testMatch 仅限 00-setup-wizard.spec.ts
 * - test.describe.serial 强制顺序执行：初始化是单向状态跃迁
 *   （Setup 模式 → 完整模式），测试间有严格时序依赖
 */
export default defineConfig({
  testDir: './e2e/setup-wizard',
  testMatch: /00-setup-wizard\.spec\.ts/,
  timeout: 180_000,
  expect: { timeout: 15_000 },
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: [['line']],
  use: {
    baseURL: process.env.SETUP_E2E_FRONT_BASE || 'http://localhost:3100',
    headless: true,
    actionTimeout: 30_000,
    navigationTimeout: 60_000,
    locale: 'zh-CN',
    storageState: undefined,
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
});

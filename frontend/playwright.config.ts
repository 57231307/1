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
  // 默认单 worker（本地顺序可复现）。CI e2e 分片通过 CLI `--workers` 覆盖为多 worker，
  // 让同一片内不同 spec 文件并行、文件内 `test.describe.serial` 业务流链仍保持串行，
  // 从而消除 flow 分片的串行长尾（见 .github/workflows/ci-cd.yml ci-e2e）。
  // 这里保持 fullyParallel:false 是有意的：开它会打散文件内 serial 链、引入假失败。
  workers: 1,
  // 真实登录一次，保存 cookie storageState 供所有 spec 复用（避免每 spec 独立登录触发 429）
  globalSetup: './e2e/global-setup.ts',
  // 同时生成 HTML 报告（可下载的 artifact）和命令行输出
  reporter: [['html'], ['line']],
  // 单测试 420s：ensureTestEntities 需 UI 创建 10+ 实体（仓库/产品/供应商/dye-batch/
  // dye-recipe/BOM/定制订单/色卡等），CI 慢环境下单个页面加载/登录可达 120s，
  // 300s 在后段实体（色卡 140s 实测）处耗尽导致 page closed 连锁失败
  timeout: 420_000,
  use: {
    baseURL: 'http://localhost:3000',
    headless: true,
    // el-dialog/el-select 开合逐帧动画导致 Playwright 动作性"element is not stable"点击超时;
    // 关 CSS 动画使交互确定性稳定,不改任何断言(组件已核无真实重渲染)。
    animations: 'disabled',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    // 诊断模式（用户指令：尽可能多获取测试日志便于错误分析，减少 CI 重跑次数）
    // video 仍只在失败时保留（通过片不产录屏，无诊断损失）。显式降低录屏分辨率到
    // 854x480（默认跟随 viewport 1280x720）：webm 体积随像素数近似线性下降，
    // 单条失败录屏约缩到原来的 ~47%，是失败片产物瘦身的主贡献项之一。
    // 失败诊断能力不减：trace（含 action 时间线/网络/console）与 error-context 仍在。
    video: { mode: 'retain-on-failure', size: { width: 854, height: 480 } },
    // 浏览器语言设为中文（i18n 浏览器语言协商读 navigator.language，
    // Playwright 默认 en-US 导致页面英文渲染，E2E 中文文本断言全部失败）
    locale: 'zh-CN',
    // CI 环境 actionTimeout 30s（单个操作超时）
    actionTimeout: 30_000,
    // CI 环境导航超时 30s
    navigationTimeout: 30_000,
    // 所有 spec 默认复用 globalSetup 保存的真实登录态（httpOnly cookie）
    storageState: 'e2e/.auth/storage-state.json',
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
  // 项目级覆盖：smoke 可并行，flow 串行。
  // 收集策略：排除式（testIgnore），不再用目录白名单。
  //
  // 白名单踩过的坑：这里曾经是
  // /(flow|smoke|enhanced|purchase|sales|purchase-ext|quality|finance|crm|bpm|traversal)\/.*\.spec\.ts|^[^/]*\.spec\.ts$/
  // CI 矩阵给 extras 分片新增了 ai/dashboard/fabric/inventory/mrp/production/
  // quotations/sales-ext/system 九个目录，白名单没同步，而这九个目录在 Linux 上
  // 一条分支都匹配不到 → 分片以 `Error: No tests found.` 退出码 1 直接红。
  // 本地 Windows 看不出这个差异：testMatch 匹配的是**相对 config 文件**的路径，
  // Windows 分隔符是反斜杠，第二分支 `^[^/]*\.spec\.ts$` 里的 `[^/]*` 能吃掉整条
  // `e2e\ai\01-process.spec.ts`，于是同一份配置实测收 260 文件（Windows）
  // vs 237 文件（Linux）。结论：这类"只在一种路径分隔符下成立"的锚定正则不可用。
  //
  // 现在的写法：testMatch 只认扩展名，目录级排除交给 testIgnore（优先级高于
  // testMatch，且不需要锚点，天然对两种分隔符一致）。新增测试目录无需登记，
  // 忘记登记也不会再出现"目录写了 spec 但主套件不收"的漂移。
  testIgnore: /[/\\]setup-wizard[/\\]/,
  testIgnore: /[/\\]setup-wizard[/\\]/,
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
      testMatch: /\.spec\.ts$/,
    },
    {
      name: 'firefox',
      use: { ...devices['Desktop Firefox'], fullyParallel: true, workers: 2 },
      testMatch: /smoke\/.*\.spec\.ts/,
    },
    {
      name: 'webkit',
      use: { ...devices['Desktop Safari'] },
      testMatch: /\.spec\.ts$/,
    },
  ],
})

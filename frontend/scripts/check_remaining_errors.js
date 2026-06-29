import { chromium } from 'playwright'

// 批次 28 v7 P0-2 修复：移除硬编码生产 IP + admin/admin123 凭据，改用环境变量（fail-secure）。
// 必需环境变量：TEST_BASE_URL / TEST_ADMIN_PASSWORD（TEST_ADMIN_USERNAME 默认 admin）
if (!process.env.TEST_BASE_URL) {
  console.error('ERROR: 必须设置 TEST_BASE_URL 环境变量（被测系统基础地址，如 http://localhost:3000）')
  process.exit(1)
}
if (!process.env.TEST_ADMIN_PASSWORD) {
  console.error('ERROR: 必须设置 TEST_ADMIN_PASSWORD 环境变量（管理员密码）')
  process.exit(1)
}
const BASE_URL = process.env.TEST_BASE_URL
const ADMIN_USERNAME = process.env.TEST_ADMIN_USERNAME || 'admin'
const ADMIN_PASSWORD = process.env.TEST_ADMIN_PASSWORD

const browser = await chromium.launch({ headless: true })
const context = await browser.newContext()
const page = await context.newPage()

const apiErrors = []
const consoleErrors = []

page.on('console', (msg) => {
  if (msg.type() === 'error') {
    consoleErrors.push({ page: page.url(), text: msg.text() })
  }
})

page.on('response', (response) => {
  const url = response.url()
  if (url.includes('/api/') && response.status() >= 400) {
    apiErrors.push({ page: page.url(), url: url, status: response.status() })
  }
})

// 登录
await page.goto(`${BASE_URL}/login`, { waitUntil: 'networkidle', timeout: 30000 })
await page.waitForTimeout(2000)
const usernameInput = await page.$(
  'input[type="text"], input[placeholder*="用户"], input[placeholder*="账号"]'
)
const passwordInput = await page.$('input[type="password"]')
if (usernameInput && passwordInput) {
  await usernameInput.fill(ADMIN_USERNAME)
  await passwordInput.fill(ADMIN_PASSWORD)
  const loginButton = await page.$('button[type="submit"], button:has-text("登录")')
  if (loginButton) {
    await loginButton.click()
    await page.waitForTimeout(3000)
  }
}

// 测试有问题的页面
const problemPages = [
  { url: '/product', name: '产品管理' },
  { url: '/ap', name: '应付管理' },
  { url: '/system', name: '系统管理' },
  { url: '/departments', name: '部门管理' },
  { url: '/sales-ext', name: '销售扩展' },
  { url: '/omni-audit', name: '审计' },
  { url: '/assist-accounting', name: '辅助核算' },
]

for (const p of problemPages) {
  apiErrors.length = 0
  consoleErrors.length = 0

  try {
    await page.goto(`${BASE_URL}${p.url}`, { waitUntil: 'networkidle', timeout: 15000 })
    await page.waitForTimeout(2000)

    console.log(`\n=== ${p.name} (${p.url}) ===`)
    if (apiErrors.length > 0) {
      console.log('API 错误:')
      apiErrors.forEach((e) =>
        console.log(`  [${e.status}] ${e.url.replace(/http:\/\/[^\/]+/, '')}`)
      )
    }
    if (consoleErrors.length > 0) {
      console.log('控制台错误:')
      consoleErrors.forEach((e) => console.log(`  ${e.text.substring(0, 100)}`))
    }
    if (apiErrors.length === 0 && consoleErrors.length === 0) {
      console.log('✓ 无错误')
    }
  } catch (e) {
    console.log(`✗ ${p.name}: ${e.message.substring(0, 50)}`)
  }
}

await browser.close()

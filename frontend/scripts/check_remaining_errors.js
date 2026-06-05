import { chromium } from 'playwright'

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
await page.goto('http://111.230.99.236/login', { waitUntil: 'networkidle', timeout: 30000 })
await page.waitForTimeout(2000)
const usernameInput = await page.$(
  'input[type="text"], input[placeholder*="用户"], input[placeholder*="账号"]'
)
const passwordInput = await page.$('input[type="password"]')
if (usernameInput && passwordInput) {
  await usernameInput.fill('admin')
  await passwordInput.fill('admin123')
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
    await page.goto(`http://111.230.99.236${p.url}`, { waitUntil: 'networkidle', timeout: 15000 })
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

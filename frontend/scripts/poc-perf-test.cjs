/**
 * el-table-v2 POC 自动化性能测试脚本
 *
 * 用法: node scripts/poc-perf-test.cjs
 * 前提: 已通过 vite preview 启动预览服务(默认 5182)
 */
const { chromium } = require('playwright')
const fs = require('fs')
const path = require('path')

const BASE_URL = process.env.POC_URL || 'http://127.0.0.1:5182'
const SCREENSHOT_DIR = path.resolve(__dirname, '../docs/poc')

if (!fs.existsSync(SCREENSHOT_DIR)) {
  fs.mkdirSync(SCREENSHOT_DIR, { recursive: true })
}

async function run() {
  const browser = await chromium.launch({ headless: true })
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    locale: 'zh-CN',
  })
  const page = await context.newPage()

  // 收集 console 错误
  const consoleErrors = []
  page.on('pageerror', (err) => consoleErrors.push('pageerror: ' + err.message))
  page.on('console', (msg) => {
    if (msg.type() === 'error') {
      consoleErrors.push('console.error: ' + msg.text())
    }
  })

  // 通过 hash 跳转,绕开 history 路由刷新
  const targetUrl = `${BASE_URL}/index.html#/inventory-poc`
  console.log('[POC] 打开页面:', targetUrl)

  // 注入 performance observer
  await page.addInitScript(() => {
    window.__poc_perf__ = {
      firstRenderMs: 0,
      fpsSamples: [],
      memorySamples: [],
    }
    const origRAF = window.requestAnimationFrame
    let frames = 0
    let last = performance.now()
    const tick = (t) => {
      frames += 1
      if (t - last >= 1000) {
        const fps = (frames * 1000) / (t - last)
        window.__poc_perf__.fpsSamples.push({ t: Date.now(), fps })
        frames = 0
        last = t
      }
      return origRAF.call(window, tick)
    }
    origRAF.call(window, tick)
    // 内存采样
    setInterval(() => {
      const perf = performance
      if (perf && perf.memory) {
        window.__poc_perf__.memorySamples.push({
          t: Date.now(),
          mb: perf.memory.usedJSHeapSize / 1024 / 1024,
        })
      }
    }, 500)
  })

  const t0 = Date.now()
  await page.goto(targetUrl, { waitUntil: 'load', timeout: 60000 })
  await page.waitForLoadState('networkidle', { timeout: 30000 }).catch(() => {})

  // 等待表格渲染
  await page.waitForSelector('.el-table-v2', { timeout: 30000 })
  // 等待行数据出现
  await page.waitForFunction(
    () => document.querySelectorAll('.el-table-v2__row, [class*="row"]').length > 0,
    { timeout: 30000 },
  )

  const firstPaintMs = Date.now() - t0
  console.log('[POC] 首次绘制到表格就绪:', firstPaintMs, 'ms')

  // 截图 - 初始状态
  await page.screenshot({
    path: path.join(SCREENSHOT_DIR, 'poc-initial.png'),
    fullPage: false,
  })

  // 测试滚动性能
  const scrollResults = []
  for (let i = 0; i < 3; i++) {
    const t1 = Date.now()
    await page.evaluate(() => {
      const scroller = document.querySelector('.el-table-v2__viewport')
      if (scroller) {
        const target = 4000 * (1 + 1) // 模拟滚动到中段
        scroller.scrollTop = target
      }
    })
    // 等待 1.5 秒采样窗口
    await page.waitForTimeout(1500)
    const t2 = Date.now()
    scrollResults.push({ round: i + 1, scrollMs: t2 - t1 })
  }

  // 滚动到末尾
  await page.evaluate(() => {
    const scroller = document.querySelector('.el-table-v2__viewport')
    if (scroller) {
      scroller.scrollTop = scroller.scrollHeight
    }
  })
  await page.waitForTimeout(1000)
  await page.screenshot({
    path: path.join(SCREENSHOT_DIR, 'poc-scrolled.png'),
    fullPage: false,
  })

  // 测试筛选
  await page.fill('input[placeholder="产品编码/名称"]', 'P000100')
  await page.waitForTimeout(800)
  await page.screenshot({
    path: path.join(SCREENSHOT_DIR, 'poc-filtered.png'),
    fullPage: false,
  })

  // 读取应用内指标
  const appMetrics = await page.evaluate(() => {
    return {
      firstRenderMs: window.__poc_perf__ ? 0 : 0,
      perf: window.__poc_perf__ || null,
      tableRowCount: document.querySelectorAll('.el-table-v2__row, [class*="table-v2"][class*="row"]').length,
      renderedDomNodes: document.querySelectorAll('*').length,
    }
  })

  // 读取页面顶部 performance 指标
  const pageMetrics = await page.evaluate(() => {
    const metrics = document.querySelectorAll('.metric-value')
    const result = {}
    metrics.forEach((m) => {
      const label = m.previousElementSibling?.textContent || ''
      result[label] = m.textContent
    })
    return result
  })

  // 提取 perf 摘要
  const perf = appMetrics.perf || {}
  const fpsValues = (perf.fpsSamples || []).map((s) => s.fps)
  const memValues = (perf.memorySamples || []).map((s) => s.mb)
  const avgFps = fpsValues.length ? fpsValues.reduce((a, b) => a + b, 0) / fpsValues.length : 0
  const maxMem = memValues.length ? Math.max(...memValues) : 0

  const report = {
    timestamp: new Date().toISOString(),
    url: targetUrl,
    dataCount: 10000,
    firstPaintMs,
    avgFps: Number(avgFps.toFixed(2)),
    maxMemoryMB: Number(maxMem.toFixed(2)),
    visibleRowCount: appMetrics.tableRowCount,
    totalDomNodes: appMetrics.renderedDomNodes,
    pageMetrics,
    scrollRounds: scrollResults,
    fpsSamples: (perf.fpsSamples || []).slice(-10),
    memorySamples: (perf.memorySamples || []).slice(-10),
    consoleErrors,
  }

  console.log('\n========== POC 性能测试报告 ==========')
  console.log(JSON.stringify(report, null, 2))
  console.log('=========================================\n')

  const reportPath = path.join(SCREENSHOT_DIR, 'poc-perf-data.json')
  fs.writeFileSync(reportPath, JSON.stringify(report, null, 2))
  console.log('[POC] 报告已写入:', reportPath)

  await browser.close()
  return report
}

run()
  .then((r) => {
    process.exit(r.consoleErrors.length === 0 ? 0 : 1)
  })
  .catch((err) => {
    console.error('[POC] 测试失败:', err)
    process.exit(2)
  })

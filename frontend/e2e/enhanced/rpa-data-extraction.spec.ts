/**
 * RPA / 爬虫类数据提取测试
 *
 * 批次 262：验证 RPA 类表单自动化与数据提取能力。
 *
 * 测试范围：
 * - 表格数据提取：爬虫类批量收集表格行数据
 * - 表单自动化：RPA 类批量表单字段定位与填充
 * - 按钮自动化：按文本定位并点击按钮
 * - 请求观察：记录 API 请求供断言（爬虫类请求采集）
 * - RPA 流程录制：记录操作时间戳供性能分析
 *
 * 设计说明：
 * - 真实后端：`applyAuthMocks` 是 2026-09-09 去 mock 时保留的旧函数名，函数体已是
 *   真实 API 登录并注入 cookie（不拦截任何路由），故本文件的断言打的是真库真接口；
 *   原先这里写着"使用 mock 模式（不依赖真实后端数据）"，属误导性的过时注释。
 * - 通过 fixtures/rpa.ts 与 fixtures/network.ts 的工具函数
 */
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { observeRequests } from '../fixtures/network';
import {
  createRpaRecorder,
  extractTableData,
  autoClickButton,
  waitForTableLoaded,
} from '../fixtures/rpa';

test.describe('RPA：表格数据提取（爬虫类）', () => {
  test.beforeEach(async ({ context }) => {
    await applyAuthMocks(context);
  });

  test('提取表格行数据结构', async ({ page }) => {
    await page.goto('/sales');
    await waitForTableLoaded(page);

    // 提取表格数据（爬虫类批量收集）
    const rows = await extractTableData(page);

    // 原断言只有 `expect(Array.isArray(rows)).toBe(true)`：extractTableData 的返回类型
    // 就是 string[][]，恒为数组 ⇒ 零断言价值（空表也算通过）。改为断真实内容。
    expect(rows.length, '销售列表未提取到任何数据行（表格空态或被筛选清空）').toBeGreaterThan(0);
    for (const row of rows) {
      expect(
        row.some(cell => cell.length > 0),
        `存在整行空白的数据行，提取选择器与表格实现不匹配：${JSON.stringify(row)}`
      ).toBe(true);
    }
  });

  test('翻页后重新提取表格数据', async ({ page }) => {
    await page.goto('/sales');
    await waitForTableLoaded(page);

    const firstPageRows = await extractTableData(page);

    // 尝试翻到下一页
    const nextBtn = page.locator('.el-pagination .btn-next').first();
    if (await nextBtn.isVisible()) {
      await nextBtn.click();
      await waitForTableLoaded(page);

      const secondPageRows = await extractTableData(page);
      // 翻页真实生效：第二页必须有数据且内容与第一页不同
      //（原来只断"是数组"，翻页按钮点了但页面没变也算通过）
      expect(secondPageRows.length, '点击下一页后未提取到任何数据行').toBeGreaterThan(0);
      expect(JSON.stringify(secondPageRows), '翻页后内容与第一页完全相同，说明分页未生效').not.toBe(
        JSON.stringify(firstPageRows)
      );
    }

    expect(firstPageRows.length, '首页未提取到任何数据行（表格空态或加载未完成）').toBeGreaterThan(
      0
    );
  });
});

test.describe('RPA：表单自动化', () => {
  test.beforeEach(async ({ context }) => {
    await applyAuthMocks(context);
  });

  test('按文本定位按钮', async ({ page }) => {
    await page.goto('/sales');
    await waitForTableLoaded(page);

    // 验证可按文本定位"新建"按钮（不实际点击，仅验证可见）
    const newBtn = page.locator('button:has-text("新建")').first();
    await expect(newBtn).toBeVisible({ timeout: 30_000 });
  });

  test('autoClickButton 工具函数可用', async ({ page }) => {
    await page.goto('/sales');
    await waitForTableLoaded(page);

    // 验证 autoClickButton 函数可调用（按钮不存在时应有合理超时）
    await autoClickButton(page, '搜索', { timeout: 15_000 });
  });
});

test.describe('RPA：请求观察（爬虫类请求采集）', () => {
  test.beforeEach(async ({ context }) => {
    await applyAuthMocks(context);
  });

  test('观察 API 请求记录', async ({ page }) => {
    // 启动请求观察器
    const observer = observeRequests(page, '**/api/v1/erp/**');
    await observer.start();

    try {
      await page.goto('/sales');
      await waitForTableLoaded(page);

      // 收集观察到的请求
      const requests = await observer.collect();

      // 原写法是 `if (requests.length > 0) { ...断言... }`：一条请求都没抓到时
      // 整个用例零断言通过，正是"条件成立才断言"的假绿形态。
      // 页面加载必然请求 ERP 接口（列表数据来自后端），抓不到就是真失败。
      expect(
        requests.length,
        '访问 /sales 未捕获到任何 /api/v1/erp 请求，说明列表未真实取数或观察器未生效'
      ).toBeGreaterThan(0);
      for (const req of requests) {
        expect(req.url, `捕获到非 ERP 请求：${req.url}`).toContain('/api/v1/erp/');
        expect(typeof req.method, `请求缺少 method：${JSON.stringify(req)}`).toBe('string');
        expect(
          req.status,
          `请求未拿到响应状态码（可能被中止）：${req.url} ${JSON.stringify(req)}`
        ).toBeGreaterThan(0);
      }
    } finally {
      await observer.stop();
    }
  });

  test('请求观察器可停止', async ({ page }) => {
    const observer = observeRequests(page, '**/api/v1/erp/**');
    await observer.start();

    await page.goto('/sales');
    await waitForTableLoaded(page);

    const beforeStop = await observer.collect();
    await observer.stop();

    // 停止后不应再收集新请求
    await page.reload();
    await waitForTableLoaded(page);

    const afterStop = await observer.collect();
    // 停止后请求数不应增加（或仅增加停止前已 pending 的）
    expect(afterStop.length).toBeLessThanOrEqual(beforeStop.length + 1);
  });
});

test.describe('RPA：流程录制（性能基准）', () => {
  test.beforeEach(async ({ context }) => {
    await applyAuthMocks(context);
  });

  test('流程录制器记录时间戳', async ({ page }) => {
    const recorder = createRpaRecorder();

    await page.goto('/sales');
    recorder.mark('页面加载完成');

    await waitForTableLoaded(page);
    recorder.mark('表格加载完成');

    const report = recorder.report();
    const total = recorder.total();

    // 验证录制器记录了 2 个标记
    expect(report).toHaveLength(2);
    expect(report[0].label).toBe('页面加载完成');
    expect(report[1].label).toBe('表格加载完成');
    // report() 的 elapsed 语义（见 fixtures/rpa.ts）：首项为绝对偏移，其余为相邻标记差值。
    // 断言每个耗时是有限非负数，而非仅 >=0（NaN/Infinity 也能骗过 toBeGreaterThanOrEqual）。
    for (const row of report) {
      expect(Number.isFinite(row.elapsed), `elapsed 应为有限数：${row.elapsed}`).toBe(true);
      expect(row.elapsed).toBeGreaterThanOrEqual(0);
    }
    // 分段耗时累加 = 末个标记的墙钟偏移，必然 ≤ 在其之后测得的 total()，
    // 该一致性可暴露 elapsed 计算错位/重复计数；total 为正数说明确有耗时。
    const sumOfSegments = report.reduce((acc, row) => acc + row.elapsed, 0);
    expect(sumOfSegments).toBeGreaterThan(0);
    expect(sumOfSegments).toBeLessThanOrEqual(total);
  });
});

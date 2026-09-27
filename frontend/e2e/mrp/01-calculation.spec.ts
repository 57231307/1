// MRP 计算 E2E 测试
// 创建时间: 2026-08-19
// 覆盖范围：MRP 计算执行 → 结果查看 → 建议采购
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCallRaw, ensureTestEntities } from '../flow/helpers';
import { pickSelectIn, formItemByExactLabel } from '../flow/ui-helpers';

test.describe('MRP 计算', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('进入 MRP 计算页面', async ({ page }) => {
    await page.goto('/mrp');
    await expect(page.getByRole('heading', { name: /MRP/ })).toBeVisible({ timeout: 30000 });
  });

  test('MRP 计算可执行', async ({ page }) => {
    // 真实造数：补前置实体，确保 MRP 产品下拉有可参与运算的产品。
    // 原实现 `if (await calcBtn.isVisible())` + 断言 `计算完成|计算中`：
    // 表单校验未过时根本不发请求，文案也永不匹配；可见性分支一旦不成立即零断言假绿。
    await ensureTestEntities(page);

    const products = await apiCallRaw<unknown>(page, 'GET', '/production/mrp/products?keyword=E2E');
    if (!Array.isArray(products)) {
      throw new Error(
        `/production/mrp/products 响应 data 不是数组，实际：${JSON.stringify(products).slice(0, 200)}`
      );
    }
    expect(products.length, '造数后 MRP 产品下拉仍为空，属数据/端点缺陷').toBeGreaterThan(0);

    await page.goto('/mrp');

    // 硬断言（Tier A）：MRP 页必然提供"开始计算"按钮
    const calcBtn = page.getByRole('button', { name: /开始计算/ });
    await expect(calcBtn, 'MRP 页面应渲染"开始计算"按钮').toBeVisible({ timeout: 30000 });

    // 产品为 remote el-select（filterable remote，:remote-method=searchProducts）：旧写法
    // getByRole('combobox').click() 命中只读内层 input，被 placeholder「请输入产品名称搜索」拦
    // pointer events → click 超时；即便点开，pressSequentially 也未点外层 wrapper 稳定触发 remote。
    // 改用冻结 helper：以计算参数表单容器（aria-label=MRP 计算参数表单）为 root + 精确 label
    // 「产品选择」锚定，点外层 .el-select__wrapper → keyboard.type('E2E') 触发远程搜索 → 选首项。
    const calcForm = page.getByLabel('MRP 计算参数表单');
    await pickSelectIn(calcForm, page, '产品选择', { query: 'E2E', index: 0 });

    // 需求日期为 el-date-picker：定位其真 input，fill 后 Enter 确认（Escape 会清空 → 校验失败）。
    const demandDate = formItemByExactLabel(calcForm, '需求日期').locator('input').first();
    await demandDate.click();
    await demandDate.fill('2026-09-30');
    await page.keyboard.press('Enter');

    await calcBtn.click();
    await expect(page.getByText(/计算成功/)).toBeVisible({ timeout: 30000 });
    await expect(page.getByText(/物料需求列表/)).toBeVisible({ timeout: 30000 });
  });

  test('MRP 历史页面可正常加载', async ({ page }) => {
    await page.goto('/mrp/history');
    await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
  });
});

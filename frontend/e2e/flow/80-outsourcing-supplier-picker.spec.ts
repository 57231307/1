import { test, expect, type Page, type Locator } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  trackPageHealth,
  assertPageHealthy,
  BROWSER_NETWORK_NOISE,
  tryCleanup,
} from './helpers';
import {
  safeGoto,
  openDialog,
  formItemByExactLabel,
  openFieldDropdownByLabel,
  dropdownItems,
  fillFieldByLabel,
  getFieldValue,
} from './ui-helpers';

// 覆盖前端改动：委外新建单“供应商/加工商”由裸手输 supplier_id 改为可搜索 el-select。
// 用 UI 真实操作锁定：造数保证候选存在、输入部分字符命中选项、点选后落库 supplier_id 正确、
// 以及不选供应商被必填拦截。集成侧走真实后端建单与回读，不 mock。

const outsourcingOrderPath = '/production/outsourcing-orders';
const supplierPath = '/purchase/suppliers';

type SupplierRow = {
  id?: number;
  supplier_name?: string;
  supplier_code?: string;
};

// 建单对话框各表单项的精确中文 label（取自 locales/zh-CN.ts outsourcing 段，逐字符一致）
const LABEL_SUPPLIER = '供应商/加工商';
const LABEL_ORDER_NO = '委外单号';
const LABEL_ISSUE_DATE = '发出日期';
const LABEL_ISSUE_QTY = '发出数量';
const LABEL_MATERIAL_COST = '材料成本';
const LABEL_PROCESSING_FEE = '加工费';
const LABEL_FREIGHT_FEE = '运费';
const LABEL_TAX_AMOUNT = '税额';

async function fillNonSupplierRequired(dialog: Locator, page: Page): Promise<void> {
  const today = new Date().toISOString().slice(0, 10);
  await fillFieldByLabel(dialog, page, LABEL_ISSUE_DATE, today);
  await formItemByExactLabel(dialog, LABEL_ISSUE_DATE).locator('input').first().press('Enter');
  await page.waitForTimeout(200);
  for (const [label, value] of [
    [LABEL_ISSUE_QTY, '100'],
    [LABEL_MATERIAL_COST, '500'],
    [LABEL_PROCESSING_FEE, '100'],
    [LABEL_FREIGHT_FEE, '50'],
    [LABEL_TAX_AMOUNT, '10'],
  ] as const) {
    // 数值项是纯 el-input 加数字校验，label 锚定的 .el-input input 即录入框本身。
    // fill 后按 Tab 让显示收敛，再读回真实值，确认模型确实收到该值而非只填了 DOM。
    const numInput = formItemByExactLabel(dialog, label).locator('.el-input input').first();
    await numInput.fill(value);
    await numInput.press('Tab');
    await expect(numInput, `${label} 录入框应保留手写值 ${value}`).toHaveValue(value);
    await page.waitForTimeout(120);
  }
}

test.describe('委外新建单供应商可搜索选择器端到端', () => {
  let createdSupplierId: number | undefined;
  let createdSupplierName = '';
  let createdSupplierCode = '';
  let createdOrderId: number | undefined;

  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test.afterEach(async ({ page }) => {
    // 先删委外单（草稿态可删），再删造的供应商，避免外键残留
    if (createdOrderId !== undefined) {
      await tryCleanup(page, 'DELETE', `${outsourcingOrderPath}/${createdOrderId}`, '清理委外单');
      createdOrderId = undefined;
    }
    if (createdSupplierId !== undefined) {
      await tryCleanup(page, 'DELETE', `${supplierPath}/${createdSupplierId}`, '清理供应商');
      createdSupplierId = undefined;
    }
  });

  test('正向：输入名称部分字符筛出候选、点选后建单并回读 supplier_id 正确', async ({ page }) => {
    const collector = trackPageHealth(page);

    // 1. 用唯一名称与唯一 18 位信用代码造一个可用供应商，保证选择器有候选
    const uniq = `${Date.now()}${Math.floor(Math.random() * 1000)}`;
    createdSupplierName = `E2E委外供应商${uniq}`;
    const creditCode = `91${uniq.slice(0, 14)}X`.slice(0, 18).padEnd(18, '0');
    await apiCall<{ id?: number }>(page, 'POST', supplierPath, {
      supplier_name: createdSupplierName,
      supplier_short_name: 'E2E委外',
      contact_phone: '13800000001',
      credit_code: creditCode,
    });

    // 按唯一名称回读，取得后端落库的真实主键与自动取号的编码（不信任响应体形状）
    const list = await apiCall<{ items?: SupplierRow[] }>(
      page,
      'GET',
      `${supplierPath}?keyword=${encodeURIComponent(createdSupplierName)}&page=1&page_size=50`
    );
    const row = (list.data?.items ?? []).find(r => r.supplier_name === createdSupplierName);
    expect(row, `建供应商后应能按唯一名称回读命中：${createdSupplierName}`).toBeTruthy();
    expect(row!.id, '回读供应商行应含主键 id').toBeTruthy();
    createdSupplierId = row!.id;
    createdSupplierCode = row!.supplier_code ?? '';

    // 2. 导航到委外管理页并断言页面健康
    await safeGoto(page, '/outsourcing');
    await page.waitForTimeout(500);
    await assertPageHealthy(page, collector, {
      consoleNoisePatterns: BROWSER_NETWORK_NOISE,
      label: '委外管理页',
    });

    // 3. 打开新建委外单对话框
    const dialog = await openDialog(page, '新建委外单');

    // 读取对话框自动生成的唯一委外单号（回读落库时按它命中，不依赖创建响应体）
    const orderNo = await getFieldValue(dialog, LABEL_ORDER_NO);
    expect(orderNo, '新建单应自动生成委外单号').not.toBe('');

    // 4. 在供应商选择器输入该供应商名的部分字符，断言下拉出现且能筛出该选项（filter 生效）
    const partial = createdSupplierName.slice(3);
    // helper 按 LABEL_SUPPLIER 精确锚定 form-item、点 wrapper 输入触发词，并把面板钉定到
    // 该字段自己名下返回（替代旧「openOnly + 任取最后一个可见面板」）
    const dropdown = await openFieldDropdownByLabel(dialog, page, LABEL_SUPPLIER, {
      query: partial,
    });
    await expect(dropdown, '输入部分名称后供应商下拉应可见').toBeVisible();
    const matchedOption = dropdownItems(dropdown).filter({ hasText: partial });
    await expect(
      matchedOption.first(),
      `下拉应出现含部分名称「${partial}」的供应商选项`
    ).toBeVisible();
    // 唯一名称保证过滤后仅剩目标一项，证明确实按输入词收窄了候选而非全量展示
    expect(
      await dropdownItems(dropdown).count(),
      '唯一部分名称应把候选过滤到 1 项'
    ).toBe(1);

    // 5. 点选该供应商
    await matchedOption.first().click();
    const expectedLabel = createdSupplierCode
      ? `${createdSupplierName}（${createdSupplierCode}）`
      : createdSupplierName;
    await expect(
      formItemByExactLabel(dialog, LABEL_SUPPLIER)
        .locator('.el-select__selected-item.el-select__placeholder:not(.is-transparent)')
        .first(),
      '选中供应商应回显为名称加编码的选项文案'
    ).toHaveText(expectedLabel);

    // 6. 补齐其余必填项并提交建单
    await fillNonSupplierRequired(dialog, page);
    await dialog.getByRole('button', { name: '保存' }).click();
    await expect(page.getByText('委外单已创建'), '应提示建单成功').toBeVisible({ timeout: 15_000 });

    // 7. 由后端按唯一委外单号回读，交叉校验持久化的 supplier_id 等于造的供应商主键
    const detail = await apiCallRaw<{ id?: number; supplier_id?: number }>(
      page,
      'GET',
      `${outsourcingOrderPath}/by-no/${encodeURIComponent(orderNo)}`
    );
    expect(detail.id, `按委外单号 ${orderNo} 应回读到新建委外单`).toBeTruthy();
    createdOrderId = detail.id;
    expect(
      detail.supplier_id,
      `新建委外单落库 supplier_id 应等于所选供应商主键（期望 ${createdSupplierId} 实际 ${detail.supplier_id}）`
    ).toBe(createdSupplierId);
  });

  test('负向：不选供应商直接提交被必填拦截且不建单', async ({ page }) => {
    trackPageHealth(page);
    await safeGoto(page, '/outsourcing');
    await page.waitForTimeout(500);

    const dialog = await openDialog(page, '新建委外单');

    // 除供应商外全部必填项填满，单独把供应商留空，精准命中供应商必填门
    await fillNonSupplierRequired(dialog, page);
    await dialog.getByRole('button', { name: '保存' }).click();

    // 前端必填校验拦下：出现必填提示且不落库，对话框保持打开
    await expect(page.getByText('请填写必填项'), '缺供应商应弹必填校验提示').toBeVisible({
      timeout: 5_000,
    });
    await expect(page.getByText('委外单已创建'), '缺供应商不应提示建单成功').toHaveCount(0);
    await expect(dialog, '缺供应商被拦截后对话框应保持打开（未提交）').toBeVisible();
  });
});

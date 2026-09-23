// 采购 E2E 套件 — 10 采购退货（专用页 /purchase-return）
// 迁移来源：e2e/purchase-ext/03-return.spec.ts（枢纽 Tab 已删除）
//   03-01 Tab 加载        → 10-01
//   03-02 新建采购退货单  → 10-02
//
// 状态机真值（backend/src/models/status/purchase_inventory.rs 的 purchase_return，全小写：
//   draft/submitted/approved/rejected）：create_return 写 draft（services/purchase_return_service.rs:127）。
// 建单必填（CreatePurchaseReturnRequest）：supplier_id、reason_type、return_date 为非 Option，
//   order_id/receipt_id/warehouse_id 为 Option；前端表单把“采购订单”作为必填项驱动，
//   选中订单后供应商由 handleOrderChange 自动派生（usePrRtn.ts:341）。
// 退货原因取值见 constants/return-reason.ts，value 为中文业务词，落库原值即中文（此处取“色差”）。
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCallRaw, ensureTestEntities, tryCleanup } from '../flow/helpers';
import type { Page } from '@playwright/test';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** 展开可见 el-select 并点选第一个选项（用于 header 内的 el-form-item 选择器） */
async function pickHeaderFirstOption(page: Page, labelText: string): Promise<void> {
  const dlg = page.locator('.el-dialog:visible');
  await dlg
    .locator('.el-form-item')
    .filter({ hasText: labelText })
    .first()
    .locator('.el-select')
    .click();
  await page
    .locator('.el-select-dropdown:visible .el-select-dropdown__item')
    .first()
    .click();
}

/** 展开可见 el-select 并点选指定文案选项（用于固定词表，如退货原因） */
async function pickHeaderOptionByText(page: Page, labelText: string, optionText: string): Promise<void> {
  const dlg = page.locator('.el-dialog:visible');
  await dlg
    .locator('.el-form-item')
    .filter({ hasText: labelText })
    .first()
    .locator('.el-select')
    .click();
  await page
    .locator('.el-select-dropdown:visible .el-select-dropdown__item')
    .filter({ hasText: optionText })
    .first()
    .click();
}

test.describe('10 采购退货', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
    await ensureTestEntities(page);
  });

  test('10-01 进入采购退货列表页', async ({ page }) => {
    await page.goto('/purchase-return');
    await expect(page.getByText('采购退货').first()).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('button', { name: '新建退货单' })).toBeVisible();
    await expect(page.locator('.el-table')).first().toBeVisible({ timeout: 30000 });
  });

  test('10-02 新建采购退货单（UI 建单 → 详情回读 + 列表按单号检索确认落库）', async ({ page }) => {
    await page.goto('/purchase-return');
    await expect(page.getByRole('button', { name: '新建退货单' })).toBeVisible();
    await page.getByRole('button', { name: '新建退货单' }).click();
    const dlg = page.locator('.el-dialog:visible');
    await expect(dlg).toBeVisible({ timeout: 30000 });

    // 采购订单：选中后供应商自动派生（后端 CreatePurchaseReturnRequest.supplier_id 非 Option）
    await pickHeaderFirstOption(page, '采购订单');

    // 退货日期（该 date-picker 带 value-format=YYYY-MM-DD，填入即字符串，无时区退化）
    const dateInput = dlg
      .locator('.el-form-item')
      .filter({ hasText: '退货日期' })
      .first()
      .locator('input')
      .first();
    await dateInput.click();
    await dateInput.fill('2026-08-19');
    await page.keyboard.press('Enter');

    // 原因类型（固定中文词表，取“色差”）+ 退货原因详情
    await pickHeaderOptionByText(page, '原因类型', '色差');
    await dlg
      .locator('.el-form-item')
      .filter({ hasText: '退货原因' })
      .first()
      .locator('textarea')
      .first()
      .fill('E2E 迁移用例：色差退货');

    // 退货明细：选中订单后已自动生成一行，补一行并给首个产品赋值（quantity 默认 1）
    await dlg.getByRole('button', { name: '添加明细' }).click();
    const itemSelect = dlg.locator('.el-table .el-select').last();
    await itemSelect.click();
    await page
      .locator('.el-select-dropdown:visible .el-select-dropdown__item')
      .first()
      .click();

    const createdResp = page
      .waitForResponse(
        res =>
          res.request().method() === 'POST' &&
          res.url().includes('/purchase/returns') &&
          !res.url().includes('/items'),
        { timeout: 30000 }
      )
      .catch(() => null);

    await dlg.getByRole('button', { name: '确定' }).click();
    await expect(page.getByText('创建成功')).toBeVisible({ timeout: 30000 });

    const resp = await createdResp;
    expect(resp, '未捕获到建单 POST 响应').not.toBeNull();
    const body = (await resp!.json()) as {
      data: {
        id: number;
        return_no: string;
        supplier_id: number;
        reason_type: string;
        return_status: string;
      };
    };
    const created = body.data;
    expect(created?.id, '建单响应未返回 id').toBeTruthy();
    CLEANUP.push({ path: `/purchase/returns/${created.id}`, label: 'purchase_return' });

    // 详情回读：状态 draft、原因类型逐字等于落库中文“色差”、供应商为真实外键
    const detail = await apiCallRaw<{
      id: number;
      return_no: string;
      return_status: string;
      reason_type: string;
      supplier_id: number;
    }>(page, 'GET', `/purchase/returns/${created.id}`);
    expect(detail.id).toBe(created.id);
    expect(detail.return_status, '新建退货单应为 draft 态').toBe('draft');
    expect(detail.reason_type, '退货原因类型应逐字落库为中文“色差”').toBe('色差');
    expect(detail.supplier_id, '供应商应为后端派生的真实 id（>0）').toBeGreaterThan(0);

    // 列表检索回读：后端 list 支持 keyword 匹配 return_no（PaginatedResponse ⇒ data.items）
    const list = await apiCallRaw<{ items: Array<{ id: number; return_no: string }>; total: number }>(
      page,
      'GET',
      `/purchase/returns?keyword=${created.return_no}&page=1&page_size=20`
    );
    expect(
      list.items.some(r => r.id === created.id),
      `列表按单号 ${created.return_no} 检索未命中新建退货单`
    ).toBe(true);
  });
});

// 报价单 E2E 套件 — 01 报价单创建
// 创建时间: 2026-08-19
// 覆盖范围：报价单创建（含明细行、币种、价格条款）→ 提交审批
import { test, expect } from '../diagnose-fixture';
import { applyAuthMocks } from '../smoke/_helpers';
import { pickSelect, pickSelectIn } from '../flow/ui-helpers';
import { fillCellNumberByColumnAndReadBack } from '../flow/table-column';

test.describe('01 报价单创建', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入报价单列表页', async ({ page }) => {
    await page.goto('/quotations');
    // 列表页真实标题 quotations.list.title = '报价单管理'，breadcrumb 同文本需 .first()
    await expect(page.getByText('报价单管理').first()).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('button', { name: /新建报价单/ })).toBeVisible();
  });

  test('01-02 新建报价单', async ({ page }) => {
    await page.goto('/quotations/new');
    await expect(page.locator('form')).toBeVisible({ timeout: 30000 });
    // labelCustomer='客户' 与 labelCustomerLevel='客户等级' 共享子串，pickSelectIn 精确锚定
    await pickSelectIn(page, page, '客户');
    await page.getByLabel('报价日期').fill('2026-08-19');
    await page.keyboard.press('Enter');
    await page.getByLabel('有效期至').fill('2026-09-19');
    await page.keyboard.press('Enter');
    await pickSelectIn(page, page, '价格条款');
    await pickSelectIn(page, page, '币种');
    await page.getByLabel('汇率').fill('1');
    // 报价单 items 验证规则要求至少 1 行明细
    await page.getByRole('button', { name: '添加产品' }).click();
    // 明细表 aria-label 在 el-table 包装 div 上，用属性选择器定位
    const itemsTable = page.locator('[aria-label="报价明细编辑表"]');
    await pickSelect(page, itemsTable.locator('.el-select').first());
    // 明细行数值列是纯手写 el-input 加数字校验（步进控件已全项目禁用），录入框按列头锚定，
    // 失焦归一后读回真值；同时反向钉住该表不再出现步进角色。
    const firstItemRow = itemsTable.locator('.el-table__row').first();
    await expect(
      firstItemRow.getByRole('spinbutton'),
      '报价明细行数值录入不应是步进角色'
    ).toHaveCount(0);
    await fillCellNumberByColumnAndReadBack(itemsTable, firstItemRow, '数量', '100');
    await fillCellNumberByColumnAndReadBack(itemsTable, firstItemRow, '单价', '50');
    // 税率必录（落 sales_quotations.tax_rate NOT NULL，系统不设默认档位），13 是本例录入样本值
    await page.getByPlaceholder('按百分数填写，如 13 表示 13%').fill('13');
    await page.getByLabel('备注').fill('E2E 测试报价单');
    await page.getByRole('button', { name: '保存草稿' }).click();
    // 真实成功提示 quotations.create.draftSaved = '草稿保存成功'
    await expect(page.getByText('草稿保存成功')).toBeVisible({ timeout: 30000 });
  });

  test('01-03 报价单列表可正常加载', async ({ page }) => {
    await page.goto('/quotations');
    // 列表 el-table 包装 div 带 aria-label '报价单列表'（quotationList.tableAriaLabel）。
    // 原 `page.locator('table, .el-table')` 在 strict 模式下命中 3 个（外层 div + 内层
    // header/body <table>）→ "strict mode violation" 假红。改按唯一 aria-label 锚定包装元素。
    await expect(page.locator('[aria-label="报价单列表"]')).toBeVisible({ timeout: 30000 });
  });
});

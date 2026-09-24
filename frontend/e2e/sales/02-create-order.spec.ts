// P9-3 销售 E2E 套件 — 02 创建销售订单
// 覆盖范围：销售订单列表访问、报价转订单、建单表单字段/校验/明细行增删

import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

/**
 * 真实 UI 事实（据源码核对，不虚构选择器）：
 * - 销售管理为扁平单页：router path:'sales' → views/sales/index.vue → OrderListView.vue。
 *   无 /sales/order/create 子路由；新建为页内 OrderFormDialog 对话框。
 * - OrderFormDialog.vue 真实字段（locales/zh-CN.ts sales.orderForm.*）：
 *   客户(customer select 占位 '选择客户')、订单日期(默认今日)、要求交货日期(必填 date 占位 '选择日期')、
 *   联系人(占位 '联系人姓名')、联系电话(占位 '联系电话')、收货地址(占位 '详细收货地址')、
 *   明细行内 el-table：产品(select 占位 '选择产品')、色号(select 占位 '留空表示白坯布')、
 *   数量(spinbutton)、单价(spinbutton)；底部按钮 '取消'/'确定'。
 *   —— 不存在“双计量(米+公斤)”“潘通色号”“等级”等字段，原 02-03/02-04 系按不存在的 UI 编写。
 * - 明细行操作：'添加明细' 按钮新增、行内 '删除' 链接删除（剩 1 行时删除弹 '至少保留一条明细'）。
 * - 提交成功走 useOlvProc.handleFormSubmit → msg.success('createSuccess') = '创建成功'
 *   （message.createSuccess 存在）。必填项缺失时 el-form 校验文案 '请选择客户' 等。
 * - 报价转订单：/quotations 列表已批准行（状态标签 '已批准'，QUOTATION_STATUS_LABELS）
 *   行内 '转订单' 按钮 → ElMessageBox.confirm('确定') → convertQuotation →
 *   router.push('/sales/orders/:id') → OrderDetail.vue 标题 '销售订单详情'。
 */
test.describe('02 创建销售订单', () => {
  test.beforeEach(async ({ context }) => {
    // auth mock 仅注入登录态，业务 API 走真实后端
    await applyAuthMocks(context);
  });

  test('02-01 销售订单列表可访问', async ({ page }) => {
    await page.goto('/sales');
    // OrderListView 页头真实标题 sales.indexPage.title = '销售订单管理'
    await expect(page.getByText('销售订单管理')).toBeVisible();
    // 新建按钮真实文案 sales.indexPage.newOrder = '新建订单'
    await expect(page.getByRole('button', { name: /新建订单/ })).toBeVisible();
  });

  test('02-02 从报价单一键转销售订单并跳转订单详情', async ({ page }) => {
    await page.goto('/quotations');
    // 报价单已批准状态标签为 '已批准'（QUOTATION_STATUS_LABELS.approved），原用例误写 '已审批'
    const approved = page.getByRole('row').filter({ hasText: '已批准' }).first();
    await expect(approved).toBeVisible();
    // 真实行内按钮文案 quotations.list.convertOrder = '转订单'
    await approved.getByRole('button', { name: '转订单', exact: true }).click();
    // 确认对话框（handleConvert ElMessageBox.confirm）
    await page.getByRole('button', { name: '确定', exact: true }).click();
    // 成功后跳转 /sales/orders/:id（OrderDetail.vue）
    await expect(page).toHaveURL(/\/sales\/orders\/\d+/);
    // 详情页真实标题 sales.orderDetail.title = '销售订单详情'
    await expect(page.getByText('销售订单详情')).toBeVisible();
  });

  test('02-03 建单表单必填校验拦截空提交', async ({ page }) => {
    await page.goto('/sales');
    await page.getByRole('button', { name: /新建订单/ }).click();
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    // 明细行产品未选、必填基本信息未填，直接点 '确定'
    await dialog.getByRole('button', { name: '确定', exact: true }).click();
    // el-form 校验：客户为必填 → 展示 sales.orderForm.customerRequired = '请选择客户'
    await expect(dialog.getByText('请选择客户')).toBeVisible();
    // 校验未通过时不应产生成功提示
    await expect(page.locator('.el-message--success')).toHaveCount(0);
  });

  test('02-04 合法填写客户与明细后可创建销售订单', async ({ page }) => {
    await page.goto('/sales');
    await page.getByRole('button', { name: /新建订单/ }).click();
    const dialog = page.getByRole('dialog');
    // 客户：对话框内首个 combobox（el-select），下拉选项 teleported 到 body
    await dialog.getByRole('combobox').first().click();
    await page.getByRole('option').first().click();
    // 要求交货日期（必填 date picker，占位 '选择日期' 的第 2 个）
    await dialog.getByPlaceholder('选择日期').nth(1).fill('2026-12-31');
    await page.keyboard.press('Enter');
    // 联系人 / 联系电话 / 收货地址（el-input，占位属性真实存在）
    await dialog.getByPlaceholder('联系人姓名').fill('张三');
    await dialog.getByPlaceholder('联系电话').fill('13800138000');
    await dialog.getByPlaceholder('详细收货地址').fill('浙江省杭州市西湖区文三路 100 号');
    // 明细行产品（客户之后第 2 个 combobox）
    await dialog.getByRole('combobox').nth(1).click();
    await page.getByRole('option').first().click();
    // 数量 / 单价（spinbutton）
    await dialog.getByRole('spinbutton').first().fill('200');
    await dialog.getByRole('spinbutton').nth(1).fill('45.5');
    // 确定提交
    await dialog.getByRole('button', { name: '确定', exact: true }).click();
    // 真实落库反馈：msg.success('createSuccess') = '创建成功'
    await expect(page.getByText('创建成功')).toBeVisible({ timeout: 30000 });
  });

  test('02-05 销售订单明细行可动态增删', async ({ page }) => {
    await page.goto('/sales');
    await page.getByRole('button', { name: /新建订单/ }).click();
    const dialog = page.getByRole('dialog');
    // 明细编辑表 aria-label sales.orderForm.itemsTableAriaLabel = '销售订单明细编辑表'
    const itemsTable = dialog.getByRole('table', { name: '销售订单明细编辑表' });
    // 默认 1 行
    await expect(itemsTable.getByRole('row')).toHaveCount(1);
    // '添加明细' 两次 → 3 行
    await dialog.getByRole('button', { name: '添加明细' }).click();
    await dialog.getByRole('button', { name: '添加明细' }).click();
    await expect(itemsTable.getByRole('row')).toHaveCount(3);
    // 删除末行 → 2 行
    await itemsTable.getByRole('row').last().getByRole('button', { name: '删除' }).click();
    await expect(itemsTable.getByRole('row')).toHaveCount(2);
  });
});

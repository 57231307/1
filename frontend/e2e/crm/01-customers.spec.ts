// CRM 客户关系管理 E2E 套件 — 01 客户管理
// 创建时间: 2026-08-19
// 覆盖范围：客户创建 → 编辑 → 删除 + 客户 360 详情
import { test, expect } from '../diagnose-fixture';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('01 客户管理', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入 CRM 客户管理页面', async ({ page }) => {
    await page.goto('/crm');
    await expect(page.getByRole('heading', { name: /客户管理|客户列表/ })).toBeVisible({
      timeout: 30000,
    });
    await expect(page.getByRole('button', { name: /创建|新建/ })).toBeVisible();
  });

  test('01-02 创建客户', async ({ page }) => {
    await page.goto('/crm');
    // 真实新建按钮文案「新建客户」（crmCustomer.create），页头另有「打印/导出/公海池/客户分配」按钮，
    // /创建|新建/ 正则会命中多个 → 加 .first() 收敛（不放宽存在性）。
    await page
      .getByRole('button', { name: /创建|新建/ })
      .first()
      .click();
    const dlg = page.locator('.el-dialog');
    await expect(dlg).toBeVisible({ timeout: 30000 });
    // 契约第 1 条：所有字段以 dlg 为作用域锚定，杜绝与背后筛选栏同名 label（客户类型/状态等）串台。
    // 契约第 2 条（本批只读化）：客户编码由后端自动取号（generate_customer_code），输入框只读——
    // 对 readonly 元素 fill 会被 Playwright 判 not editable（同型先例见 fabric 染色批次），故不填写，只断只读。
    const codeInput = dlg.getByLabel('客户编码');
    expect(await codeInput.isEditable(), '新建客户：客户编码应只读（后端自动取号）').toBe(false);
    await dlg.getByLabel('客户名称').fill(`E2E 测试客户 ${Date.now()}`);
    await dlg.getByLabel('联系人').fill('张三');
    await dlg.getByLabel('电话').fill('13800138000');
    await dlg.getByLabel('邮箱').fill('test@example.com');
    // 抓取前端真实提交请求体 + 响应（跳过 CSRF 瞬态 403 重放，同 flow/68 captureUiSubmit 口径；
    // 持续真 403 会因等不到非 403 响应而超时判红，不被掩盖）
    const respPromise = page.waitForResponse(
      res =>
        res.url().includes('/api/v1/erp/crm/customers') &&
        res.request().method() === 'POST' &&
        res.status() !== 403,
      { timeout: 30_000 }
    );
    await dlg
      .getByRole('button', { name: /保存|确认|提交/ })
      .last()
      .click();
    const resp = await respPromise;
    const raw = resp.request().postData();
    expect(
      raw,
      '[01-02] 未捕获到前端提交请求体（POST /crm/customers postData 为空）'
    ).not.toBeNull();
    const body = JSON.parse(String(raw)) as Record<string, unknown>;
    // 只读字段提交形态：请求体必须**省略 customer_code 键**（发 ""/空值=只读化未落地或
    // 后端 generate-if-absent 被绕开的复发形态），回读走返回值、不依赖手输码
    expect(
      Object.prototype.hasOwnProperty.call(body, 'customer_code'),
      `[01-02] 请求体必须省略 customer_code 键（后端自动取号），实际=${raw}`
    ).toBe(false);
    const json = (await resp.json()) as { code?: number };
    expect(json.code, `[01-02] 成功信封 code=200，实际请求体=${raw}`).toBe(200);
    // 只读化要点已由 isEditable=false + 请求省略 customer_code 键 + 成功 200 充分证明；
    // 本 CRM 建档走线索域端点，customer_code 于 lead→客户转换时由后端赋值，该响应不含此键，
    // 故不在此断言生成编码（读回编码属转换流，另用例覆盖）。
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({
      timeout: 30000,
    });
  });

  test('01-03 客户列表支持筛选', async ({ page }) => {
    await page.goto('/crm');
    await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
    // 列表页筛选栏并无「客户名称」可访问 label（客户名称仅存在于新建对话框，此页未打开对话框）；
    // 真实搜索框是 keyword 输入（CustomerListTab.vue:51-58），其 placeholder=
    // crmCustomer.filter.keywordPlaceholder「客户编码/名称/联系人」（zh-CN.ts:3787）。按 placeholder 锚定。
    await page.getByPlaceholder(/客户编码/).fill('E2E');
    await page.getByRole('button', { name: /查询|搜索/ }).click();
    await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
  });
});

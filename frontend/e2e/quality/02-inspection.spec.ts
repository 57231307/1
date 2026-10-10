// 质量管理 E2E 套件 — 02 检验记录与缺陷处理
// 创建时间: 2026-08-19
// 覆盖范围：检验记录创建 → 合格/不合格判定 → 缺陷处理
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  APP_ERROR_CODES,
  genCode,
  tryCleanup,
} from '../flow/helpers';
import { pickSelectIn, fillFieldByLabel, formItemByExactLabel } from '../flow/ui-helpers';

/**
 * 缺陷管理（DefectTab）列表数据源是 unqualified_products 表
 * （backend/services/quality_inspection_service.rs::get_defects_list）。
 *
 * 动作语义（D1②，本用例按新契约改写）：
 * - `/defects/{id}/process` = 从**质检记录**开单（{id}=质检记录 id，INSERT 一行台账，
 *   带同记录非终态行幂等守卫）；
 * - `/defects/{id}/process-result` = **台账行原地更新**处置结果（{id}=行自身 id，
 *   不新开行）：仅 handling_status=pending 可行，成功后推进为 approved 并写
 *   handling_by/handling_at；状态门拒 ⇒ 4xx BUSINESS_ERROR，词表外方式 ⇒ 4xx
 *   VALIDATION_ERROR（本仓拒绝文案永久脱敏，只断 status + 机器码，不断案文案）。
 * 台账 UI 的「处理」按钮已改指 process-result + row.id（DefectTab.vue），正例核心判据
 * =「本行被更新且台账行数不变」。
 */
const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** 台账行出参（unqualified_product::Model 直出的本用例所需键） */
interface DefectRow {
  id: number;
  unqualified_no: string;
  handling_status: string;
}

/** 造一条缺陷：建质检记录 → 开单为不合格品（生成 unqualified_product），回双 id */
async function seedDefect(
  page: import('@playwright/test').Page
): Promise<{ recordId: number; rowId: number }> {
  const products = await apiCallRaw<{ items?: Array<{ id: number }> }>(
    page,
    'GET',
    '/products?page=1&page_size=1'
  );
  const productId = products.items?.[0]?.id;
  if (!productId) throw new Error('无可用产品，无法构造质检记录');

  const inspectionNo = genCode('E2E-QIR');
  const record = await apiCall<{ id?: number }>(
    page,
    'POST',
    '/production/quality-inspection/records',
    {
      inspection_no: inspectionNo,
      inspection_type: 'finished',
      product_id: productId,
      inspection_date: new Date().toISOString().slice(0, 10),
      total_qty: 100,
      inspected_qty: 100,
      qualified_qty: 90,
      unqualified_qty: 10,
      inspection_result: '不合格',
      grade: 'C',
    }
  );
  if (!record.data?.id) throw new Error(`建质检记录失败：${JSON.stringify(record)}`);
  CLEANUP.push({
    path: `/production/quality-inspection/records/${record.data.id}`,
    label: 'quality_record',
  });

  // 由不合格记录派生 unqualified_product（缺陷列表条目）；开单响应回整行 Model
  const defect = await apiCall<DefectRow>(
    page,
    'POST',
    `/production/quality-inspection/defects/${record.data.id}/process`,
    {
      unqualified_qty: 10,
      unqualified_reason: 'E2E 缺陷前置',
      handling_method: 'rework',
      remark: 'E2E',
    }
  );
  const rowId = defect.data?.id;
  if (!rowId) throw new Error(`缺陷开单未返回行 id：${JSON.stringify(defect)}`);
  return { recordId: record.data.id, rowId };
}

test.describe('02 检验记录与缺陷处理', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('02-01 新建检验记录', async ({ page }) => {
    await page.goto('/quality');
    await page.getByRole('tab', { name: /检验记录/ }).click();
    await page.getByRole('button', { name: /新建/ }).click();
    const dlg = page.locator('.el-dialog:visible').last();
    await expect(dlg).toBeVisible({ timeout: 30000 });
    // 旧用例用错控件/标签：把「产品」当文本框 fill（实为 filterable el-select）、把「检验结果」
    // 当 el-select（实为 el-radio-group，默认已选「待检」），且漏填必填的 记录编号/检验类型/
    // 检验日期/送检总数/实际检验数 → 本地必填校验拦下，永不发请求（timeout 根因）。
    // 按真实表单契约以可见对话框为 root + 精确 label 逐项驱动（number fill 后 Tab 提交 v-model）。
    await fillFieldByLabel(dlg, page, '记录编号', genCode('E2E-QIR'));
    await pickSelectIn(dlg, page, '检验类型', { index: 0 });
    await pickSelectIn(dlg, page, '产品', { index: 0 });
    await fillFieldByLabel(dlg, page, '批次号', `BATCH-${Date.now()}`);
    const dateInput = formItemByExactLabel(dlg, '检验日期').locator('input').first();
    await dateInput.click();
    await dateInput.fill(new Date().toISOString().slice(0, 10));
    await page.keyboard.press('Enter');
    await fillFieldByLabel(dlg, page, '送检总数', '100');
    await page.keyboard.press('Tab');
    await fillFieldByLabel(dlg, page, '实际检验数', '100');
    await page.keyboard.press('Tab');
    await dlg.getByRole('button', { name: '确定' }).click();
    // 成功提示 quality.message.operationSuccess=「操作成功」（非「创建成功/保存成功」）
    await expect(page.getByText('操作成功')).toBeVisible({ timeout: 30000 });
  });

  test('02-02 检验记录列表可正常加载', async ({ page }) => {
    await page.goto('/quality');
    await page.getByRole('tab', { name: /记录/ }).click();
    await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
  });

  test('02-03 缺陷管理 Tab 可正常加载', async ({ page }) => {
    await page.goto('/quality');
    await page.getByRole('tab', { name: /缺陷/ }).click();
    await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
  });

  test('02-04 台账处理=本行原地更新（process-result）：正例本行更新且行数不变，负例只断 status+机器码', async ({
    page,
  }) => {
    const { recordId, rowId } = await seedDefect(page);
    // D2 筛选已下推真实生效：record_id 等值命中派生自该质检记录的台账行
    const listByRecord = () =>
      apiCallRaw<DefectRow[]>(
        page,
        'GET',
        `/production/quality-inspection/defects?record_id=${recordId}&page=1&page_size=100`
      );

    const before = await listByRecord();
    expect(Array.isArray(before), `record_id 筛选应返回数组，实际=${JSON.stringify(before)}`).toBe(
      true
    );
    expect(before, '开单后该质检记录应恰有一条台账行').toHaveLength(1);
    expect(before[0].id, '筛选命中行须为 seed 行').toBe(rowId);
    expect(before[0].handling_status, '新开单行初态应为 pending').toBe('pending');

    // 正例：台账行原地更新处置结果（路径 id = 行自身 id，非质检记录 id）
    const updated = await apiCallRaw<DefectRow & { handling_by: number | null }>(
      page,
      'POST',
      `/production/quality-inspection/defects/${rowId}/process-result`,
      { handling_method: 'rework', reason: 'E2E 处置结果验证' }
    );
    expect(updated.id, '响应应为被更新的同一行').toBe(rowId);
    expect(updated.handling_status, '处置成功后本行 handling_status 应推进为 approved').toBe(
      'approved'
    );
    expect(updated.handling_by, '操作人应由服务端会话派生落库（非空）').toBeTruthy();
    const after = await listByRecord();
    expect(after, '处置后台账行数不变（原地更新，不新开行）').toHaveLength(1);
    expect(after[0].handling_status, '列表回读与写响应一致（真实落库）').toBe('approved');

    // 负例①（状态门）：approved 终态行再次提处置结果 ⇒ 4xx + BUSINESS_ERROR；
    // 本仓拒绝文案永久脱敏，只断 status + 机器码，不断案文案
    const reProcess = await apiCallExpectFail(
      page,
      'POST',
      `/production/quality-inspection/defects/${rowId}/process-result`,
      { handling_method: 'downgrade_sale' }
    );
    expect(reProcess.status, '终态行原地更新应被状态门拒绝（400）').toBe(400);
    expect(
      failureCode(reProcess),
      `状态门机器码应为 BUSINESS_ERROR，实际=${JSON.stringify(reProcess.code)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 负例②（词表门）：词表外 handling_method ⇒ 400 + VALIDATION_ERROR
    const badMethod = await apiCallExpectFail(
      page,
      'POST',
      `/production/quality-inspection/defects/${rowId}/process-result`,
      { handling_method: 'REWORK' }
    );
    expect(badMethod.status, '词表外处理方式应被字段校验拒绝（400）').toBe(400);
    expect(
      failureCode(badMethod),
      `词表机器码应为 VALIDATION_ERROR，实际=${JSON.stringify(badMethod.code)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 被拒零半途写入：终态行状态不因两次被拒而漂移
    const finalRows = await listByRecord();
    expect(finalRows).toHaveLength(1);
    expect(finalRows[0].handling_status, '被拒后行状态不得漂移').toBe('approved');
  });
});

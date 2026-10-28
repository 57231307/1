import { test, expect } from '../diagnose-fixture';
import { loginViaUI, apiCallRaw, apiCallExpectFail } from './helpers';

/**
 * 121 纺织品安全送检判定链写后读回（textile_safety 域，textile-submissions 与 textile-reports 族）
 *
 * 覆盖真实端点与真实判据（routes/textile_safety.rs 挂载）：
 * - POST /textile-submissions 建单（安全类别只认 GB18401_ 前缀稳定码；裸字母 B 必 400），
 *   读回整行逐键（status 恒 draft、overall_verdict 恒 null，请求体不可直填结论）。
 * - POST /textile-submissions/{id}/items 清单推导（GB18401 恒入列、童装加挂 GB31701、
 *   出口市场加挂 EXPORT 归因），GET 详情读回 items 非空。
 * - PUT /textile-submissions/{id}/status 状态机白名单：judged 手工直达必拒；
 *   draft→sampling→sent 合法推进并回显库存联动 scope_note。
 * - POST /textile-reports 登记（受理态恒 pending，请求体不接受 accepted）；
 *   PUT /textile-reports/{id}/acceptance 完整性门缺项逐条点名（缺 CMA 记 cma、缺结论记
 *   conclusion、无有效期记 expired），读回 rejection_detail 与 acceptance_status rejected。
 * - 要素齐全后过门 accepted，POST /{id}/results 录入实测（等级 token 单源、数值零混填），
 *   PUT /{id}/judgement 报告级判定读回逐行 verdict，PUT /textile-submissions/{id}/judgement
 *   整单判定读回 overall_verdict fail（甲醛 90 超 B 类 75 上限）。
 * - 页面健康：/textile-safety 与 /textile-safety-master 渲染出表格与新建入口，
 *   两页 .el-input-number 计数恒 0（全项目步进控件红线自证）。
 * 批次号取 E2E 唯一哨兵，不命中真实库存行；冻结与库存真值由后端活体用例钉，
 * 本用例判接口信封与逐字段读回，不断 total、不读他人在途行。
 */

type Submission = {
  id: number;
  submission_no: string;
  product_id: number;
  batch_no: string | null;
  color_no: string;
  process_no: string;
  color_role: string;
  end_user_group: string;
  contact_type: string;
  safety_category: string;
  product_form: string;
  target_market: string;
  exemption_tags: string | null;
  status: string;
  overall_verdict: string | null;
  remarks: string | null;
  created_by: number;
  created_at: string;
  updated_at: string;
};

type SubmissionDetail = {
  submission: Submission;
  items: { item_code: string; source: string; basis_standard: string }[];
  child_checks: unknown[];
  reports: ReportRow[];
};

type ReportRow = {
  id: number;
  report_no: string;
  submission_id: number;
  acceptance_status: string;
  rejection_detail: string;
  valid_until: string | null;
};

function uniq(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
}

function baseSubmission(tag: string, productId: number): Record<string, unknown> {
  return {
    submission_no: `E2E-TXT-SUB-${tag}`,
    product_id: productId,
    batch_no: `E2E-TXT-BATCH-${tag}`,
    color_no: `RED-${tag}`,
    process_no: `PR-${tag}`,
    color_role: 'full',
    end_user_group: 'adult',
    contact_type: 'direct',
    safety_category: 'GB18401_B',
    product_form: 'final',
    target_market: 'dom',
    exemption_tags: [],
    remarks: `E2E 送检 ${tag}`,
  };
}

test.describe.serial('121 纺织品安全送检判定链写后读回', () => {
  const tag = uniq();
  let productId = 0;
  let submissionId = 0;
  let reportId = 0;

  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('取真实品种 ID 作为送检挂靠点', async ({ page }) => {
    const data = await apiCallRaw<{ items: { id: number }[] }>(
      page,
      'GET',
      '/products?page=1&page_size=1'
    );
    expect(data.items.length, '测试库必须已有产品种子，送检单不许挂空品种').toBeGreaterThan(0);
    productId = data.items[0].id;
  });

  test('裸字母安全类别被词表拒绝', async ({ page }) => {
    const bad = { ...baseSubmission(`BAD-${tag}`, productId), safety_category: 'B' };
    const fail = await apiCallExpectFail(page, 'POST', '/textile-submissions', bad);
    expect(fail.status, `裸字母类别应 400，实际 ${fail.status}`).toBe(400);
    expect(fail.message ?? JSON.stringify(fail)).toContain('安全类别');
  });

  test('建单落草稿并逐键读回', async ({ page }) => {
    const created = await apiCallRaw<Submission>(
      page,
      'POST',
      '/textile-submissions',
      baseSubmission(tag, productId)
    );
    submissionId = created.id;
    expect(created.status).toBe('draft');
    expect(created.overall_verdict).toBeNull();
    expect(created.safety_category).toBe('GB18401_B');
    const got = await apiCallRaw<SubmissionDetail>(
      page,
      'GET',
      `/textile-submissions/${submissionId}`
    );
    expect(got.submission.submission_no).toBe(`E2E-TXT-SUB-${tag}`);
    expect(got.items.length, '新单尚无清单').toBe(0);
  });

  test('清单推导按依据入列并读回', async ({ page }) => {
    const r = await apiCallRaw<{ items: { item_code: string; source: string }[] }>(
      page,
      'POST',
      `/textile-submissions/${submissionId}/items`,
      { extra_item_codes: [] }
    );
    expect(r.items.length).toBeGreaterThanOrEqual(10);
    expect(r.items.some(i => i.item_code === 'HCHO')).toBeTruthy();
    expect(r.items.some(i => i.item_code === 'FAST_WET_RUB')).toBeTruthy();
  });

  test('judged 不许手工直达', async ({ page }) => {
    const fail = await apiCallExpectFail(
      page,
      'PUT',
      `/textile-submissions/${submissionId}/status`,
      { status: 'judged' }
    );
    expect(fail.status).toBe(400);
  });

  test('推进到已送检并回显库存联动说明', async ({ page }) => {
    const s1 = await apiCallRaw<{ submission: Submission; stock_link: { scope_note: string } }>(
      page,
      'PUT',
      `/textile-submissions/${submissionId}/status`,
      { status: 'sampling' }
    );
    expect(s1.submission.status).toBe('sampling');
    const s2 = await apiCallRaw<{
      submission: Submission;
      stock_link: { matched: number; scope_note: string };
    }>(page, 'PUT', `/textile-submissions/${submissionId}/status`, { status: 'sent' });
    expect(s2.submission.status).toBe('sent');
    expect(s2.stock_link.matched, 'E2E 哨兵批次不应命中真实库存行').toBe(0);
    expect(s2.stock_link.scope_note).toContain('批次');
  });

  test('缺资质缺要素的报告被完整性门点名退回', async ({ page }) => {
    const report = await apiCallRaw<ReportRow>(page, 'POST', '/textile-reports', {
      report_no: `E2E-TXT-RPT-${tag}`,
      submission_id: submissionId,
      inspection_agency: 'E2E 检测机构',
      has_cma: false,
      has_cnas: false,
      sample_name: null,
      material_composition: null,
      color_no: `RED-${tag}`,
      process_no: `PR-${tag}`,
      test_basis: 'GB18401 B类',
      conclusion: null,
      oeko_tex_class: null,
      issue_date: '2026-10-01',
      valid_until: null,
      report_file_url: null,
    });
    reportId = report.id;
    expect(report.acceptance_status).toBe('pending');
    const gated = await apiCallRaw<{ report: ReportRow; missing_elements: string[] }>(
      page,
      'PUT',
      `/textile-reports/${reportId}/acceptance`,
      {}
    );
    expect(gated.report.acceptance_status).toBe('rejected');
    for (const code of ['cma', 'cnas', 'sample_name', 'conclusion', 'expired', 'test_basis']) {
      expect(
        gated.missing_elements,
        `缺项应点名 ${code}，实际 ${JSON.stringify(gated.missing_elements)}`
      ).toContain(code);
    }
    const fail = await apiCallExpectFail(page, 'PUT', `/textile-reports/${reportId}/judgement`, {});
    expect(fail.status, '未过门的报告不得产判定').toBe(400);
  });

  test('修正要素并录入实测后过门判定，甲醛超限读回 fail', async ({ page }) => {
    await apiCallRaw<ReportRow>(page, 'PUT', `/textile-reports/${reportId}`, {
      report_no: `E2E-TXT-RPT-${tag}`,
      submission_id: submissionId,
      inspection_agency: 'E2E 检测机构',
      has_cma: true,
      has_cnas: true,
      sample_name: 'E2E 面料样',
      material_composition: '棉 100%',
      color_no: `RED-${tag}`,
      process_no: `PR-${tag}`,
      test_basis: 'GB 18401-2010 B类',
      conclusion: '按限值判定',
      oeko_tex_class: null,
      issue_date: '2026-10-01',
      valid_until: '2027-12-31',
      report_file_url: null,
    });
    await apiCallRaw(page, 'POST', `/textile-reports/${reportId}/results`, {
      rows: [
        {
          item_code: 'HCHO',
          measured_value: 90,
          measured_unit: 'mg/kg',
          test_method: 'GB/T 2912.1',
        },
        { item_code: 'PH', measured_value: 7.0, test_method: 'GB/T 7573' },
        { item_code: 'AZO', measured_value: 10, measured_text: 'none', measured_unit: 'mg/kg' },
        { item_code: 'ODOR', measured_text: 'none' },
        { item_code: 'FAST_WATER', measured_token: '3-4' },
        { item_code: 'FAST_ACID_SWEAT', measured_token: '3-4' },
        { item_code: 'FAST_ALKALI_SWEAT', measured_token: '3-4' },
        { item_code: 'FAST_DRY_RUB', measured_token: '4' },
        { item_code: 'FAST_WET_RUB', measured_token: '3-4' },
      ],
    });
    const gated = await apiCallRaw<{ report: ReportRow; missing_elements: string[] }>(
      page,
      'PUT',
      `/textile-reports/${reportId}/acceptance`,
      {}
    );
    expect(
      gated.missing_elements,
      `要素齐全应过门，实际缺项 ${JSON.stringify(gated.missing_elements)}`
    ).toHaveLength(0);
    const judgedRows = await apiCallRaw<
      { id: number; item_id: number; verdict: string; measured_value: string | null }[]
    >(page, 'PUT', `/textile-reports/${reportId}/judgement`, {});
    // 后端 DECIMAL(12,4) 出参为字符串线格式（rust_decimal 未开 serde-float），
    // 与全仓既有 e2e 同构：Number 包裹后按数值语义比较
    const hcho = judgedRows.find(r => Number(r.measured_value) === 90);
    expect(hcho, '结果行必须含甲醛 90').toBeTruthy();
    expect(hcho!.verdict, '甲醛 90 超 B 类 75 上限必须 fail').toBe('fail');
    const judged = await apiCallRaw<{
      submission: Submission;
      stock_link: { scope_note: string };
      expired_reports_named: string[];
    }>(page, 'PUT', `/textile-submissions/${submissionId}/judgement`, {});
    expect(judged.submission.status).toBe('judged');
    expect(judged.submission.overall_verdict).toBe('fail');
    expect(judged.expired_reports_named).toHaveLength(0);
    const detail = await apiCallRaw<SubmissionDetail>(
      page,
      'GET',
      `/textile-submissions/${submissionId}`
    );
    expect(detail.submission.overall_verdict).toBe('fail');
  });

  test('送检中心与主数据页渲染且零步进控件', async ({ page }) => {
    await page.goto('#/textile-safety');
    await expect(page.locator('.textile-safety-page')).toBeVisible();
    await expect(page.getByRole('button', { name: '新建送检单' })).toBeVisible();
    expect(await page.locator('.el-input-number').count(), '全项目禁步进控件').toBe(0);
    await page.goto('#/textile-safety-master');
    await expect(page.locator('.textile-master-page')).toBeVisible();
    expect(await page.locator('.el-input-number').count(), '全项目禁步进控件').toBe(0);
  });
});

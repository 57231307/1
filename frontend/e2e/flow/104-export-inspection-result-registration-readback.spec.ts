import { test, expect } from '../diagnose-fixture';
import { APP_ERROR_CODES, loginViaUI, apiCall, apiCallRaw, apiCallExpectFail } from './helpers';

/**
 * 104 出口商检建单与结论登记读回（export_inspection_routes 域，export-inspections 族）
 * 覆盖四个真实端点（handlers/export_inspection_handler.rs，routes/export_inspection_routes.rs 挂载）：
 * - POST /export-inspections 建单，入参 inspection_no、product_name、hs_code、inspection_type、
 *   inspection_agency、inspection_date、sales_order_id 必填（sales_order_id 是 INTEGER 无外键约束，
 *   用例取 990011 私有段哨兵值，不引用他人在途订单），delivery_id 与 remarks 可选；
 *   请求体不接受 result（服务层恒落词表 pending，小写三值词表为 pending、pass、fail）与
 *   created_by（建单人取会话，审计归属不可伪造），返回落库整行。
 * - GET /export-inspections/{id} 取单条逐键读回，id 不存在返回 404 且码为 NOT_FOUND。
 * - PUT /export-inspections/{id}/result 登记结论，result 必填且逐字符命中词表，词表外（含大写
 *   变体与中文近义）一律 400 且码为 VALIDATION_ERROR；report_url、certificate_no、certificate_expiry
 *   可选，但更新是整行覆盖语义（未提交的可选列被落 NULL 而非保留旧值），改判 pass 到 fail 可重复登记。
 * - GET /export-inspections 列表，出参 data 是分页信封，入参 inspection_no 按包含匹配、
 *   result 按等值过滤；page 从 1 开始、page_size 限 1 到 100，越界按 400 fail-visible 拒绝，
 *   不静默夹紧。
 * 本域没有删除端点，测试数据留在表里：每条用例的断言一律按自建行 id 与唯一单号命中，
 * 不读别人的行、不断 total，避免并发分片相互干扰。
 * 断言均为值判据（建单读回 pending 与逐字段原文、登记后读回证书号与到期日、改判后旧值被覆盖、
 * 负例钉到具体 HTTP 码与机器码），不只判元素可见、不以小于 500 当健康。
 */

// 商检单整行（export_inspection Model 的 snake_case 键）。
type Inspection = {
  id: number;
  inspection_no: string;
  sales_order_id: number;
  delivery_id: number | null;
  product_name: string;
  hs_code: string;
  inspection_type: string;
  inspection_agency: string;
  inspection_date: string;
  result: string;
  report_url: string | null;
  certificate_no: string | null;
  certificate_expiry: string | null;
  remarks: string | null;
  created_by: number;
  created_at: string;
  updated_at: string;
};

// 列表出参（ApiResponse.data 即分页信封）。
type InspectionPage = {
  items: Inspection[];
  total: number;
  page: number;
  page_size: number;
};

function uniq(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
}

/** 建一张待检态商检单（哨兵订单号 990011，单号带唯一后缀）。 */
async function seedInspection(
  page: import('@playwright/test').Page,
  tag: string
): Promise<Inspection> {
  return apiCallRaw<Inspection>(page, 'POST', '/export-inspections', {
    inspection_no: `E2E-EI-${tag}`,
    sales_order_id: 990011,
    product_name: `E2E 出口面料 ${tag}`,
    hs_code: '5209210000',
    inspection_type: 'PRE_EXPORT',
    inspection_agency: 'E2E 商检机构',
    inspection_date: '2026-09-15',
    remarks: `建单备注 ${tag}`,
  });
}

test.describe.serial('104 出口商检建单与结论登记读回', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('建单落 pending 与逐字段原文，单条读回一致', async ({ page }) => {
    const tag = uniq();
    const created = await seedInspection(page, tag);
    expect(Number.isInteger(created.id) && created.id > 0, `建单应返回正整数 id`).toBe(true);

    // 建单回显即服务端派生值：结论恒 pending、证书三列全空、建单人取会话。
    expect(created.result, `建单结论应恒落词表 pending，实际：${String(created.result)}`).toBe(
      'pending'
    );
    expect(created.report_url, `建单 report_url 应为 NULL`).toBeNull();
    expect(created.certificate_no, `建单 certificate_no 应为 NULL`).toBeNull();
    expect(created.certificate_expiry, `建单 certificate_expiry 应为 NULL`).toBeNull();
    expect(
      Number.isInteger(created.created_by) && created.created_by > 0,
      `created_by 应为登录用户 id 整数，实际：${String(created.created_by)}`
    ).toBe(true);

    // 从库经单条接口逐键读回，证明落库而非仅在写接口回显。
    const got = await apiCallRaw<Inspection>(page, 'GET', `/export-inspections/${created.id}`);
    expect(got.inspection_no).toBe(`E2E-EI-${tag}`);
    expect(got.product_name).toBe(`E2E 出口面料 ${tag}`);
    expect(got.hs_code, `HS 编码应原样读回，实际：${got.hs_code}`).toBe('5209210000');
    expect(got.inspection_type).toBe('PRE_EXPORT');
    expect(got.inspection_agency).toBe('E2E 商检机构');
    expect(got.inspection_date, `商检日期应读回提交值，实际：${String(got.inspection_date)}`).toBe(
      '2026-09-15'
    );
    expect(
      got.sales_order_id,
      `哨兵订单号应原样落库读回，实际：${String(got.sales_order_id)}`
    ).toBe(990011);
    expect(got.delivery_id, `未提交 delivery_id 应为 NULL`).toBeNull();
    expect(got.remarks).toBe(`建单备注 ${tag}`);
    expect(got.result, `单条读回结论仍应为 pending`).toBe('pending');
  });

  test('登记 pass 后读回证书三列，改判 fail 时未提交可选列落 NULL', async ({ page }) => {
    const tag = uniq();
    const created = await seedInspection(page, tag);

    const passed = await apiCallRaw<Inspection>(
      page,
      'PUT',
      `/export-inspections/${created.id}/result`,
      {
        result: 'pass',
        report_url: `/reports/e2e-${tag}.pdf`,
        certificate_no: `CERT-${tag}`,
        certificate_expiry: '2027-03-31',
      }
    );
    expect(passed.result, `登记后回显 result 应为 pass`).toBe('pass');

    const readBack = await apiCallRaw<Inspection>(page, 'GET', `/export-inspections/${created.id}`);
    expect(readBack.result, `读回结论应为 pass，实际：${readBack.result}`).toBe('pass');
    expect(
      readBack.certificate_no,
      `证书号应原样读回，实际：${String(readBack.certificate_no)}`
    ).toBe(`CERT-${tag}`);
    expect(
      readBack.certificate_expiry,
      `证书到期日应读回提交值，实际：${String(readBack.certificate_expiry)}`
    ).toBe('2027-03-31');
    expect(readBack.report_url).toBe(`/reports/e2e-${tag}.pdf`);

    // 改判为 fail 且只提交 result：整行覆盖语义下未提交的可选列应被落 NULL，不是保留旧值。
    await apiCall(page, 'PUT', `/export-inspections/${created.id}/result`, { result: 'fail' });
    const afterFlip = await apiCallRaw<Inspection>(
      page,
      'GET',
      `/export-inspections/${created.id}`
    );
    expect(afterFlip.result, `改判后结论应为 fail，实际：${afterFlip.result}`).toBe('fail');
    expect(
      afterFlip.certificate_no,
      `改判未提交 certificate_no，应被覆盖为 NULL，实际：${String(afterFlip.certificate_no)}`
    ).toBeNull();
    expect(
      afterFlip.report_url,
      `改判未提交 report_url，应被覆盖为 NULL，实际：${String(afterFlip.report_url)}`
    ).toBeNull();
  });

  test('词表外结论被拒 400，登记到不存在单号 404', async ({ page }) => {
    const tag = uniq();
    const created = await seedInspection(page, tag);

    // 大写变体与中文近义都不在词表，逐字符判定必须拒绝。
    const upper = await apiCallExpectFail(page, 'PUT', `/export-inspections/${created.id}/result`, {
      result: 'PASS',
    });
    expect(upper.status, `大写 PASS 应 400，实际 status=${upper.status} code=${upper.code}`).toBe(
      400
    );
    expect(upper.code, `大写变体的码应为 VALIDATION_ERROR，实际：${String(upper.code)}`).toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );

    const chinese = await apiCallExpectFail(
      page,
      'PUT',
      `/export-inspections/${created.id}/result`,
      {
        result: '合格',
      }
    );
    expect(
      chinese.status,
      `中文近义应 400，实际 status=${chinese.status} code=${chinese.code}`
    ).toBe(400);
    expect(chinese.code, `中文近义的码应为 VALIDATION_ERROR，实际：${String(chinese.code)}`).toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );

    // 两次拒绝后结论未被改动（拒绝不落库）。
    const unchanged = await apiCallRaw<Inspection>(
      page,
      'GET',
      `/export-inspections/${created.id}`
    );
    expect(unchanged.result, `被拒后结论应仍为 pending，实际：${unchanged.result}`).toBe('pending');

    const missing = await apiCallExpectFail(page, 'PUT', '/export-inspections/990012/result', {
      result: 'pass',
    });
    expect(
      missing.status,
      `登记到不存在商检单应 404，实际 status=${missing.status} code=${missing.code}`
    ).toBe(404);
    expect(missing.code, `不存在商检单的码应为 NOT_FOUND，实际：${String(missing.code)}`).toBe(
      APP_ERROR_CODES.NOT_FOUND
    );
  });

  test('列表按单号包含与结论等值双向命中，页码越界 fail-visible 判红', async ({ page }) => {
    const tag = uniq();
    const created = await seedInspection(page, tag);

    // 正向：唯一单号加 pending 过滤应命中自建行。
    const hit = await apiCall<InspectionPage>(
      page,
      'GET',
      `/export-inspections?inspection_no=E2E-EI-${tag}&result=pending&page=1&page_size=20`
    );
    const found = hit.data.items.find(r => r.id === created.id);
    expect(found, `列表应命中自建行 id=${created.id}`).toBeTruthy();
    expect(found!.inspection_no).toBe(`E2E-EI-${tag}`);
    expect(found!.result).toBe('pending');

    // 反向：同一行改判 pass 后，pending 等值过滤不再含它，pass 过滤开始含它。
    await apiCall(page, 'PUT', `/export-inspections/${created.id}/result`, { result: 'pass' });
    const afterPass = await apiCall<InspectionPage>(
      page,
      'GET',
      `/export-inspections?inspection_no=E2E-EI-${tag}&result=pending&page=1&page_size=20`
    );
    expect(
      afterPass.data.items.some(r => r.id === created.id),
      `改判 pass 后 pending 过滤不应再含 id=${created.id}`
    ).toBe(false);
    const passList = await apiCall<InspectionPage>(
      page,
      'GET',
      `/export-inspections?inspection_no=E2E-EI-${tag}&result=pass&page=1&page_size=20`
    );
    expect(
      passList.data.items.some(r => r.id === created.id),
      `pass 过滤应含改判后的 id=${created.id}`
    ).toBe(true);

    // 分页入参越界不静默夹紧，page=0 与 page_size=101 各自 400 且码为 VALIDATION_ERROR。
    const badPage = await apiCallExpectFail(page, 'GET', '/export-inspections?page=0&page_size=20');
    expect(
      badPage.status,
      `page=0 应 400，实际 status=${badPage.status} code=${badPage.code}`
    ).toBe(400);
    expect(badPage.code, `page=0 的码应为 VALIDATION_ERROR，实际：${String(badPage.code)}`).toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
    const badSize = await apiCallExpectFail(
      page,
      'GET',
      '/export-inspections?page=1&page_size=101'
    );
    expect(
      badSize.status,
      `page_size=101 应 400，实际 status=${badSize.status} code=${badSize.code}`
    ).toBe(400);
    expect(
      badSize.code,
      `page_size=101 的码应为 VALIDATION_ERROR，实际：${String(badSize.code)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
  });
});

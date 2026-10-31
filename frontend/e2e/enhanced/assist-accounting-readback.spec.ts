// [法#3 范围锁定] 目标：glob='frontend/e2e/enhanced/assist-accounting-readback.spec.ts'
// 辅助核算域零覆盖补齐 E2E — 维度 余额 核对 穿透 业务单查 五维查 汇总
//
// 功能：把 assist-accounting 域七枚在册 GET 端点钉成契约面真值链：
// 维度种子词表逐字回读，余额恒等式独立于库内容成立，期间解析门逐文案钉死，
// 查询参数必填门钉统一信封，穿透分页 clamp 回显钉实现值。
// 调用方：路由注册在 backend/src/routes/analytics.rs 的 assist_accounting 分组
// （nest 前缀 assist-accounting），handler 在
// backend/src/handlers/assist_accounting_handler.rs，聚合与期间解析在
// backend/src/services/assist_accounting_service.rs；前端调用面为辅助核算报表页。
// 入参：balance 与 check-balance 与 drill-down 与 summary 与 records/business
// 均为 Query 必填字符串参数（accounting_period period dimension_code 等），
// 缺参由提取器拒绝走统一信封 400；期间格式 YYYY-MM，越界月份拒收。
// 传给谁：assist_accounting_dimension、assist_accounting_record、
// assist_accounting_summary 三张表与 voucher 系总账联查（check-balance）。
// 存什么：本文件只读，不落库；维度八条种子由服务启动 initialize_dimensions
// 幂等写入（BATCH COLOR DYE_LOT GRADE WORKSHOP WAREHOUSE CUSTOMER SUPPLIER）。
// 存哪里：assist-accounting 三表；本文件负例断言回读仅验证查询面零结果，不写行。
// 本文件覆盖端点：GET assist-accounting/dimensions、GET balance、
// GET check-balance、GET drill-down、GET records/business、
// GET records/five-dimension/{id}、GET summary。
// 隔离事实：域内无写端点，用例不造行；不存在标记（本轮唯一 business_no 与
// 远未来期间 2099-01 与不存在的维度值 ID）钉零结果，条件真值腿仅在库中
// 已有辅助核算记录时按真实 business_type business_no five_dimension_id 钉过滤生效，
// 库为空时该腿如实不钉（不假装有数据契约）。
// 脱敏事实：parse_period 与维度词表走 validation_displayable，出参外显真实文案；
// 提取器缺参拒绝由中间件收口为 400 加 VALIDATION_ERROR 统一信封，文案不外锁。
import { test, expect } from '../diagnose-fixture';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  APP_ERROR_CODES,
  apiCallExpectFail,
  apiCallRaw,
  failureCode,
  genCode,
} from '../flow/helpers';

type Row = Record<string, unknown>;
const str = (v: unknown): string => String(v);
const num = (v: unknown): number => Number(String(v));

/** 远未来期间：辅助核算与总账在该窗口必然无行，零结果与恒等式都稳定成立 */
const EMPTY_PERIOD = '2099-01';

test.describe('辅助核算 读面契约回读', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('AA-01 维度种子词表：八码齐全 is_active 全真 sort_order 升序', async ({ page }) => {
    const dims = await apiCallRaw<Row[]>(page, 'GET', '/assist-accounting/dimensions');
    expect(Array.isArray(dims), `维度应出数组，实际 ${JSON.stringify(dims)}`).toBe(true);
    const codes = dims.map(d => str(d.dimension_code));
    for (const code of [
      'BATCH',
      'COLOR',
      'DYE_LOT',
      'GRADE',
      'WORKSHOP',
      'WAREHOUSE',
      'CUSTOMER',
      'SUPPLIER',
    ]) {
      expect(codes, `启动种子应含维度 ${code}，实际 ${JSON.stringify(codes)}`).toContain(code);
    }
    for (const d of dims) {
      expect(
        str(d.dimension_code).length > 0 && str(d.dimension_name).length > 0,
        `维度行应带编码与名称，实际 ${JSON.stringify(d)}`
      ).toBe(true);
      expect(d.is_active, `维度列表只出激活行，实际 ${JSON.stringify(d)}`).toBe(true);
      expect(Number.isInteger(num(d.sort_order)), `维度 sort_order 应为整数`).toBe(true);
    }
    for (let i = 1; i < dims.length; i++) {
      expect(
        num(dims[i].sort_order) >= num(dims[i - 1].sort_order),
        `维度应按 sort_order 升序，实际第 ${i} 项 ${dims[i].sort_order} 小于前项 ${dims[i - 1].sort_order}`
      ).toBe(true);
    }
  });

  test('AA-02 余额恒等式与期间解析门：回显 恒等 逐文案拒绝', async ({ page }) => {
    const balance = await apiCallRaw<Row>(
      page,
      'GET',
      `/assist-accounting/balance?accounting_period=${EMPTY_PERIOD}&dimension_code=BATCH`
    );
    expect(str(balance.accounting_period), `余额应回显查询期间`).toBe(EMPTY_PERIOD);
    expect(str(balance.dimension_code), `余额应回显查询维度`).toBe('BATCH');
    expect(balance.dimension_value_id, `未限定维度值应回显 null`).toBe(null);
    // 恒等式独立于库内容成立：期初净额等于期初借贷之差，期末等于期初加本期净额
    expect(
      num(balance.opening_balance),
      `opening_balance 应等于 opening_debit 减 opening_credit，实际 ${JSON.stringify(balance)}`
    ).toBe(num(balance.opening_debit) - num(balance.opening_credit));
    expect(
      num(balance.closing_balance),
      `closing_balance 应等于 opening_balance 加 current_debit 减 current_credit，实际 ${JSON.stringify(balance)}`
    ).toBe(num(balance.opening_balance) + num(balance.current_debit) - num(balance.current_credit));

    // 期间解析门（validation_displayable，文案外显）逐字钉
    const badFormat = await apiCallExpectFail(
      page,
      'GET',
      '/assist-accounting/balance?accounting_period=209901&dimension_code=BATCH'
    );
    expect(badFormat.status, `无分隔期间应恰 400，实际 ${badFormat.status}`).toBe(400);
    expect(failureCode(badFormat)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      String(badFormat.message ?? ''),
      `displayable 出参应外显期间格式指引，实际 ${badFormat.message}`
    ).toContain('期间格式错误，应为 YYYY-MM');
    const badMonth = await apiCallExpectFail(
      page,
      'GET',
      '/assist-accounting/balance?accounting_period=2099-13&dimension_code=BATCH'
    );
    expect(badMonth.status).toBe(400);
    expect(failureCode(badMonth)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(String(badMonth.message ?? '')).toContain('月份必须在1-12之间');
    const badYear = await apiCallExpectFail(
      page,
      'GET',
      '/assist-accounting/balance?accounting_period=abcd-05&dimension_code=BATCH'
    );
    expect(badYear.status).toBe(400);
    expect(failureCode(badYear)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(String(badYear.message ?? '')).toContain('年份解析错误');

    // 必填参缺失走提取器拒绝统一信封（400 加 VALIDATION_ERROR，文案不锁）
    const missParam = await apiCallExpectFail(
      page,
      'GET',
      '/assist-accounting/balance?dimension_code=BATCH'
    );
    expect(missParam.status, `缺 accounting_period 应恰 400，实际 ${missParam.status}`).toBe(400);
    expect(failureCode(missParam)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
  });

  test('AA-03 主辅账核对：零窗口双零平衡与差异恒等式', async ({ page }) => {
    const wrapped = await apiCallRaw<Row>(
      page,
      'GET',
      `/assist-accounting/check-balance?period=${EMPTY_PERIOD}`
    );
    expect(str(wrapped.period), `外层应回显期间`).toBe(EMPTY_PERIOD);
    const result = wrapped.data as Row;
    expect(result, `核对结果体应存在，实际 ${JSON.stringify(wrapped)}`).toBeTruthy();
    expect(str(result.accounting_period), `内层应回显同一期间`).toBe(EMPTY_PERIOD);
    expect(
      typeof result.is_balanced,
      `is_balanced 应为布尔，实际 ${typeof result.is_balanced}`
    ).toBe('boolean');
    // 差异恒等式：debit_diff 与 credit_diff 必为两侧总额之差（与库内容无关）
    expect(
      num(result.debit_diff),
      `debit_diff 应等于辅助借方减总账借方，实际 ${JSON.stringify(result)}`
    ).toBe(num(result.assist_total_debit) - num(result.general_total_debit));
    expect(num(result.credit_diff)).toBe(
      num(result.assist_total_credit) - num(result.general_total_credit)
    );
    expect(
      result.is_balanced,
      `两侧差异同为 0 时 is_balanced 必为 true（同源自洽），实际 ${JSON.stringify(result)}`
    ).toBe(num(result.debit_diff) === 0 && num(result.credit_diff) === 0);
    // 远未来窗口两侧都无行：双总额应为数值零（非缺键非 NaN）
    for (const key of [
      'assist_total_debit',
      'assist_total_credit',
      'general_total_debit',
      'general_total_credit',
    ]) {
      expect(
        Number.isFinite(num(result[key])),
        `零窗口聚合键 ${key} 应为有限数值，实际 ${result[key]}`
      ).toBe(true);
      expect(num(result[key]), `零窗口 ${key} 应为 0，实际 ${result[key]}`).toBe(0);
    }

    const missPeriod = await apiCallExpectFail(page, 'GET', '/assist-accounting/check-balance');
    expect(missPeriod.status, `缺 period 应恰 400，实际 ${missPeriod.status}`).toBe(400);
    expect(failureCode(missPeriod)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
  });

  test('AA-04 穿透查询：不存在维度值零命中与分页 clamp 回显', async ({ page }) => {
    const drill = await apiCallRaw<Row>(
      page,
      'GET',
      `/assist-accounting/drill-down?accounting_period=${EMPTY_PERIOD}&dimension_code=CUSTOMER&dimension_value_id=999999&page=2&page_size=250`
    );
    const records = drill.records as Row[];
    expect(Array.isArray(records), `穿透应出 records 数组，实际 ${JSON.stringify(drill)}`).toBe(
      true
    );
    expect(records.length, `远未来期间加不存在维度值应零命中`).toBe(0);
    expect(num(drill.total), `穿透 total 应为 0`).toBe(0);
    expect(num(drill.page), `page 应回显提交值 2`).toBe(2);
    // page_size 服务端 clamp 到 100（handler 实现值 1 到 100），回显钉实现口径
    expect(num(drill.page_size), `page_size=250 应被 clamp 为 100`).toBe(100);

    // 三枚必填参缺一各钉一次（URL 整串写在调用点，判定器可解析位点）
    const missPeriod = await apiCallExpectFail(
      page,
      'GET',
      '/assist-accounting/drill-down?dimension_code=CUSTOMER&dimension_value_id=1'
    );
    expect(missPeriod.status, `缺 accounting_period 应恰 400，实际 ${missPeriod.status}`).toBe(400);
    expect(failureCode(missPeriod)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    const missDim = await apiCallExpectFail(
      page,
      'GET',
      '/assist-accounting/drill-down?accounting_period=2099-01&dimension_value_id=1'
    );
    expect(missDim.status, `缺 dimension_code 应恰 400，实际 ${missDim.status}`).toBe(400);
    expect(failureCode(missDim)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    const missValue = await apiCallExpectFail(
      page,
      'GET',
      '/assist-accounting/drill-down?accounting_period=2099-01&dimension_code=CUSTOMER'
    );
    expect(missValue.status, `缺 dimension_value_id 应恰 400，实际 ${missValue.status}`).toBe(400);
    expect(failureCode(missValue)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
  });

  test('AA-05 业务单与五维与汇总三读腿：唯一标记零命中 缺参拒绝 条件真值腿', async ({ page }) => {
    // 唯一标记不存在于库中：三枚读腿按各自过滤口径应零命中（过滤失效会漏回全量行）
    const mark = genCode('E2E-AA');
    const byBusiness = await apiCallRaw<Row[]>(
      page,
      'GET',
      `/assist-accounting/records/business?business_type=E2E_TYPE_${mark}&business_no=${mark}`
    );
    expect(
      Array.isArray(byBusiness),
      `业务单查询应出数组，实际 ${JSON.stringify(byBusiness)}`
    ).toBe(true);
    expect(byBusiness.length, `唯一标记业务单应零命中`).toBe(0);
    const byFiveDim = await apiCallRaw<Row[]>(
      page,
      'GET',
      `/assist-accounting/records/five-dimension/${mark}`
    );
    expect(Array.isArray(byFiveDim), `五维查询应出数组`).toBe(true);
    expect(byFiveDim.length, `唯一标记五维应零命中`).toBe(0);
    const summary = await apiCallRaw<Row[]>(
      page,
      'GET',
      `/assist-accounting/summary?accounting_period=${EMPTY_PERIOD}&dimension_code=BATCH`
    );
    expect(Array.isArray(summary), `汇总应出数组`).toBe(true);
    expect(summary.length, `远未来期间汇总应零命中`).toBe(0);
    // 汇总实现分支：dimension_code 缺省直接空集（不查库），钉短路行为
    const summaryNoDim = await apiCallRaw<Row[]>(
      page,
      'GET',
      `/assist-accounting/summary?accounting_period=${EMPTY_PERIOD}`
    );
    expect(summaryNoDim.length, `缺 dimension_code 应走空集短路`).toBe(0);

    // 缺参拒绝：business 两参必填，summary 期间必填
    const failBiz = await apiCallExpectFail(
      page,
      'GET',
      `/assist-accounting/records/business?business_no=${mark}`
    );
    expect(failBiz.status, `缺 business_type 应恰 400，实际 ${failBiz.status}`).toBe(400);
    expect(failureCode(failBiz)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    const failSummary = await apiCallExpectFail(page, 'GET', '/assist-accounting/summary');
    expect(failSummary.status).toBe(400);
    expect(failureCode(failSummary)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 条件真值腿：库里已有辅助核算记录时，按真实行回查业务单与五维两腿，
    // 钉过滤只命中同键行；库为空时该腿如实跳过（不假装有数据契约）
    const pageData = await apiCallRaw<Row>(
      page,
      'GET',
      '/assist-accounting/records?page=1&page_size=1'
    );
    const first = (pageData.records as Row[] | undefined)?.[0];
    if (first) {
      const realBiz = await apiCallRaw<Row[]>(
        page,
        'GET',
        `/assist-accounting/records/business?business_type=${encodeURIComponent(str(first.business_type))}&business_no=${encodeURIComponent(str(first.business_no))}`
      );
      expect(
        realBiz.some(r => num(r.id) === num(first.id)),
        `真实行业务单回查应命中该行 id ${first.id}，实际 ${JSON.stringify(realBiz.map(r => r.id))}`
      ).toBe(true);
      for (const r of realBiz) {
        expect(str(r.business_no), `业务单回查结果行不得漏回他单`).toBe(str(first.business_no));
        expect(str(r.business_type), `业务单回查结果行业务类型同源`).toBe(str(first.business_type));
      }
      if (str(first.five_dimension_id).length > 0) {
        const realFd = await apiCallRaw<Row[]>(
          page,
          'GET',
          `/assist-accounting/records/five-dimension/${encodeURIComponent(str(first.five_dimension_id))}`
        );
        expect(
          realFd.some(r => num(r.id) === num(first.id)),
          `真实行五维回查应命中该行`
        ).toBe(true);
        for (const r of realFd) {
          expect(str(r.five_dimension_id), `五维回查结果行不得漏回他维`).toBe(
            str(first.five_dimension_id)
          );
        }
      }
      // records 列表腿信封（本域已有覆盖，这里只借回读通道）：
      // 真值不变式 total（全库计数）不少于当页行数
      const listed = pageData.records as Row[];
      expect(
        num(pageData.total),
        `total 不少于当页行数，实际 ${pageData.total} 对 ${listed.length}`
      ).toBeGreaterThanOrEqual(listed.length);
    }
  });
});

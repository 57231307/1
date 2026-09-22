// 质检记录列表的筛选契约冒烟测试
//
// 此前该端点收下 product_id/batch_number 却一个都不参与查询，还把 inspection_result 塞进
// 名为 inspection_type 的字段里去过滤另一列；前端列表又读不存在的 row.result，整页没有可用筛选。
// 本用例钉住修好后的四条真实契约（不依赖库里预置数据——该表只由运行期写入）：
//   1. 出参字段名与后端 Model 一致（inspection_no / inspection_result / product_id / inspector_id）
//   2. 行内 inspection_type 与 inspection_result 都落在后端取值域内
//   3. 每个筛选参数都真下推：按某值筛选返回的每一行都必须等于该值
//   4. 越界取值被 4xx 拒绝并在错误信息里列出允许值（不筛选的假控件会放行成「查无数据」）
import { test, expect } from '../diagnose-fixture';
import type { APIRequestContext } from '@playwright/test';

// 与前端 src/api/request.ts 同源：经 vite/preview 的 /api/ 代理转发到后端
const API_PREFIX = '/api/v1/erp';
const RECORDS_URL = `${API_PREFIX}/production/quality-inspection/records`;

/** 后端 quality_inspection_type::ALL（四个界面码 + 委外回仓自动写入的来源标识） */
const INSPECTION_TYPES = ['incoming', 'process', 'finished', 'outgoing', 'outsourcing_receipt'];

/** 后端 quality_inspection_result::ALL */
const INSPECTION_RESULTS = ['待检', '合格', '不合格'];

interface RecordRow {
  id: number;
  inspection_no: string;
  inspection_type: string;
  inspection_result: string;
  product_id: number;
  inspector_id: number | null;
  batch_no: string | null;
  total_qty: string | number;
  inspected_qty: string | number;
}

interface RecordListData {
  items: RecordRow[];
  total: number;
  page: number;
  page_size: number;
}

async function listRecords(
  request: APIRequestContext,
  query: string
): Promise<{ status: number; data: RecordListData | undefined; body: string }> {
  const res = await request.get(`${RECORDS_URL}?${query}`);
  const text = await res.text();
  let data: RecordListData | undefined;
  try {
    const parsed = JSON.parse(text) as { data?: RecordListData };
    data = parsed.data;
  } catch {
    // 非 JSON 响应交给断言处理，这里不静默吞掉
  }
  return { status: res.status(), data, body: text };
}

/** 错误响应的稳定错误码。对外 message 已统一脱敏，能判定的只有 code；解析失败时把原文
 *  作为返回值带进断言消息，不在这里静默吞掉 */
function errorCode(body: string): string {
  try {
    return String((JSON.parse(body) as { code?: unknown }).code ?? '');
  } catch {
    return `<非 JSON 响应: ${body.slice(0, 120)}>`;
  }
}

test.describe('质检记录列表筛选契约', () => {
  test('出参字段与后端模型一致，且行内取值都在词表内', async ({ request }) => {
    const { status, data } = await listRecords(request, 'page=1&page_size=5');
    expect(status).toBe(200);
    expect(data, '列表响应缺少 data 对象').toBeTruthy();
    const payload = data as RecordListData;
    // 缺 items 键与 items=[] 分开判（缺键=后端契约破坏，非空集合）
    expect(
      Object.prototype.hasOwnProperty.call(payload, 'items'),
      '响应缺少 items 字段（后端未返回该键）'
    ).toBe(true);
    expect(Array.isArray(payload.items), 'items 必须是数组').toBe(true);
    expect(typeof payload.total).toBe('number');
    expect(payload.page).toBe(1);
    expect(payload.page_size).toBe(5);
    // 真实内容一致性：本页行数 ≤ total；total 非空时首页必须带行
    expect(
      payload.items.length,
      `首页行数 ${payload.items.length} 不应超过 total ${payload.total}`
    ).toBeLessThanOrEqual(payload.total);
    if (payload.total > 0) {
      expect(
        payload.items.length,
        `total=${payload.total} 但首页 items 为空，列表分页与数据不一致`
      ).toBeGreaterThan(0);
    }
    for (const row of payload.items) {
      expect(typeof row.inspection_no).toBe('string');
      expect(row.inspection_no.length, '检验单号不能为空串').toBeGreaterThan(0);
      expect(typeof row.product_id).toBe('number');
      expect(INSPECTION_TYPES, `出现取值域外的检验类型：${row.inspection_type}`).toContain(
        row.inspection_type
      );
      expect(INSPECTION_RESULTS, `出现取值域外的检验结论：${row.inspection_result}`).toContain(
        row.inspection_result
      );
    }
  });

  test('结论与类型筛选都真正下推', async ({ request }) => {
    for (const result of INSPECTION_RESULTS) {
      const { status, data } = await listRecords(
        request,
        `page=1&page_size=50&inspection_result=${encodeURIComponent(result)}`
      );
      expect(status).toBe(200);
      for (const row of (data as RecordListData).items) {
        expect(row.inspection_result, `结论筛选未下推，混入：${row.inspection_result}`).toBe(
          result
        );
      }
    }
    for (const type of INSPECTION_TYPES) {
      const { status, data } = await listRecords(
        request,
        `page=1&page_size=50&inspection_type=${type}`
      );
      expect(status).toBe(200);
      for (const row of (data as RecordListData).items) {
        expect(row.inspection_type, `类型筛选未下推，混入：${row.inspection_type}`).toBe(type);
      }
    }
  });

  test('产品与批号筛选生效：命中行必等于所筛条件', async ({ request }) => {
    const seed = await listRecords(request, 'page=1&page_size=1');
    const rows = (seed.data as RecordListData).items;
    if (rows.length === 0) {
      // 该表只由运行期写入，CI 库可能确实没有记录；此时下推性由越界拒绝与结论/类型两测覆盖
      expect(seed.status).toBe(200);
      return;
    }
    const row = rows[0];
    const byProduct = await listRecords(
      request,
      `page=1&page_size=50&product_id=${row.product_id}`
    );
    expect(byProduct.status).toBe(200);
    for (const hit of (byProduct.data as RecordListData).items) {
      expect(hit.product_id).toBe(row.product_id);
    }
    if (row.batch_no) {
      const byBatch = await listRecords(
        request,
        `page=1&page_size=50&batch_no=${encodeURIComponent(row.batch_no)}`
      );
      expect(byBatch.status).toBe(200);
      for (const hit of (byBatch.data as RecordListData).items) {
        expect(hit.batch_no, `批号模糊筛选混入不匹配行：${hit.batch_no}`).toContain(
          row.batch_no as string
        );
      }
    }
  });

  test('越界筛选值被拒绝（稳定错误码），而不是静默返回空集', async ({ request }) => {
    // 断言的是错误码而不是文案：AppError 的对外 message 统一脱敏（漏洞 #4/#8/#12 修复），
    // 允许值清单只进服务端 detail 日志。"拒绝信息必须列出合法值"这条由
    // backend/tests/handlers_quality_inspection_result_test.rs 钉住。
    const badResult = await listRecords(request, 'page=1&page_size=10&inspection_result=pass');
    expect(badResult.status, '旧前端词表 pass 应被取值域校验拒绝').toBe(400);
    expect(
      errorCode(badResult.body),
      `越界结论应返回 VALIDATION_ERROR，实际 body：${badResult.body}`
    ).toBe('VALIDATION_ERROR');

    const badType = await listRecords(request, 'page=1&page_size=10&inspection_type=inprocess');
    expect(badType.status, 'ai_quality_predictions 那套词表不得用于本列').toBe(400);
    expect(errorCode(badType.body)).toBe('VALIDATION_ERROR');

    // 不受影响的正常查询仍能跑通（证明拒绝来自校验而非查询整体失败）
    const ok = await listRecords(request, 'page=1&page_size=10');
    expect(ok.status).toBe(200);
  });
});

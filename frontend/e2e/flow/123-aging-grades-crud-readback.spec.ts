import { test, expect } from '../diagnose-fixture';
import {
  APP_ERROR_CODES,
  loginViaUI,
  apiCall,
  apiCallExpectFail,
  deferCleanup,
  flushDeferredCleanups,
  failureCode,
  type DeferredCleanup,
} from './helpers';

/**
 * 123 账龄档位配置端点族的增删改查读回（finance 域，aging-grades 族）
 *
 * 功能：守住账龄档位配置面（表 aging_grade_configs）确实把提交值落库并能读回，
 * 覆盖以下五个真实端点，均由 aging_grade_handler 产出：
 * - GET /aging-grades 列表（返回 Vec，非分页信封；后端按 is_active=true 过滤并按 sort_order 升序）。
 * - POST /aging-grades 建档位，grade_name/min_days/max_days 必填；
 *   sort_order 缺省 0、is_active 缺省 true、remark 缺省 null。
 *   未提交列由 handler:38 unwrap_or(0) 与 unwrap_or(true) 兜底。
 * - GET /aging-grades/{id} 复用 list_aging_grades 处理（handler 不提取路径 id），
 *   返回与 GET /aging-grades 相同的全量启用列表。本用例仍向该路径发请求以满足端点覆盖。
 * - PUT /aging-grades/{id} 部分更新，只覆盖请求体里出现的列，未出现的列保持原值；
 *   id 不存在返回 404 且码为 NOT_FOUND。
 * - DELETE /aging-grades/{id} 硬删除（SeaORM active.delete），删后按 id 不再出现在列表中。
 *
 * 调用方：routes/finance.rs:1195-1208 注册，handler 在 handlers/aging_grade_handler.rs。
 * 入参：CreateAgingGradeDto 必填 grade_name(String) min_days(i32) max_days(i32)；
 *   可选 sort_order(i32) is_active(bool) remark(String)。
 * 传给谁：aging_grade_config::ActiveModel → sea_orm → PostgreSQL。
 * 存什么：id grade_name min_days max_days sort_order is_active remark created_at updated_at。
 * 存哪里：表 aging_grade_configs，路由前缀 /api/v1/erp/aging-grades。
 *
 * 断言均为值判据，不用恒真、不只判 ok()。
 */

type GradeRow = {
  id: number;
  grade_name: string;
  min_days: number;
  max_days: number;
  sort_order: number;
  is_active: boolean;
  remark: string | null;
};

function uniqTag(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
}

test.describe.serial('123 账龄档位配置增删改查读回', () => {
  const cleanup: DeferredCleanup[] = [];

  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test.afterEach(async ({ page }) => {
    await flushDeferredCleanups(page, cleanup);
  });

  test('POST 建档位后由列表读回逐键持久，未提交列落后端默认', async ({ page }) => {
    const tag = uniqTag();
    const name = `e2e档位-${tag}`;

    const created = await apiCall<GradeRow>(page, 'POST', '/aging-grades', {
      grade_name: name,
      min_days: 10,
      max_days: 20,
    });
    const id = created.data.id;
    expect(Number.isInteger(id) && id > 0, `创建应返回正整数 id，实际：${String(id)}`).toBe(true);
    deferCleanup(cleanup, 'DELETE', `/aging-grades/${id}`, `ag-${tag}`);

    expect(created.data.grade_name).toBe(name);
    expect(created.data.min_days).toBe(10);
    expect(created.data.max_days).toBe(20);
    expect(created.data.sort_order, '未提交 sort_order 应落后端默认 0').toBe(0);
    expect(created.data.is_active, '未提交 is_active 应落后端默认 true').toBe(true);
    expect(created.data.remark, '未提交 remark 应为 null').toBeNull();

    // 从列表读回（证明落库，不只 POST 回显）。
    const listed = await apiCall<GradeRow[]>(page, 'GET', '/aging-grades');
    const hit = listed.data.find(r => r.id === id);
    expect(hit, `列表应包含刚建档位 id=${id}`).toBeDefined();
    expect(hit!.grade_name).toBe(name);
    expect(hit!.min_days).toBe(10);
    expect(hit!.max_days).toBe(20);
  });

  test('GET /aging-grades/{id} 仍返回启用档位列表', async ({ page }) => {
    const tag = uniqTag();
    const created = await apiCall<GradeRow>(page, 'POST', '/aging-grades', {
      grade_name: `e2e-byid-${tag}`,
      min_days: 31,
      max_days: 60,
      sort_order: 3,
      remark: `e2e-remark-${tag}`,
    });
    const id = created.data.id;
    deferCleanup(cleanup, 'DELETE', `/aging-grades/${id}`, `ag-byid-${tag}`);

    // 该路径复用 list 处理器（handler 不提取 id），返回启用档位数组。
    const byId = await apiCall<GradeRow[]>(page, 'GET', `/aging-grades/${id}`);
    expect(Array.isArray(byId.data), 'GET /{id} 应返回数组').toBe(true);
    const hit = byId.data.find(r => r.id === id);
    expect(hit, `列表应包含刚建档位 id=${id}`).toBeDefined();
    expect(hit!.grade_name).toBe(`e2e-byid-${tag}`);
    expect(hit!.sort_order).toBe(3);
    expect(hit!.remark).toBe(`e2e-remark-${tag}`);
  });

  test('PUT 部分更新后旧值变新值，缺席列保持原值', async ({ page }) => {
    const tag = uniqTag();
    const created = await apiCall<GradeRow>(page, 'POST', '/aging-grades', {
      grade_name: `原始档位-${tag}`,
      min_days: 61,
      max_days: 90,
      sort_order: 5,
      remark: '原始备注',
    });
    const id = created.data.id;
    deferCleanup(cleanup, 'DELETE', `/aging-grades/${id}`, `ag-upd-${tag}`);

    const updated = await apiCall<GradeRow>(page, 'PUT', `/aging-grades/${id}`, {
      grade_name: `更新档位-${tag}`,
      max_days: 120,
    });
    expect(updated.data.grade_name).toBe(`更新档位-${tag}`);
    expect(updated.data.max_days).toBe(120);
    // 缺席列保持原值
    expect(updated.data.min_days, '缺席的 min_days 应保持原值 61').toBe(61);
    expect(updated.data.sort_order, '缺席的 sort_order 应保持原值 5').toBe(5);

    // 从列表读回确认持久
    const listed = await apiCall<GradeRow[]>(page, 'GET', '/aging-grades');
    const hit = listed.data.find(r => r.id === id);
    expect(hit!.grade_name).toBe(`更新档位-${tag}`);
    expect(hit!.max_days).toBe(120);
    expect(hit!.min_days).toBe(61);
  });

  test('DELETE 硬删后 id 不再出现在列表中', async ({ page }) => {
    const tag = uniqTag();
    const created = await apiCall<GradeRow>(page, 'POST', '/aging-grades', {
      grade_name: `待删档位-${tag}`,
      min_days: 999,
      max_days: 999,
    });
    const id = created.data.id;

    const delRes = await apiCall<string>(page, 'DELETE', `/aging-grades/${id}`);
    expect(delRes.code, '删除应返回 200').toBe(200);

    // 列表不应包含已删行
    const listed = await apiCall<GradeRow[]>(page, 'GET', '/aging-grades');
    const gone = listed.data.find(r => r.id === id);
    expect(gone, `删除后 id=${id} 不应再出现在列表中`).toBeUndefined();
  });

  test('PUT 不存在的 id 返回 404 且码为 NOT_FOUND', async ({ page }) => {
    const fail = await apiCallExpectFail(page, 'PUT', '/aging-grades/999999999', {
      grade_name: 'ghost',
    });
    expect(fail.status).toBe(404);
    expect(failureCode(fail)).toBe(APP_ERROR_CODES.NOT_FOUND);
  });

  test('DELETE 不存在的 id 返回 404 且码为 NOT_FOUND', async ({ page }) => {
    const fail = await apiCallExpectFail(page, 'DELETE', '/aging-grades/999999998');
    expect(fail.status).toBe(404);
    expect(failureCode(fail)).toBe(APP_ERROR_CODES.NOT_FOUND);
  });
});

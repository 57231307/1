// 8D 报告列表读口契约 — 05 分页回显、越界 clamp 零漂移与不存在 id 负例
// 契约真相（逐段读后端源，非推测）：
//   GET quality-8d-reports（handler handlers/quality_8d_handler.rs list_8d，
//   service services/quality_8d_service.rs）出参 EightDPagedResponse，键 items、total、
//   page、page_size；page 缺省 1 并 clamp 到 1..1000，page_size 缺省 20 并 clamp 到
//   1..200，回显值是被 clamp 后的实际取页数，不是提交原值。行集经数据范围 ctx 过滤。
//   GET quality-8d-reports/{id}（get_8d 与 get_by_id）id 不存在时先于
//   归属门命中 EightDError::NotFound，eight_d_err 映射点 AppError::not_found，
//   出 HTTP 404 与机器码 NOT_FOUND，默认脱敏不读文案。
// 正向与零漂移：page=2、page_size=10 请求回显逐键等值；首页全窗（page=1、page_size=200）
//   下行数等于 total（total<=200 时）或恰为窗宽 200（超出时），行数与总数互相锁死，
//   任何一侧掺水即红。越界请求 page=0 回显 1、page=99999 回显 1000、page_size=0 回显 1、
//   page_size=999 回显 200，钉住 clamp 现行为防口径漂移。
// 假绿防线：不以 200 或小于 500 当健康，逐键断言回显值；负例钉 404 与 NOT_FOUND 双判据。
import { test, expect } from '../diagnose-fixture';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCallRaw, apiCallExpectFail, failureCode } from '../flow/helpers';

interface EightDPage {
  items: unknown[];
  total: number;
  page: number;
  page_size: number;
}

test.describe('05 8D 列表读口契约', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('05-01 分页参数逐键回显且首页全窗行数与 total 互锁', async ({ page }) => {
    const win = await apiCallRaw<EightDPage>(
      page,
      'GET',
      '/quality-8d-reports?page=2&page_size=10'
    );
    expect(Array.isArray(win.items), `items 应为数组，实际 ${JSON.stringify(win)}`).toBe(true);
    expect(Number(win.page), `page 应回显提交值 2，实际 ${win.page}`).toBe(2);
    expect(Number(win.page_size), `page_size 应回显提交值 10，实际 ${win.page_size}`).toBe(10);
    expect(
      Number.isInteger(Number(win.total)) && Number(win.total) >= 0,
      `total 应为非负整数，实际 ${win.total}`
    ).toBe(true);
    expect(
      win.items.length,
      `第 2 页窗宽 10 下行数不得超过 10，实际 ${win.items.length}`
    ).toBeLessThanOrEqual(10);

    // 全窗一致性：第一页取满 200（clamp 上限），行数必等于 min(total, 200)
    const full = await apiCallRaw<EightDPage>(
      page,
      'GET',
      '/quality-8d-reports?page=1&page_size=200'
    );
    const total = Number(full.total);
    expect(
      full.items.length,
      `首页行数应等于 min(total,200)，total=${total}，实际 ${full.items.length}`
    ).toBe(Math.min(total, 200));
  });

  test('05-02 越界分页参数按 clamp 现行为回显（零漂移钉值）', async ({ page }) => {
    const low = await apiCallRaw<EightDPage>(page, 'GET', '/quality-8d-reports?page=0&page_size=0');
    expect(Number(low.page), `page=0 应被 clamp 为 1，实际回显 ${low.page}`).toBe(1);
    expect(Number(low.page_size), `page_size=0 应被 clamp 为 1，实际回显 ${low.page_size}`).toBe(1);

    const high = await apiCallRaw<EightDPage>(
      page,
      'GET',
      '/quality-8d-reports?page=99999&page_size=999'
    );
    expect(Number(high.page), `page=99999 应被 clamp 为 1000，实际回显 ${high.page}`).toBe(1000);
    expect(
      Number(high.page_size),
      `page_size=999 应被 clamp 为 200，实际回显 ${high.page_size}`
    ).toBe(200);
  });

  test('05-03 不存在的 8D id 详情口 404 与机器码 NOT_FOUND', async ({ page }) => {
    // service get_by_id find_by_id 无行即 EightDError::NotFound，先于行级归属门；
    // not_found 默认脱敏，只钉 HTTP 码与 failureCode 双判据，不读文案。
    const fail = await apiCallExpectFail(page, 'GET', '/quality-8d-reports/999999999');
    expect(fail.status, `不存在 id 应 404，实际 ${fail.status}`).toBe(404);
    expect(failureCode(fail), `应 NOT_FOUND，实际 code=${fail.code}`).toBe('NOT_FOUND');
  });
});

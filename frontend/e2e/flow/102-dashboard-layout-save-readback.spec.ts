import { test, expect } from '../diagnose-fixture';
import { loginViaUI, apiCall, apiCallRaw, apiCallExpectFail, APP_ERROR_CODES } from './helpers';

/**
 * 102 仪表板布局配置的保存与读回（dashboard 域，layout GET/PUT 族）
 *
 * 覆盖两个真实端点（routes/system.rs dashboard 挂载，权限键按 URL 段取 dashboard/layout）：
 * - PUT /dashboard/layout 保存当前登录用户的布局，入参 card_config 必填（任意 JSON），
 *   is_default 可选布尔、缺省落 false（service save_dashboard_layout 取 unwrap_or(false)），
 *   同一 user_id 已存在则整行覆盖（先查后改，无版本累积），返回更新后的整行。
 * - GET /dashboard/layout 取当前登录用户的布局，出参是单对象（user_id、card_config、is_default、
 *   updated_at），无该用户行时返回内置默认（is_default 为 true、cards 为四张默认卡）。
 *
 * 为什么按 user_id 隔离可安全读写：布局以 user_id 为唯一键存于 dashboard_layouts 表，
 * 每个 e2e worker 以各自账号登录（helpers testUsername 取 worker 身份），彼此不共享行；
 * 全仓 e2e 仅此规格写 layout，无并发规格改同一行，故写后立即读即拿到本次写入值。
 *
 * 断言均为值判据（写什么 card_config 读回什么、数组顺序原样保留、is_default 布尔翻转读回、
 * 覆盖后旧值消失），不用只判 200、不只判元素存在。负例钉到具体 HTTP 码与机器码。
 */

// 布局单行（DashboardLayoutResponse 的 snake_case 键）。
type Layout = {
  user_id: number;
  card_config: unknown;
  is_default: boolean;
  updated_at: string;
};

function uniq(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
}

test.describe.serial('102 仪表板布局保存与读回', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test.afterEach(async ({ page }) => {
    // 布局无删除端点且按 user 单行存储，交回内置默认形态，避免把探针配置留给同账号的后续页面用例。
    // 恢复失败只告警不吞判据：本用例的断言在测试体内已完成。
    await apiCall(page, 'PUT', '/dashboard/layout', {
      card_config: { cards: ['overview', 'sales', 'inventory', 'low_stock_alerts'] },
      is_default: true,
    }).catch(e => {
      console.warn(`[102] 布局恢复默认失败: ${(e as Error).message}`);
    });
  });

  test('写入自定义 card_config 后 GET 逐键读回且 is_default 缺省为 false', async ({ page }) => {
    const tag = uniq();
    const config = { cards: ['alpha', 'beta'], order: 7, marker: `cfg-${tag}` };

    const saved = await apiCallRaw<Layout>(page, 'PUT', '/dashboard/layout', {
      card_config: config,
    });
    expect(
      saved.is_default,
      `未提交 is_default 应落 false，实际：${String(saved.is_default)}`
    ).toBe(false);
    expect(
      saved.card_config,
      `保存回显的 card_config 应与提交值逐键相等，实际：${JSON.stringify(saved.card_config)}`
    ).toEqual(config);
    expect(Number.isInteger(saved.user_id) && saved.user_id > 0).toBe(true);

    // 从库经单条接口读回，证明落库而非仅在写接口回显。
    const readBack = await apiCallRaw<Layout>(page, 'GET', '/dashboard/layout');
    expect(
      readBack.card_config,
      `读回 card_config 应与保存值一致，实际：${JSON.stringify(readBack.card_config)}`
    ).toEqual(config);
    expect(
      readBack.is_default,
      `读回 is_default 应为 false，实际：${String(readBack.is_default)}`
    ).toBe(false);
    expect(readBack.user_id, `读回 user_id 应与保存回显同源`).toBe(saved.user_id);
  });

  test('再次保存整行覆盖旧值，读回只见新 card_config 与新 is_default', async ({ page }) => {
    const tag = uniq();
    const first = { marker: `old-${tag}`, cards: ['one'] };
    await apiCallRaw<Layout>(page, 'PUT', '/dashboard/layout', {
      card_config: first,
      is_default: false,
    });

    const second = { marker: `new-${tag}`, cards: ['two', 'three'] };
    await apiCallRaw<Layout>(page, 'PUT', '/dashboard/layout', {
      card_config: second,
      is_default: true,
    });

    const readBack = await apiCallRaw<Layout>(page, 'GET', '/dashboard/layout');
    expect(
      readBack.card_config,
      `覆盖后读回应为第二次写入值，实际：${JSON.stringify(readBack.card_config)}`
    ).toEqual(second);
    // 旧 marker 不再存在（证明确是整行覆盖，不是合并追加）。
    expect(
      JSON.stringify(readBack.card_config).includes(`old-${tag}`),
      `覆盖后旧 card_config 不应残留`
    ).toBe(false);
    expect(
      readBack.is_default,
      `第二次提交 is_default true 应读回 true，实际：${String(readBack.is_default)}`
    ).toBe(true);
  });

  test('card_config 为数组时顺序原样读回（后端不重排）', async ({ page }) => {
    const tag = uniq();
    const ordered = [`z-${tag}`, `a-${tag}`, `m-${tag}`, `b-${tag}`];

    await apiCallRaw<Layout>(page, 'PUT', '/dashboard/layout', {
      card_config: { cards: ordered },
    });

    const readBack = await apiCallRaw<Layout>(page, 'GET', '/dashboard/layout');
    const got = (readBack.card_config as { cards: string[] }).cards;
    expect(got, `数组顺序应逐元素原样读回，实际：${JSON.stringify(got)}`).toEqual(ordered);
    expect(got[0]).toBe(`z-${tag}`);
    expect(got[3]).toBe(`b-${tag}`);
  });

  test('缺 card_config 的保存被拒 400 且码为 VALIDATION_ERROR', async ({ page }) => {
    const fail = await apiCallExpectFail(page, 'PUT', '/dashboard/layout', {
      is_default: true,
    });
    expect(fail.status, `缺 card_config 应 400，实际 status=${fail.status} code=${fail.code}`).toBe(
      400
    );
    expect(fail.code, `缺 card_config 的码应为 VALIDATION_ERROR，实际：${String(fail.code)}`).toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
  });
});

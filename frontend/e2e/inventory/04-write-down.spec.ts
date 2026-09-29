// 库存管理 E2E 套件 — 04 存货跌价准备（计提 → 确认 → 账面价值回读）
// 覆盖路由：routes/inventory.rs:248-268 write_down_routes
//   POST /inventory/write-downs        （create_write_down）
//   GET  /inventory/write-downs/{id}   （get_write_down，写后回读通道）
//   POST /inventory/write-downs/{id}/confirm（confirm_write_down）
// 账价值口径（以 handler/service 源码为准，不虚构）：
//   create 时 write_down_amount = original_cost − net_realizable_value
//   （inventory_write_down_service.rs:52），即"成本 − 可变现净值"的账面跌价额；
//   等式：original_cost = net_realizable_value + write_down_amount。
//   成本 10000、可变现净值 7500 → 账面价值自 10000 降至 7500，跌价 2500。
// 诚实标注（交判责，不作为断言放宽）：
//   confirm 仅迁移状态 draft→confirmed 并落 confirmed_by/confirmed_at
//   （service.rs:63-71），当前实现不回写库存计价、不生凭证、不动其他表——
//   本用例钉"记录级账面价值等式 + 状态迁移"的实现真相；若业务要求确认联动
//   存货账面/凭证，属源码缺口，禁止在此测试里造假象断言。
//   另：confirm 无状态门（对已 confirmed 记录重复 confirm 仍 2xx）、create 无
//   original_cost ≥ net_realizable_value 边界校验（负跌价额可落库），均为已知
//   未覆盖风险点，见交付报告；不在本 spec 断言，避免把缺陷行为钉成正契约。
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  ensureTestEntities,
  getCtx,
  genCode,
} from '../flow/helpers';

const toDec = (v: unknown): number => Number(String(v));

test.describe('库存管理 - 04 存货跌价准备', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('跌价准备：POST 创建(draft)→回读→confirm→回读账面价值等式', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    expect(productId, '前置：跌价准备需至少一个产品').toBeTruthy();

    const originalCost = 10000;
    const nrv = 7500;
    const period = new Date().toISOString().slice(0, 10);
    const reason = `E2E 跌价准备贯穿 ${genCode('WD')}`;

    // ---- 1. 创建：write_down_amount 应由后端按 成本−NRV 计算落库 ----
    const created = await apiCall<{ id?: number; status?: string }>(
      page,
      'POST',
      '/inventory/write-downs',
      {
        product_id: productId,
        write_down_type: '呆滞面料',
        original_cost: String(originalCost),
        net_realizable_value: String(nrv),
        reason,
        period,
      }
    );
    const wdId = created.data?.id;
    expect(
      wdId,
      `创建跌价准备应返回 data.id，实际：${JSON.stringify(created).slice(0, 200)}`
    ).toBeTruthy();

    // 写后必回读（GET 详情，非响应回声）：初始状态 draft（service.rs:55 写入值，小写）
    const draft = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/inventory/write-downs/${wdId}`
    );
    expect(String(draft.status), `回读：新建跌价准备初始态应为 draft，实际 ${draft.status}`).toBe(
      'draft'
    );
    expect(toDec(draft.original_cost), '回读：成本应与提交一致').toBeCloseTo(originalCost, 2);
    expect(toDec(draft.net_realizable_value), '回读：可变现净值应与提交一致').toBeCloseTo(nrv, 2);
    expect(
      toDec(draft.write_down_amount),
      `回读：后端应落 write_down_amount=成本−NRV=${originalCost - nrv}，实际 ${draft.write_down_amount}`
    ).toBeCloseTo(originalCost - nrv, 2);
    expect(draft.confirmed_by, '回读：draft 阶段 confirmed_by 应为空').toBeFalsy();

    // ---- 2. 确认：状态 draft→confirmed，confirmed_by/confirmed_at 落值 ----
    await apiCall(page, 'POST', `/inventory/write-downs/${wdId}/confirm`);

    // 写后必回读
    const confirmedRow = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/inventory/write-downs/${wdId}`
    );
    expect(
      String(confirmedRow.status),
      `回读：确认后状态应为 confirmed，实际 ${confirmedRow.status}`
    ).toBe('confirmed');
    expect(
      Number(confirmedRow.confirmed_by),
      '回读：确认应落操作人 confirmed_by（handler 传 auth.user_id）'
    ).toBeGreaterThan(0);
    expect(confirmedRow.confirmed_at, '回读：确认应落 confirmed_at').toBeTruthy();

    // ---- 3. 账面价值等式（确认前后金额守恒、价值口径钉死）----
    // 等式：original_cost(成本 10000) = net_realizable_value(账面价值 7500) + write_down_amount(跌价 2500)
    // 即账面价值由成本 10000 记减至可变现净值 7500，跌价准备 2500；确认动作不篡改三金额。
    expect(toDec(confirmedRow.original_cost), '确认不应改变成本').toBeCloseTo(
      toDec(draft.original_cost),
      2
    );
    expect(
      toDec(confirmedRow.net_realizable_value),
      '确认不应改变可变现净值（账面价值）'
    ).toBeCloseTo(toDec(draft.net_realizable_value), 2);
    expect(toDec(confirmedRow.write_down_amount), '确认不应改变跌价准备金额').toBeCloseTo(
      toDec(draft.write_down_amount),
      2
    );
    expect(
      toDec(confirmedRow.original_cost),
      `账面价值等式应成立：成本(${confirmedRow.original_cost}) == NRV(${confirmedRow.net_realizable_value}) + 跌价额(${confirmedRow.write_down_amount})`
    ).toBeCloseTo(
      toDec(confirmedRow.net_realizable_value) + toDec(confirmedRow.write_down_amount),
      2
    );
  });

  test('跌价准备必填缺失：创建被 4xx 拒绝（不静默落库）', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    expect(productId, '前置：跌价准备需至少一个产品').toBeTruthy();

    // 缺 net_realizable_value（CreateWriteDownPayload 必填字段，handler 未调 Validator，
    // 由 serde 反序列化拒绝——axum Json 拒绝为 422）：钉"必填缺失必拒"，不钉具体 400/422
    // 以免把 deserialization 层位置写死成假契约；5xx 视为裸崩判红。
    const rejected = await apiCallExpectFail(page, 'POST', '/inventory/write-downs', {
      product_id: productId,
      write_down_type: '呆滞面料',
      original_cost: '1000',
      reason: 'E2E 缺字段负例',
      period: new Date().toISOString().slice(0, 10),
    });
    console.warn(
      `[跌价必填] 缺 net_realizable_value：status=${rejected.status} code=${rejected.code} message=${rejected.message}`
    );
    expect(
      rejected.status,
      `必填缺失应被 4xx 拒绝（实际 ${rejected.status}，若为 5xx 属反序列化裸崩判红）`
    ).toBeGreaterThanOrEqual(400);
    expect(rejected.status, '必填缺失不应升级为 5xx').toBeLessThan(500);
  });

  test('跌价准备确认非法目标：不存在的 id 应 404（不静默成功）', async ({ page }) => {
    // get_by_id → AppError::not_found（service.rs:39-44），confirm 前置依赖它。
    // 用一个必然不存在的大 id 钉"确认不可凭空成功"；回读通道即 GET 详情本身。
    const ghostId = 999_999_999;
    const fail = await apiCallExpectFail(page, 'POST', `/inventory/write-downs/${ghostId}/confirm`);
    console.warn(`[跌价404] confirm id=${ghostId}：status=${fail.status} message=${fail.message}`);
    expect(
      fail.status,
      `确认不存在的跌价准备记录应 404（实际 ${fail.status}）——不应静默 2xx 或裸 500`
    ).toBe(404);
  });
});

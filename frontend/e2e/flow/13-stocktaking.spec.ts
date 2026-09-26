import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  genCode,
  getCtx,
  verifyStockFourDim,
  verifyAuditLog,
  ensureTestEntities,
  failureCode,
  APP_ERROR_CODES,
} from './helpers';

test.describe('库存盘点完整流程', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('盘点：创建→录入实盘→提交→审批→调整验证', async ({ page }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];

    // 记录盘点前库存行（同一行提供仓库、库存 ID 与调整前可用量）
    // 仓库取实际库存行自带的 warehouse_id：ensureTestEntities 每次 beforeEach
    // 重查仓库列表，ctx.warehouseIds[0] 会漂移，而库存兜底建仓在先——两者
    // 可能不一致导致盘点报"仓库 X 下无库存"。用库存行仓库保证数据自洽。
    const stockBefore = await verifyStockFourDim(page, productId, ctx.colorNos[0]);
    expect(
      stockBefore,
      `盘点前应存在产品 ${productId} / 色号 ${ctx.colorNos[0]} 的库存行`
    ).toBeTruthy();
    const warehouseId = Number(stockBefore!.warehouse_id);
    const stockId = Number(stockBefore!.id);
    const qtyBefore = Number(stockBefore!.quantity_available);

    // 后端 CreateCountPayload 真实字段
    const countData = {
      warehouse_id: warehouseId,
      count_date: new Date().toISOString(),
      notes: 'E2E 盘点测试',
    };

    const result = await apiCall<{ id?: number }>(page, 'POST', '/inventory/counts', countData);
    const countId = result.data?.id;
    expect(
      countId,
      `盘点建单应返回 data.id，实际响应：${JSON.stringify(result).slice(0, 200)}`
    ).toBeTruthy();

    // 验证初始状态：建单写 PENDING（inventory_count_service.rs:155；
    //  盘点词表只有 pending/completed，purchase_inventory.rs:82-88，无 draft）
    const created = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/inventory/counts/${countId}`
    );
    expect(created.status.toLowerCase()).toBe('pending');

    // 录入实盘数据（后端 RecordItemInput 真实字段：stock_id + quantity_actual 字符串）
    // stock_id 直接取盘点前命中的库存行（原实现查不到行时兜底成 1，会把盘点
    // 差异记到别的库存行上，断言与真实数据无关）
    await apiCall(page, 'POST', `/inventory/counts/${countId}/record`, {
      items: [
        {
          stock_id: stockId,
          quantity_actual: String(qtyBefore + 3),
          notes: 'E2E 盘点差异',
        },
      ],
    });

    // 提交审批：submit_count 写 "in_review"（inventory_count_service.rs:417），非 pending；
    // 该值未收录进 inventory_count 词表（pending/completed），登记为状态机一致性问题
    await apiCall(page, 'POST', `/inventory/counts/${countId}/submit`);
    const submitted = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/inventory/counts/${countId}`
    );
    expect(submitted.status.toLowerCase()).toBe('in_review');

    // 审批通过：approve_count 写 COMPLETED（inventory_count_service.rs:524），词表无 approved
    await apiCall(page, 'POST', `/inventory/counts/${countId}/approve`);
    const approved = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/inventory/counts/${countId}`
    );
    expect(approved.status.toLowerCase()).toBe('completed');

    // 验证审计日志：盘点审批端点 POST /inventory/counts/{id}/approve 被
    // omni_audit classify_operation 归类为 event_type="APPROVE"（路径末段含 approve → APPROVE，
    // omni_audit.rs:410-413；audit/search 以 module 列按 event_type 过滤，omni_audit_handler.rs:235-239），
    // 而非 UPDATE——审批是动作类端点，库存调整在 handler 内部完成、不单独产生 UPDATE 审计请求。
    const auditLogged = await verifyAuditLog(page, 'APPROVE', 'inventory');
    expect(auditLogged).toBe(true);

    // 验证库存已调整：审批后同一库存行的在库量/可用量都应等于录入的实盘数量
    // （后端 update_many 同时写 QuantityOnHand 与 QuantityAvailable）
    const stockAfter = await verifyStockFourDim(
      page,
      productId,
      ctx.colorNos[0],
      stockBefore!.dye_lot_no ? String(stockBefore!.dye_lot_no) : undefined,
      { warehouseId, batchNo: String(stockBefore!.batch_no) }
    );
    expect(stockAfter, '盘点调整后应仍命中同一库存行（仓库/批次/缸号维度）').toBeTruthy();
    expect(
      Number(stockAfter!.quantity_available),
      `可用量应调整为实盘量 ${qtyBefore + 3}（实际 ${stockAfter!.quantity_available}）`
    ).toBe(qtyBefore + 3);
    expect(
      Number(stockAfter!.quantity_on_hand),
      `在库量应调整为实盘量 ${qtyBefore + 3}（实际 ${stockAfter!.quantity_on_hand}）`
    ).toBe(qtyBefore + 3);
  });

  test('盘点拒绝：负数实盘数量应被拒', async ({ page }) => {
    const ctx = getCtx();

    // 仓库取实际库存行自带的 warehouse_id（避免 ctx.warehouseIds 漂移导致
    // "仓库 X 下无库存"，与第一个盘点测试同因）
    const stockRow = await verifyStockFourDim(page, ctx.productIds[0], ctx.colorNos[0]);
    expect(
      stockRow,
      `应存在产品 ${ctx.productIds[0]} / 色号 ${ctx.colorNos[0]} 的库存行（用例需真实仓库维度）`
    ).toBeTruthy();
    const countData = {
      warehouse_id: Number(stockRow!.warehouse_id),
      count_date: new Date().toISOString(),
    };

    const result = await apiCall<{ id?: number }>(page, 'POST', '/inventory/counts', countData);
    const countId = result.data?.id;
    expect(
      countId,
      `负例前置：盘点建单应返回 data.id，否则 record 请求打到 /undefined 会让拒绝断言假绿；实际响应：${JSON.stringify(result).slice(0, 200)}`
    ).toBeTruthy();

    // 负数实盘必须命中非负校验分支（record_count_items 的 is_sign_negative，
    // inventory_count_service.rs:400-405 → AppError::validation → HTTP 400 /
    // VALIDATION_ERROR）。但该校验位于 item_map.get(&stock_id) 之后（:395-397）：
    // 若 stock_id 不在本次盘点明细里会先返回 not_found（HTTP 404 / NOT_FOUND），
    // 根本走不到非负判断。建单用的是本行仓库（stockRow.warehouse_id），盘点的
    // 明细正是该仓库下的库存行，故 stock_id 必须取 stockRow.id（与状态机用例
    // :195 同法）才能命中真实修复分支；硬编码 id 会把 404 当成"拒绝"蒙过断言。
    const illegalRecord = await apiCallExpectFail(
      page,
      'POST',
      `/inventory/counts/${countId}/record`,
      {
        items: [
          {
            stock_id: Number(stockRow!.id),
            quantity_actual: '-100',
          },
        ],
      }
    );
    // 精确锚定非负校验：HTTP 400 + 机器码 VALIDATION_ERROR（error.rs:171/474），
    // 排除 404 NOT_FOUND 或其它码混过。status 用 toBe 精确匹配而非 >=400，
    // failureCode 用 toBe 精确命中 VALIDATION_ERROR（非负校验唯一出口）。
    const recordRejectCode = failureCode(illegalRecord);
    expect(
      illegalRecord.status,
      `负数实盘应命中非负校验返回 HTTP 400，实际 status=${illegalRecord.status}、` +
        `code=${recordRejectCode}、body=${JSON.stringify(illegalRecord).slice(0, 200)}`
    ).toBe(400);
    expect(
      recordRejectCode,
      `负数实盘应命中 VALIDATION_ERROR（非负校验分支），实际 code=${recordRejectCode}、` +
        `status=${illegalRecord.status}——若是 NOT_FOUND 说明 stock_id 未在盘点明细内、走错分支`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
  });

  test('盘点状态机：已审批不能再次提交', async ({ page }) => {
    const ctx = getCtx();

    // 仓库取实际库存行自带的 warehouse_id（避免 ctx.warehouseIds 漂移导致
    // "仓库 X 下无库存"，与第一个盘点测试同因）
    const stockRow = await verifyStockFourDim(page, ctx.productIds[0], ctx.colorNos[0]);
    expect(
      stockRow,
      `应存在产品 ${ctx.productIds[0]} / 色号 ${ctx.colorNos[0]} 的库存行（用例需真实仓库维度）`
    ).toBeTruthy();
    const countData = {
      warehouse_id: Number(stockRow!.warehouse_id),
      count_date: new Date().toISOString(),
    };

    const result = await apiCall<{ id?: number }>(page, 'POST', '/inventory/counts', countData);
    const countId = result.data?.id;
    expect(
      countId,
      `负例前置：盘点建单应返回 data.id，否则 submit/approve 打到 /undefined 会让状态机断言假绿；实际响应：${JSON.stringify(result).slice(0, 200)}`
    ).toBeTruthy();

    // 提交前必须录入至少一条实盘明细：后端 submit_count 校验"盘点单尚未录入任何实盘数量，
    // 无法提交审批"（见 reports/backend.log 该端点 400），否则首次 submit 即被拒、
    // 无法推进到 approved，也就到不了"已审批不能再次提交"这一被测状态。
    await apiCall(page, 'POST', `/inventory/counts/${countId}/record`, {
      items: [
        {
          stock_id: Number(stockRow!.id),
          quantity_actual: String(Number(stockRow!.quantity_available) + 2),
          notes: 'E2E 状态机前置实盘',
        },
      ],
    });

    await apiCall(page, 'POST', `/inventory/counts/${countId}/submit`);
    await apiCall(page, 'POST', `/inventory/counts/${countId}/approve`);

    // 负例必须用 apiCallExpectFail（apiCall/apiCallRaw 对非 2xx 直接抛，是成功语义）
    const illegalSubmit = await apiCallExpectFail(
      page,
      'POST',
      `/inventory/counts/${countId}/submit`
    );
    expect(illegalSubmit.status).toBeGreaterThanOrEqual(400);
    // 已审批(completed)不能重复提交：submit_count 抛 AppError::business（service:406-409），
    // 出参 code 稳定为 BUSINESS_ERROR（utils/error.rs:413；真实文案经 public_message 脱敏不进响应）
    expect(failureCode(illegalSubmit), '应命中业务拒绝码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
  });
});

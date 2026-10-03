import { test, expect } from '../diagnose-fixture';
import { pickListArray } from './ui-helpers';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  expectBusinessRejection,
  genCode,
  getCtx,
  verifyStockFourDim,
  verifyAuditLog,
  ensureTestEntities,
  seedFourDimStockIn,
  pickDyeableWarehouse,
  seedDyedOutboundBundle,
  readDyedPieceByNo,
  seedInspectionPass,
} from './helpers';

test.describe('库存调拨完整流程', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('调拨：创建→审批→出库→在途→入库→双仓库库存变化验证', async ({ page }) => {
    const ctx = getCtx();
    expect(ctx.warehouseIds.length).toBeGreaterThanOrEqual(2);

    const productId = ctx.productIds[0] || 1;

    // 调出仓库必须有库存：出库对染色布强制四维=缸号/色号/批次/匹号（用户 2026-10-02 口径），
    // 调拨同销售出库走 consume_dyed_piece_for_outbound（inv/batch.rs），且**建单期**即
    // validate_dyed_piece_for_outbound 预检真实可用匹（batch.rs:1193-1201）。
    // ensureStockInWarehouse 命中的历史行 batch≠缸号——写入方（piece_domain_service.rs:540）
    // 的染色匹恒 batch_no=dye_lot_no=缸号，按该 tuple 真实链配不出可命中匹，建单即被拒。
    // 故改用 helpers.seedDyedOutboundBundle：batch=缸号 四维库存行 + 委外染色真实链
    // 同维 AVAILABLE 匹（flow/07 先例链），调出仓取可承载染色匹的仓库（确定性，不靠 ctx 漂移）。
    const target = await pickDyeableWarehouse(page);
    const bundle = await seedDyedOutboundBundle(page, {
      productId,
      warehouseId: target.id,
      quantityMeters: '1000',
      pieceCount: 1,
      context: '12-T1',
    });
    const stockRow = bundle.stockRow;
    const colorNo = bundle.colorNo;
    const fromWarehouseId = target.id;
    const toWarehouseId =
      ctx.warehouseIds.find(id => id !== fromWarehouseId) || ctx.warehouseIds[1];
    if (!toWarehouseId || toWarehouseId === fromWarehouseId) {
      throw new Error(
        '[12-T1] 前置缺失：找不到与调出仓不同的调入仓（ctx.warehouseIds 不足），显式判红'
      );
    }

    // 调拨前：来源仓必须已有该产品+色号的库存行
    // （原实现读不存在的 quantity/available_qty 字段算出 qtyBefore，且该值后续从未参与断言）
    const stockBefore = await verifyStockFourDim(page, productId, colorNo, undefined, {
      warehouseId: fromWarehouseId,
    });
    expect(
      stockBefore,
      `调拨前来源仓 ${fromWarehouseId} 应存在产品 ${productId} / 色号 ${colorNo} 的库存行`
    ).toBeTruthy();

    // 出库按 产品+色号+缸号+批次 四维精确扣减、不回退（backend.log 明确："…批次… 在源仓库
    // 无任何库存记录，出库被拒绝（不回退到产品+色号扣减）"），染色布另强制第四维匹号。
    // 明细的缸号/批次/匹号必须取自本 bundle 的真实落库值（batch=缸号；匹号=真实链产出），
    // 不能硬编码——否则建单即被第四维预检拒，或 ship 时按不存在的 tuple 扣减必然被拒。
    const transferDyeLot = bundle.dyeLotNo;
    const transferBatchNo = String(stockRow.batch_no);
    if (transferBatchNo !== transferDyeLot) {
      throw new Error(
        `[12-T1] 库存行批次(${transferBatchNo})与缸号(${transferDyeLot})不一致，` +
          '与染色匹写入方口径（batch=缸号）矛盾，出库 tuple 必不命中——真实链断裂，显式判红'
      );
    }
    const transferPieceNo = bundle.pieces[0].piece_no;
    const transferQty = 5;

    // 后端 CreateInventoryTransferRequest 真实字段
    const transferData = {
      from_warehouse_id: fromWarehouseId,
      to_warehouse_id: toWarehouseId,
      transfer_date: new Date().toISOString(),
      notes: 'E2E 调拨测试',
      items: [
        {
          product_id: productId,
          quantity: String(transferQty),
          color_no: colorNo,
          dye_lot_no: transferDyeLot,
          batch_no: transferBatchNo,
          piece_no: transferPieceNo,
        },
      ],
    };

    const result = await apiCall<{ id?: number }>(
      page,
      'POST',
      '/inventory/transfers',
      transferData
    );
    const transferId = result.data?.id;
    expect(
      transferId,
      `调拨建单应返回 data.id，实际响应：${JSON.stringify(result).slice(0, 200)}`
    ).toBeTruthy();

    // 验证初始状态：build_transfer_active_model 默认写 PENDING
    // （inventory_move.rs:210-212；调拨词表只有 pending/approved/rejected/shipped/completed，
    //  无 draft——见 purchase_inventory.rs:63-78）
    const created = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/inventory/transfers/${transferId}`
    );
    expect(created.status.toLowerCase()).toBe('pending');

    // 审批调拨
    // ApproveTransferRequest { approved: bool, notes? } 必填
    await apiCall(page, 'POST', `/inventory/transfers/${transferId}/approve`, { approved: true });
    const approved = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/inventory/transfers/${transferId}`
    );
    expect(approved.status.toLowerCase()).toBe('approved');

    // 出库：update_transfer_to_shipped 写 "shipped"（batch.rs:457，词表无 in_transit）
    await apiCall(page, 'POST', `/inventory/transfers/${transferId}/ship`);
    const shipped = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/inventory/transfers/${transferId}`
    );
    expect(shipped.status.toLowerCase()).toBe('shipped');

    // 第四维消耗回读（写后必回读）：调拨出库对染色匹同事务 CAS AVAILABLE→SHIPPED
    const shippedPiece = await readDyedPieceByNo(page, {
      productId,
      warehouseId: fromWarehouseId,
      dyeLotNo: transferDyeLot,
      batchNo: transferBatchNo,
      pieceNo: transferPieceNo,
    });
    expect(shippedPiece, `出库后应能按四维 tuple 回读到匹 ${transferPieceNo}`).toBeTruthy();
    expect(
      String(shippedPiece!.status),
      `调拨出库应消耗该匹为 SHIPPED（词表 inventory_piece 大写），实际 ${shippedPiece!.status}`
    ).toBe('SHIPPED');

    // 验证非法操作：在途状态不能再次出库
    const illegalShip = await apiCallExpectFail(
      page,
      'POST',
      `/inventory/transfers/${transferId}/ship`
    );
    expect(illegalShip.status).toBeGreaterThanOrEqual(400);

    // 入库
    await apiCall(page, 'POST', `/inventory/transfers/${transferId}/receive`);
    const received = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/inventory/transfers/${transferId}`
    );
    expect(received.status.toLowerCase()).toBe('completed');

    // 调拨入库必须落到目标仓的四维库存行（产品+色号+缸号+批次+目标仓）
    const stockTo = await verifyStockFourDim(page, productId, colorNo, transferDyeLot, {
      batchNo: transferBatchNo,
      warehouseId: toWarehouseId,
    });
    expect(
      stockTo,
      `调拨完成后目标仓 ${toWarehouseId} 应存在缸号 ${transferDyeLot} / 批次 ${transferBatchNo} 的库存行`
    ).toBeTruthy();
    expect(
      Number(stockTo!.quantity_on_hand),
      `目标仓在库量应不少于调拨数量 ${transferQty}（实际 ${stockTo!.quantity_on_hand}）`
    ).toBeGreaterThanOrEqual(transferQty);

    // 验证审批动作端点(/inventory/transfers/:id/approve)落 omni_audit。
    // 事件类型以源码为唯一准：backend/src/middleware/omni_audit.rs::classify_operation
    // 优先级 0 规定"路径末段含 approve(或末段为 reject/submit)"→ event_type="APPROVE"，
    // 而非 HTTP 方法映射的 CREATE。落库映射见 omni_audit_service.rs:207(omni.module 列=event_type)、
    // :209(resource_type 列=infer_module_from_path 的业务段=inventory)、request_path=uri(含 approve)。
    // 故按 APPROVE + resource_type=inventory + request_path 含 approve 精确匹配，不放宽为恒真。
    const auditLogged = await verifyAuditLog(page, 'APPROVE', 'inventory', 'approve');
    expect(
      auditLogged,
      '审批动作端点(/inventory/transfers/:id/approve)应落 omni_audit：classify_operation 记为 APPROVE、resource_type=inventory'
    ).toBe(true);

    // UI 验证：访问调拨列表页
    await page.goto('/inventory-transfer');
    await page.waitForTimeout(2000);
    const tableVisible = await page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first()
      .isVisible();
    expect(tableVisible).toBe(true);
  });

  test('调拨状态机非法转换验证', async ({ page }) => {
    const ctx = getCtx();

    // 染色布建单为 fail-closed 校验：color_no 非白坯时必须同时提供缸号与批号，
    // 否则 create_transfer_items_and_compute_total 直接 400（inventory_move.rs:247-258）；
    // 出库对染色布强制四维=缸/色/批/匹（用户 2026-10-02 口径）——建单期还要按
    // validate_dyed_piece_for_outbound 预检真实 AVAILABLE 匹，故 seed 必须走
    // seedDyedOutboundBundle（batch=缸号 库存行 + 真实链同维匹），否则本负例连
    // "拿到 pending 调拨单"的前置都做不到。
    // 维度常量取自 bundle 真实落库值，确保 seed 与随后 POST 的调拨明细逐一对应。
    const target = await pickDyeableWarehouse(page);
    const bundle = await seedDyedOutboundBundle(page, {
      productId: ctx.productIds[0],
      warehouseId: target.id,
      quantityMeters: '1000',
      pieceCount: 1,
      context: '12-SM',
    });
    const colorNo = bundle.colorNo;
    const dyeLotNo = bundle.dyeLotNo;
    const batchNo = bundle.dyeLotNo; // 写入方口径：染色匹 批次=缸号
    const pieceNo = bundle.pieces[0].piece_no;

    // 根因C：本负例要先建出一张 pending 调拨单（否则 transferId=undefined，
    // /transfers/undefined/receive|ship 的非法转换断言会假绿）。而 inv/stock.rs::
    // check_from_warehouse_inventory 要求调出仓对该产品有维度匹配的足量库存，
    // 原实现未造源库存 → 建单被正确拒（无匹配库存）、拿不到 id → 本用例红。
    // 先按下方调拨明细的同一组维度（款号+色号+缸号+批次+匹号）在调出仓 seed 真实库存行+匹，
    // 再建单（不造假库存）。
    const transferData = {
      from_warehouse_id: target.id,
      to_warehouse_id: ctx.warehouseIds[1] || ctx.warehouseIds[0],
      transfer_date: new Date().toISOString(),
      items: [
        {
          product_id: ctx.productIds[0],
          quantity: '1',
          color_no: colorNo,
          dye_lot_no: dyeLotNo,
          batch_no: batchNo,
          piece_no: pieceNo,
        },
      ],
    };

    const result = await apiCall<{ id?: number }>(
      page,
      'POST',
      '/inventory/transfers',
      transferData
    );
    const transferId = result.data?.id;
    expect(
      transferId,
      `负例前置：调拨建单应返回 data.id，否则 URL 退化为 /undefined 会让非法转换断言假绿；实际响应：${JSON.stringify(result).slice(0, 200)}`
    ).toBeTruthy();

    // pending 状态直接入库应被拒
    const illegalReceive = await apiCallExpectFail(
      page,
      'POST',
      `/inventory/transfers/${transferId}/receive`
    );
    expect(illegalReceive.status).toBeGreaterThanOrEqual(400);

    // pending 状态直接出库应被拒
    const illegalShip = await apiCallExpectFail(
      page,
      'POST',
      `/inventory/transfers/${transferId}/ship`
    );
    expect(illegalShip.status).toBeGreaterThanOrEqual(400);
  });

  // ============================================================
  // 白坯布正常出库链路（真实入库→调拨出库）：
  // 业务口径：白坯布 = 没有颜色的布（color_no 为空），免缸号、批次必填。
  // 入库：走真实采购收货，明细不带 color_code / lot_no（→ inventory_stocks.color_no=''、
  //       dye_lot_no IS NULL），带批次；出库：调拨按 产品+仓库+批次+无色号 精确扣该白坯行。
  // 断言：出库后白坯库存行按批次精确减少（扣减量=出库量），出库明细色号为空、缸号为空。
  // ============================================================
  test('白坯布：真实入库后调拨出库按批次精确扣减（免缸号）', async ({ page }) => {
    const ctx = getCtx();
    expect(ctx.warehouseIds.length, '需要至少两个仓库（调出/调入）').toBeGreaterThanOrEqual(2);
    const productId = ctx.productIds[0];
    const supplierId = ctx.supplierId;
    const departmentId = ctx.departmentIds[0];
    const warehouseId = ctx.warehouseIds[0];
    expect(productId, '前置产品缺失').toBeTruthy();
    expect(supplierId, '前置供应商缺失').toBeTruthy();

    const whiteBatch = genCode('BN-WHITE');
    const inboundQty = 30;
    const outboundQty = 12;

    // ---- 1. 建采购单（白坯产品）并审批 ----
    const po = await apiCall<{ id?: number }>(page, 'POST', '/purchase/orders', {
      supplier_id: supplierId,
      warehouse_id: warehouseId,
      department_id: departmentId,
      order_date: new Date().toISOString().slice(0, 10),
      items: [{ material_id: productId, quantity_ordered: String(inboundQty), unit_price: '1' }],
    });
    const poId = po.data?.id;
    expect(poId, `采购单创建应返回 id，实际：${JSON.stringify(po).slice(0, 200)}`).toBeTruthy();
    await apiCall(page, 'POST', `/purchase/orders/${poId}/submit`);
    await apiCall(page, 'POST', `/purchase/orders/${poId}/approve`);
    console.warn(`[白坯出库] 采购单已审批 poId=${poId}`);

    // ---- 2. 建白坯入库单（不带 color_code / lot_no）并确认入库 ----
    // 后端 CreateReceiptItemRequest（purchase_receipt_dto.rs:54）material_code 为必填 String，
    // 缺失即 422（"items[0]: missing field `material_code`"）。取产品真实编码填入。
    const product = await apiCallRaw<{ code?: string }>(page, 'GET', `/products/${productId}`);
    const materialCode = String(product.code ?? '');
    expect(materialCode, `产品 ${productId} 应能读回 code 作为收货 material_code`).toBeTruthy();
    const receipt = await apiCall<{ id?: number }>(page, 'POST', '/purchase/receipts', {
      order_id: poId,
      supplier_id: supplierId,
      warehouse_id: warehouseId,
      receipt_date: new Date().toISOString().slice(0, 10),
      items: [
        {
          line_no: 1,
          material_id: productId,
          material_code: materialCode,
          material_name: 'E2E 白坯布',
          unit_master: 'm',
          quantity: String(inboundQty),
          quantity_alt: '0',
          batch_no: whiteBatch,
          // 白坯：故意不提供 color_code / lot_no
        },
      ],
    });
    const receiptId = receipt.data?.id;
    expect(receiptId, `白坯入库单创建失败：${JSON.stringify(receipt).slice(0, 200)}`).toBeTruthy();
    // 门控前置：白坯同样必经质检，整单 complete(pass) 回写 PASSED 后才允许确认入库
    await seedInspectionPass(page, {
      receiptId: receiptId as number,
      supplierId: supplierId as number,
      context: '12 白坯出库收货单',
    });
    await apiCall(page, 'POST', `/purchase/receipts/${receiptId}/confirm`);
    await expect
      .poll(
        async () => {
          const r = await apiCallRaw<{ receipt_status?: string; status?: string }>(
            page,
            'GET',
            `/purchase/receipts/${receiptId}`
          );
          return (r.receipt_status || r.status || '').toUpperCase();
        },
        { message: `白坯入库单 ${receiptId} 应进入 COMPLETED 终态` }
      )
      .toBe('COMPLETED');

    // ---- 3. 定位入库后的白坯库存行（color_no 空 + dye_lot_no 空 + 指定批次）----
    const readWhiteStock = async () => {
      const res = await apiCallRaw<{ items: Array<Record<string, unknown>> }>(
        page,
        'GET',
        `/inventory/stock?product_id=${productId}&warehouse_id=${warehouseId}&batch_no=${whiteBatch}&page=1&page_size=50`
      );
      // /stock -> PaginatedResponse（inventory_stock_handler::list_stock:250）
      const rows = pickListArray<Record<string, unknown>>(
        res,
        'items',
        '12 白坯库存 /inventory/stock'
      );
      console.warn(
        `[白坯出库] 批次 ${whiteBatch} 命中库存行 ${rows.length} 条: ${rows
          .map(
            r =>
              `id=${r.id} color=${JSON.stringify(r.color_no)} dye=${JSON.stringify(r.dye_lot_no)} avail=${r.quantity_available}`
          )
          .join(' | ')}`
      );
      return rows.find(r => !r.color_no && !r.dye_lot_no) ?? null;
    };
    const whiteBefore = await readWhiteStock();
    expect(
      whiteBefore,
      `白坯入库后应在仓库 ${warehouseId} 生成 color_no 空 + dye_lot_no 空 + 批次 ${whiteBatch} 的库存行`
    ).toBeTruthy();
    expect(
      Number(whiteBefore!.quantity_available),
      `白坯入库后可用量应为 ${inboundQty}`
    ).toBeCloseTo(inboundQty, 2);

    // ---- 4. 建白坯调拨单（色号空、无缸号、带批次）→ 审批 → 出库 ----
    const toWarehouseId = ctx.warehouseIds.find(id => id !== warehouseId) || ctx.warehouseIds[1];
    const transfer = await apiCall<{ id?: number }>(page, 'POST', '/inventory/transfers', {
      from_warehouse_id: warehouseId,
      to_warehouse_id: toWarehouseId,
      transfer_date: new Date().toISOString(),
      notes: 'E2E 白坯出库',
      items: [
        {
          product_id: productId,
          quantity: String(outboundQty),
          color_no: '',
          // 白坯免缸号：不提供 dye_lot_no
          batch_no: whiteBatch,
        },
      ],
    });
    const transferId = transfer.data?.id;
    expect(
      transferId,
      `白坯调拨建单应成功，实际：${JSON.stringify(transfer).slice(0, 200)}`
    ).toBeTruthy();
    console.warn(`[白坯出库] 白坯调拨建单成功 transferId=${transferId}`);

    // 出库明细色号为空、缸号为空（如实回显白坯维度）
    const detail = await apiCallRaw<{ items: Array<Record<string, unknown>> }>(
      page,
      'GET',
      `/inventory/transfers/${transferId}`
    );
    expect(detail.items.length, '白坯调拨明细应可读回').toBeGreaterThan(0);
    expect(String(detail.items[0].color_no ?? ''), '白坯出库明细色号应为空').toBe('');
    expect(
      detail.items[0].dye_lot_no === null ||
        detail.items[0].dye_lot_no === undefined ||
        detail.items[0].dye_lot_no === '',
      `白坯出库明细缸号应为空（实际 ${JSON.stringify(detail.items[0].dye_lot_no)}）`
    ).toBe(true);

    await apiCall(page, 'POST', `/inventory/transfers/${transferId}/approve`, { approved: true });
    await apiCall(page, 'POST', `/inventory/transfers/${transferId}/ship`);
    const shipped = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/inventory/transfers/${transferId}`
    );
    expect(shipped.status.toLowerCase(), '白坯调拨出库后状态应为 shipped').toBe('shipped');

    // ---- 5. 断言白坯库存行按批次精确减少（扣减量 = 出库量），未被跨缸回退动到其他行 ----
    const whiteAfter = await readWhiteStock();
    expect(whiteAfter, '出库后白坯库存行应仍存在（按批次定位）').toBeTruthy();
    expect(
      Number(whiteAfter!.quantity_available),
      `白坯库存行可用量应精确减少 ${outboundQty}（${inboundQty} → ${inboundQty - outboundQty}），实际 ${whiteAfter!.quantity_available}`
    ).toBeCloseTo(inboundQty - outboundQty, 2);
  });

  // ============================================================
  // 对照：染色布（色号非空）缺缸号建单必须被拒（4xx），证明白坯免缸号是"布种差异"而非
  // 放宽所有校验——与白坯出库测试同一判定源（fabric_class）双向锁死。
  // ============================================================
  // ============================================================
  // 预留链贯穿（routes/inventory.rs:186-206 reservation_routes）：
  // POST /reservations → /{id}/lock → /{id}/release，每步 GET 回读状态，
  // 词表权威 = models/status/purchase_inventory.rs::inventory_reservation（小写
  // pending/locked/released/consumed/cancelled），service 写入值逐字符一致。
  // 可用量守恒（实现真相）：create_reservation/lock/release 三步均只动
  // inventory_reservations 行（inventory_reservation_service.rs:25-127，无任何
  // inventory_stocks 更新）；库存可用量只在销售发货时扣减并把预留置 consumed
  // （so/delivery_ops/inventory.rs）。故本用例钉：预留链全程库存行的
  // quantity_available / quantity_reserved 不变，且 inventory_reservations.quantity
  // 恒等于建单预留量 —— 等式：avail(前) == avail(后)，reservation.quantity == 20。
  // 前置说明：inventory_reservations.order_id 有 FK → sales_orders.id
  // （migration m0010_add_inventory_extensions.rs:63），必须引用真实销售订单，
  // 不造假 ID（FK 裸违例会被 handler map_err 成 500，掩盖真实契约）。
  // ============================================================
  test('预留链贯穿：pending→locked→released 每步回读 + 库存可用量守恒', async ({ page }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    const warehouseId = ctx.warehouseIds[0];
    expect(productId, '前置产品缺失').toBeTruthy();
    expect(warehouseId, '前置仓库缺失').toBeTruthy();

    // 专属四维库存行（预留量 20 米 < 库存 100 米；即便实现占用也有余量）
    const tag = Date.now().toString().slice(-6);
    const colorNo = `E2E-RSV-C${tag}`;
    const dyeLotNo = `E2E-RSV-D${tag}`;
    const batchNo = `E2E-RSV-B${tag}`;
    await seedFourDimStockIn(page, {
      productId: productId!,
      warehouseId: warehouseId!,
      colorNo,
      dyeLotNo,
      batchNo,
      quantityMeters: '100',
    });
    const readStock = async () =>
      verifyStockFourDim(page, productId!, colorNo, dyeLotNo, {
        batchNo,
        warehouseId: warehouseId!,
      });
    const stock0 = await readStock();
    expect(stock0, 'seed 后应存在四维库存行').toBeTruthy();
    const avail0 = Number(stock0!.quantity_available);
    const reservedCol0 = Number(stock0!.quantity_reserved);
    expect(avail0, 'seed 库存行可用量应为 100').toBeCloseTo(100, 2);

    // FK 前置：真实销售订单（仅建单，预留链不要求审批态）
    const so = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
      customer_id: ctx.customerId,
      order_date: new Date().toISOString(),
      items: [{ product_id: productId, quantity: 5, unit_price: '20.00' }],
    });
    const soId = so.data?.id;
    expect(
      soId,
      `销售订单建单应返回 id（预留 order_id FK），实际：${JSON.stringify(so).slice(0, 200)}`
    ).toBeTruthy();

    // ---- 1. 创建预留：期望 pending；回读走 GET list（非响应回声）----
    const reservationQty = 20;
    const created = await apiCall<{ id?: number; status?: string; quantity?: string }>(
      page,
      'POST',
      '/inventory/reservations',
      {
        product_id: productId,
        warehouse_id: warehouseId,
        quantity: String(reservationQty),
        order_id: soId,
        notes: `E2E 预留链贯穿 tag=${tag}`,
      }
    );
    const reservationId = created.data?.id;
    expect(
      reservationId,
      `创建预留应返回 data.id，实际：${JSON.stringify(created).slice(0, 200)}`
    ).toBeTruthy();
    expect(String(created.data?.status), '创建预留响应状态应为 pending').toBe('pending');

    // 写后必回读：GET /reservations 列表（data_scope=all 的默认账号可见本人行）
    const listAfterCreate = await apiCallRaw<{ list: Array<Record<string, unknown>> }>(
      page,
      'GET',
      `/inventory/reservations?product_id=${productId}&warehouse_id=${warehouseId}&page=1&page_size=100`
    );
    const rowCreate = pickListArray<Record<string, unknown>>(
      listAfterCreate,
      'list',
      '12 预留列表 /inventory/reservations'
    ).find(r => Number(r.id) === reservationId);
    expect(rowCreate, `回读：预留 ${reservationId} 应出现在 GET /reservations 列表`).toBeTruthy();
    expect(String(rowCreate!.status), '回读：初始落库状态应为 pending').toBe('pending');
    expect(Number(rowCreate!.quantity), `回读：预留行数量应恒为 ${reservationQty}`).toBeCloseTo(
      reservationQty,
      2
    );

    // 守恒：pending 阶段库存行 quantity_available / quantity_reserved 不变
    const stockCreate = await readStock();
    expect(
      Number(stockCreate!.quantity_available),
      `预留创建不应改可用量（实现：create_reservation 不触 inventory_stocks），期望 ${avail0}，实际 ${stockCreate!.quantity_available}`
    ).toBeCloseTo(avail0, 2);
    expect(
      Number(stockCreate!.quantity_reserved),
      '预留创建不应改库存行 quantity_reserved 列'
    ).toBeCloseTo(reservedCol0, 2);

    // ---- 2. 锁定：pending→locked（非法态负例 + 非法写后状态不变回读）----
    await apiCall(page, 'POST', `/inventory/reservations/${reservationId}/lock`);
    const listAfterLock = await apiCallRaw<{ list: Array<Record<string, unknown>> }>(
      page,
      'GET',
      `/inventory/reservations?product_id=${productId}&warehouse_id=${warehouseId}&page=1&page_size=100`
    );
    const rowLock = pickListArray<Record<string, unknown>>(
      listAfterLock,
      'list',
      '12 预留列表 /inventory/reservations（锁定后）'
    ).find(r => Number(r.id) === reservationId);
    expect(
      String(rowLock!.status),
      `回读：锁定后状态应为 locked（词表 purchase_inventory.rs::inventory_reservation::LOCKED），实际 ${rowLock!.status}`
    ).toBe('locked');

    // 非法转换：locked 再 lock 应被状态门拒绝（service.rs:68-73 仅 pending 可锁定）
    const illegalRelock = await apiCallExpectFail(
      page,
      'POST',
      `/inventory/reservations/${reservationId}/lock`
    );
    expectBusinessRejection(illegalRelock, 'locked 状态重复锁定应被业务拒绝');
    // 非法写被拒后回读：状态必须仍是 locked（失败写不得改变状态）
    const listAfterIllegal = await apiCallRaw<{ list: Array<Record<string, unknown>> }>(
      page,
      'GET',
      `/inventory/reservations?product_id=${productId}&warehouse_id=${warehouseId}&page=1&page_size=100`
    );
    const rowAfterIllegal = pickListArray<Record<string, unknown>>(
      listAfterIllegal,
      'list',
      '12 预留列表 /inventory/reservations（非法重复锁定后）'
    ).find(r => Number(r.id) === reservationId);
    expect(
      String(rowAfterIllegal!.status),
      '非法重复锁定被拒后，预留状态应仍为 locked（失败写不改状态）'
    ).toBe('locked');
    // 守恒：lock 阶段库存可用量仍不变
    const stockLock = await readStock();
    expect(
      Number(stockLock!.quantity_available),
      `锁定不应改可用量（期望 ${avail0}，实际 ${stockLock!.quantity_available}）`
    ).toBeCloseTo(avail0, 2);

    // ---- 3. 释放：locked→released ----
    await apiCall(page, 'POST', `/inventory/reservations/${reservationId}/release`);
    const listAfterRelease = await apiCallRaw<{ list: Array<Record<string, unknown>> }>(
      page,
      'GET',
      `/inventory/reservations?product_id=${productId}&warehouse_id=${warehouseId}&page=1&page_size=100`
    );
    const rowRelease = pickListArray<Record<string, unknown>>(
      listAfterRelease,
      'list',
      '12 预留列表 /inventory/reservations（释放后）'
    ).find(r => Number(r.id) === reservationId);
    expect(
      String(rowRelease!.status),
      `回读：释放后状态应为 released，实际 ${rowRelease!.status}`
    ).toBe('released');
    expect(rowRelease!.released_at, '回读：释放应落 released_at（service.rs:117）').toBeTruthy();

    // 非法转换：released 不可再 lock（仅 pending 可锁定）
    const illegalLockAfterRelease = await apiCallExpectFail(
      page,
      'POST',
      `/inventory/reservations/${reservationId}/lock`
    );
    expectBusinessRejection(illegalLockAfterRelease, 'released 状态再锁定应被业务拒绝');

    // ---- 4. 终局守恒：全链结束后库存行数量与初始完全一致 ----
    const stockFinal = await readStock();
    expect(
      Number(stockFinal!.quantity_available),
      `预留链（pending→locked→released）全程可用量守恒：期望 ${avail0}，实际 ${stockFinal!.quantity_available}`
    ).toBeCloseTo(avail0, 2);
    expect(
      Number(stockFinal!.quantity_reserved),
      '预留链全程库存行 quantity_reserved 守恒'
    ).toBeCloseTo(reservedCol0, 2);
    // 预留行数量守恒（quantity 不随状态迁移变化）
    expect(
      Number(rowRelease!.quantity),
      `回读：释放后预留行数量仍应为 ${reservationQty}`
    ).toBeCloseTo(reservationQty, 2);
  });

  test('染色布缺缸号：调拨建单仍被拒（白坯免缸号的对照）', async ({ page }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    const rejected = await apiCallExpectFail(page, 'POST', '/inventory/transfers', {
      from_warehouse_id: ctx.warehouseIds[0],
      to_warehouse_id: ctx.warehouseIds[1] || ctx.warehouseIds[0],
      transfer_date: new Date().toISOString(),
      items: [
        {
          product_id: productId,
          quantity: '1',
          color_no: ctx.colorNos[0], // 非空色号 = 染色布
          // 故意不提供 dye_lot_no
          batch_no: genCode('BN-DYED-NEG'),
        },
      ],
    });
    console.warn(
      `[染色对照] 染色布缺缸号建单 status=${rejected.status} message=${rejected.message}`
    );
    expectBusinessRejection(
      rejected,
      '染色布缺缸号建单应被业务校验拒绝（400 + 业务码 + 非空 message）'
    );
  });
});

// 库存管理 E2E 套件 — 05 拆匹（布卷剪裁拆分）
// 覆盖路由：routes/inventory.rs:37-43 piece_split_routes（POST /inventory/piece-split）
//   + GET /inventory/pieces（inventory_piece_handler::list_pieces，回读通道）
// 母匹 seed 复用 flow/07 既有生产报工逐匹范式（真实链路：生产订单→流转卡→备布→
// 工序启动→报工逐匹），不造假匹号、不绕过状态机。
// 守恒等式（业务契约，piece_split_handler.rs:145-165 validate_split_consistency 的意图）：
//   拆分后 母匹剩余长度 + 子匹长度 == 拆分前母匹长度（一次拆分场景），重量同理。
//
// ⚠️ 预红声明（源码缺陷钉，判责在后端，禁止用放宽断言蒙过）：
//   piece_split_handler.rs:50-52 首次拆分时
//     original_length = parent.original_length.unwrap_or(parent.length + req.cut_length)
//   而母匹由生产报工创建时 original_length=NULL（piece_domain_service.rs:131
//   original_length: Set(None)），于是 virgin 母卷的 original_length 被算成
//   "现长 + 本次剪裁"（100+30=130）；随后的 validate_split_consistency 按
//   "剩余(70) + 子卷(30) == original(130)" 判定 → 恒 100≠130 → 事务回滚、必 400
//   "拆匹一致性校验失败"。即当前实现下任何 virgin 母匹的拆匹成功路径不可达。
//   正确公式应为 unwrap_or(parent.length)（原始长度=拆分前现长）。
//   本用例按**契约意图**（成功拆分+守恒）断言，预期在源码修复前为红——
//   红即暴露真实缺陷；不得改为断言"拆匹必 400"来钉死缺陷行为当绿灯。
import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  expectBusinessRejection,
  ensureTestEntities,
  getCtx,
  genCode,
  genName,
} from '../flow/helpers';
import { pickListArray } from '../flow/ui-helpers';

interface PieceRow {
  id: number;
  piece_no: string;
  length: number | string;
  weight: number | string | null;
  status: string;
  parent_piece_id: number | null;
  color_no: string | null;
  dye_lot_no: string | null;
}

const dec = (v: unknown): number => Number(String(v));

async function fetchPiecesByNo(page: Page, pieceNo: string): Promise<PieceRow[]> {
  const res = await apiCallRaw<unknown>(
    page,
    'GET',
    `/inventory/pieces?piece_no=${encodeURIComponent(pieceNo)}&page=1&page_size=20`
  );
  // /pieces -> PaginatedResponse<PieceResponse>（inventory_piece_handler.rs:141-143），items 键
  return pickListArray<PieceRow>(res, 'items', '5 拆匹回读 /inventory/pieces');
}

// 复用 flow/07 报工链 seed 一匹 virgin 生产匹（length=100、weight=50，入未分类/胚布仓）
async function seedParentPiece(page: Page): Promise<{ pieceNo: string; parentId: number }> {
  await ensureTestEntities(page);
  const ctx = getCtx();
  const productId = ctx.productIds[0];
  expect(productId, '前置：拆匹需至少一个产品').toBeTruthy();

  // 仓库选择口径：piece_domain_service::validate_warehouse_for_piece_type 对
  // greige 匹仅拒成品仓（finished），NULL/其他类型不校验（:38-48）——取首个非成品仓。
  const wh = await apiCallRaw<{ items: Array<{ id: number; warehouse_type: string | null }> }>(
    page,
    'GET',
    '/warehouses?page=1&page_size=200'
  );
  const rows = pickListArray<{ id: number; warehouse_type: string | null }>(
    wh,
    'items',
    '5 拆匹 seed 仓库列表 /warehouses'
  );
  const warehouse = rows.find(w => w.warehouse_type !== 'finished');
  expect(warehouse?.id, '前置：需至少一个非成品仓（胚布仓或未分类仓）').toBeTruthy();

  const orderNo = genCode('PO');
  const po = await apiCall<{ id?: number; order_no?: string }>(
    page,
    'POST',
    '/production/production-orders/orders',
    { order_no: orderNo, product_id: productId, planned_quantity: 100 }
  );
  expect(
    po.data?.id,
    `前置：生产订单创建应返回 id，实际：${JSON.stringify(po).slice(0, 200)}`
  ).toBeTruthy();

  const card = await apiCall<{ id?: number }>(page, 'POST', '/production/flow-cards', {
    production_order_id: po.data!.id,
    product_id: productId,
    product_name: genName('E2E拆匹胚布'),
    planned_fabric_weight: 100,
  });
  const cardId = card.data?.id;
  expect(
    cardId,
    `前置：流转卡创建应返回 id，实际：${JSON.stringify(card).slice(0, 200)}`
  ).toBeTruthy();
  await apiCall(page, 'POST', `/production/flow-cards/${cardId}/schedule`, {});
  await apiCall(page, 'POST', `/production/flow-cards/${cardId}/start-preparing`);
  await apiCall(page, 'POST', `/production/flow-cards/${cardId}/complete-preparing`, {
    actual_fabric_weight: 100,
  });

  const step = await apiCall<{ id?: number }>(page, 'POST', '/production/flow-cards/steps/start', {
    flow_card_id: cardId,
  });
  const stepId = step.data?.id;
  expect(
    stepId,
    `前置：工序启动应返回 id，实际：${JSON.stringify(step).slice(0, 200)}`
  ).toBeTruthy();

  const pieceNo = `GR-${genCode('SPLIT')}-001`;
  await apiCall(page, 'POST', `/production/flow-cards/steps/${stepId}/complete`, {
    actual_quantity: 100,
    qualified_quantity: 100,
    pieces: [
      {
        piece_no: pieceNo,
        machine_no: 'M-E2E-SPLIT',
        machine_operator: 'E2E开机人',
        length: 100,
        weight: 50,
        warehouse_id: warehouse!.id,
      },
    ],
  });
  console.warn(`[拆匹seed] 报工逐匹完成 pieceNo=${pieceNo} warehouseId=${warehouse!.id}`);

  // seed 后先回读母匹（写后必回读）：存在、长度 100、状态 AVAILABLE、尚无父
  const parentRows = await fetchPiecesByNo(page, pieceNo);
  const parent = parentRows.find(p => p.piece_no === pieceNo);
  expect(parent, `回读：报工后应能经 GET /inventory/pieces 命中母匹 ${pieceNo}`).toBeTruthy();
  expect(dec(parent!.length), '母匹初始长度应为 100').toBeCloseTo(100, 2);
  expect(
    String(parent!.status),
    `母匹初始状态应为 AVAILABLE（词表 purchase_inventory.rs::inventory_piece，大写），实际 ${parent!.status}`
  ).toBe('AVAILABLE');
  expect(parent!.parent_piece_id, '母匹（ virgin）不应有父匹关联').toBeFalsy();
  return { pieceNo, parentId: parent!.id };
}

test.describe('库存管理 - 05 拆匹（piece-split）', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('拆匹：POST /piece-split → 母匹减、子匹增、总量守恒（回读钉）', async ({ page }) => {
    const { pieceNo: parentNo, parentId } = await seedParentPiece(page);
    const parentBefore = await fetchPiecesByNo(page, parentNo);
    const parentRowBefore = parentBefore.find(p => p.piece_no === parentNo)!;
    const lengthBefore = dec(parentRowBefore.length);
    const weightBefore = dec(parentRowBefore.weight);

    const childNo = genCode('SPLIT-CHILD');
    const cut = { length: 30, weight: 10 };
    const split = await apiCall<{
      parent_piece?: { id: number; length: number | string };
      new_piece?: { id: number; piece_no: string; length: number | string };
    }>(page, 'POST', '/inventory/piece-split', {
      parent_piece_id: parentId,
      cut_length: cut.length,
      cut_weight: cut.weight,
      new_barcode: childNo,
    });
    // 若后端拒绝（当前源码状态门的真实回显必须进错误信息，便于定位）apiCall 会抛，
    // 到达此处即 2xx；响应结构仍要钉：
    expect(
      split.data?.new_piece?.id,
      `拆匹响应应回显新子匹（SplitPieceResponse.new_piece），实际：${JSON.stringify(split).slice(0, 200)}`
    ).toBeTruthy();

    // ---- 写后必回读（GET /pieces，不依赖响应回声）----
    const parentAfter = await fetchPiecesByNo(page, parentNo);
    const parentRow = parentAfter.find(p => p.piece_no === parentNo)!;
    expect(parentRow, `回读：母匹 ${parentNo} 拆分后仍存在`).toBeTruthy();
    expect(
      dec(parentRow.length),
      `母匹长度应减 ${cut.length}（${lengthBefore} → ${lengthBefore - cut.length}），实际 ${parentRow.length}`
    ).toBeCloseTo(lengthBefore - cut.length, 2);
    expect(
      dec(parentRow.weight),
      `母匹重量应减 ${cut.weight}（${weightBefore} → ${weightBefore - cut.weight}），实际 ${parentRow.weight}`
    ).toBeCloseTo(weightBefore - cut.weight, 2);

    const childAfter = await fetchPiecesByNo(page, childNo);
    const childRow = childAfter.find(p => p.piece_no === childNo);
    expect(
      childRow,
      `回读：子匹应存在且匹号取 new_barcode（generate_piece_no :169-179），childNo=${childNo}`
    ).toBeTruthy();
    expect(dec(childRow!.length), '子匹长度应等于剪裁量').toBeCloseTo(cut.length, 2);
    expect(dec(childRow!.weight), '子匹重量应等于剪裁重量').toBeCloseTo(cut.weight, 2);
    expect(childRow!.parent_piece_id, '子匹应挂 parent_piece_id 指向母匹').toBe(parentId);
    expect(String(childRow!.status), '子匹初始状态应为 AVAILABLE（build_new_piece :208）').toBe(
      'AVAILABLE'
    );

    // ---- 守恒等式：母匹剩余 + 子匹 == 拆分前母匹总量 ----
    expect(
      dec(parentRow.length) + dec(childRow!.length),
      `长度守恒：母匹剩余(${parentRow.length}) + 子匹(${childRow!.length}) 应 == 拆分前 ${lengthBefore}`
    ).toBeCloseTo(lengthBefore, 2);
    expect(
      dec(parentRow.weight) + dec(childRow!.weight),
      `重量守恒：母匹剩余(${parentRow.weight}) + 子匹(${childRow!.weight}) 应 == 拆分前 ${weightBefore}`
    ).toBeCloseTo(weightBefore, 2);
  });

  test('拆匹负例：剪裁长度超母匹现长必 4xx，且被拒后母匹长度不变', async ({ page }) => {
    const { pieceNo, parentId } = await seedParentPiece(page);
    const parentBefore = await fetchPiecesByNo(page, pieceNo);
    const rowBefore = parentBefore.find(p => p.piece_no === pieceNo)!;

    // validate_parent_piece（piece_split_handler.rs:104-109）：parent.length < cut → 400
    const rejected = await apiCallExpectFail(page, 'POST', '/inventory/piece-split', {
      parent_piece_id: parentId,
      cut_length: 999,
    });
    console.warn(
      `[拆匹负例] 超长剪裁：status=${rejected.status} code=${rejected.code} message=${rejected.message}`
    );
    expectBusinessRejection(
      rejected,
      `剪裁长度超过母匹现长应被业务拒绝（400 + 机器码 + 拒绝原因），实际 message=${rejected.message}`
    );

    // 失败写后回读：母匹长度不得变化、母匹行不得被复制/新增关联行
    const afterIllegal = await fetchPiecesByNo(page, pieceNo);
    const rowsByNo = afterIllegal.filter(p => p.piece_no === pieceNo);
    expect(rowsByNo.length, `非法剪裁被拒后母匹 ${pieceNo} 应仍唯一`).toBe(1);
    const rowAfter = rowsByNo[0];
    expect(
      dec(rowAfter.length),
      `非法剪裁被拒后母匹长度应仍为 ${rowBefore.length}，实际 ${rowAfter.length}`
    ).toBeCloseTo(dec(rowBefore.length), 2);
    // 说明：/pieces 无 parent_piece_id 过滤参数，无法廉价枚举"本次失败请求是否留下子匹行"；
    // handler 在 validate_parent_piece 之后才 insert 子匹（piece_split_handler.rs:42-80），
    // 400 发生于任何写入之前且整个事务随 Err 回滚，失败请求不产生子匹属结构保证。
  });

  test('拆匹负例：母匹不存在应 404（不静默、不裸 500）', async ({ page }) => {
    const ghost = 999_999_999;
    const fail = await apiCallExpectFail(page, 'POST', '/inventory/piece-split', {
      parent_piece_id: ghost,
      cut_length: 10,
    });
    console.warn(
      `[拆匹404] parent=${ghost}：status=${fail.status} code=${fail.code} message=${fail.message}`
    );
    expect(fail.status, `不存在母匹拆匹应 404（handler :45 not_found），实际 ${fail.status}`).toBe(
      404
    );
  });
});

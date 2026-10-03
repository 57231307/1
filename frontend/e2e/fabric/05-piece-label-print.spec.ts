// 面料管理 E2E 套件 — 05 成品布入库标签打印（#220，BE-220/FE-220 交付的 CI 内容级验证）
// 被测契约（先读交付报告 C:/Users/57231/wave220-backend.md 与 wave220-frontend.md，本文件逐条对齐）：
// - 端点 GET /inventory/pieces/{id}/print（backend/src/routes/inventory.rs piece_routes），
//   doc_type inventory_piece_label，成功=docx 二进制，失败=全站唯一 AppError 信封；
// - 8 必填字段全取 inventory_piece 自身行实测值（缸号/色号/批次/匹号/米数/重量/幅宽/克重）
//   + 条码 + 等级，款号/品名 LeftJoin products（print_service.rs::get_inventory_piece_label_print_data）；
// - 仅 piece_type='dyed' 可打；样布(SAMPLE)/生产匹 → BUSINESS 族；缺任一必填列 → 400
//   VALIDATION_ERROR 且 message 逐列点名（≠脱敏常量「请求参数验证失败」）；
// - 保密口径（裁定 §6）：supplier_piece_no/供应商侧编码/成本列不得进入标签正文；
// - 打卷三实测值（roll_weight/roll_width/roll_gram_weight）已改必填（RollFabricRequest
//   validator required），本套件按真实契约传实测值。
//
// 用例组织（serial 共享一条真实前置链，避免每条用例重跑委外链）：
// 1. 真实链造一匹字段齐全的染色匹：委外染色收回建档缸号（helpers.seedDyedPieceChain）→
//    验布创建→开始→评级→打卷（三实测值必填）→ 回读 GET /inventory/pieces 逐列核对；
// 2. 标签 docx 逐值断言：JSZip 解 word/document.xml，种子值精确匹配（范式照 traversal/37b）；
// 3. 保密反证：哨兵值注入可行性已核实为「真造不出」（详见用例内证据链注释），不造假不 skip，
//    落键名/语义级零出现反证；
// 4. 缺列 fail-closed：委外染整收回生成的真实 dyed 匹（width/gram_weight/weight 三列 NULL，
//    写入点 backend/src/services/piece_domain_service.rs:560-562，后端报告 §⑤ 声明的现状）
//    → 400 VALIDATION_ERROR 且点名缺的列；
// 5. 门控：生产匹（greige，同链报工产出）→ 只断 status/code=BUSINESS_ERROR 族（脱敏出参不断原因）。
//
// 全部真实后端 + 真实 PostgreSQL（IR 2026-09-07），每步显式日志；无 skip 掩盖、无放宽断言。
import { test, expect } from '../diagnose-fixture';
import JSZip from 'jszip';
import type { Page } from '@playwright/test';
import { pickListArray } from '../flow/ui-helpers';
import {
  loginViaUI,
  ensureTestEntities,
  getCtx,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  genCode,
  genDyeLotNo,
  pickDyeableWarehouse,
  seedDyedPieceChain,
  APP_ERROR_CODES,
  API_BASE,
  API_PREFIX,
  type DyedSeedPiece,
} from '../flow/helpers';

/**
 * serial 共享的前置链现场。test 1 真实链断裂时先行判红，后续用例依赖同一现场
 * （Playwright serial 语义：前置红则本组后续不执行，属显式暴露而非 skip 掩盖；
 *  每条用例仍自带 requireReady 防御——被单跑/重排时缺前置立即抛中文原因判红）。
 */
const S: {
  ready: boolean;
  productId?: number;
  warehouseId?: number;
  dyeLotNo?: string;
  colorNo?: string;
  /** 打卷产出的字段齐全 dyed 匹（标签内容断言对象） */
  rolled?: {
    id: number;
    pieceNo: string;
    /** 打卷提交的实测值（种子即期望值，docx 逐值断言的基准） */
    length: string;
    weight: string;
    width: string;
    gramWeight: string;
  };
  /** 委外染整收回匹：dyed 且 重量/幅宽/克重 三列 NULL（fail-closed 用例对象） */
  recovered?: DyedSeedPiece;
  /** 同链报工产出的生产匹（piece_type=greige，门控用例对象） */
  greigePiece?: { id: number; pieceNo: string };
  /** 用例 2 解出的标签正文，供用例 3 复用（同匹同文档，二次拉取也允许） */
  docXml?: string;
} = { ready: false };

// 打卷实测值种子：刻意取互不相同、带小数位的值，docx 内按 normalize 后原样出现
// （print_service.rs fmt_dec = Decimal::normalize().to_string()，182.50→"182.5"），
// 令逐值断言可精确钉死且彼此不误命中。
const ROLL_LENGTH_M = 50.25;
const ROLL_WEIGHT_KG = 21.75;
const ROLL_WIDTH_CM = 182.5;
const ROLL_GRAM = 205.4;

function requireReady(what: string): void {
  if (!S.ready) {
    throw new Error(
      `[05-label] 前置真实链未就绪（用例 1 委外建档→验布打卷回读未通过即红），` +
        `「${what}」缺可断言数据——共享现场缺失属链断，显式判红，绝不 skip 掩盖`
    );
  }
}

/**
 * Decimal 序列化归一（仅去尾零这一格式差，不改值）："50.2500"→"50.25"、50.25→"50.25"，
 * 与后端标签装配 fmt_dec = Decimal::normalize().to_string()（print_service.rs:5030）同形，
 * 保证正文逐值断言的基准串就是打卷提交的种子数值本身。
 */
function normDec(v: unknown): string {
  const n = Number(String(v));
  if (!Number.isFinite(n)) {
    throw new Error(`[05-label] Decimal 值不可解析（实际 ${JSON.stringify(v)}），格式异常判红`);
  }
  return String(n);
}

/**
 * 下载型健康断言 + JSZip 解包（二进制端点禁走 JSON 助手，否则假红；范式照 traversal/37b）。
 * 成功出参必须是 docx 容器（PK magic + word/document.xml 非空）；失败出参由调用方先断 2xx。
 */
async function fetchLabelDocXml(page: Page, pieceId: number): Promise<string> {
  const resp = await page.request.get(`${API_BASE}${API_PREFIX}/inventory/pieces/${pieceId}/print`);
  const status = resp.status();
  console.log(`[05-label] GET /inventory/pieces/${pieceId}/print → ${status}`);
  expect(
    status,
    `有效染色匹 ${pieceId} 标签打印应 200，实际 ${status}（失败体见用例 4/5 专项）`
  ).toBe(200);
  const body = await resp.body();
  expect(
    body.length,
    `标签 docx 应为非空二进制容器（>1KB），实际 ${body.length}B——JSON 助手解二进制会假红，本函数按下载断言`
  ).toBeGreaterThan(1024);
  expect(body[0], 'docx 应为 zip 容器（PK magic [0]）').toBe(0x50);
  expect(body[1], 'docx 应为 zip 容器（PK magic [1]）').toBe(0x4b);
  const zip = await JSZip.loadAsync(body);
  const docXml = await zip.file('word/document.xml')?.async('string');
  expect(
    docXml,
    `标签 docx 应包含 word/document.xml（实际 zip 条目：${Object.keys(zip.files).join(',')}）`
  ).toBeTruthy();
  expect(docXml!.length, `document.xml 应非空，实际 ${docXml!.length}B`).toBeGreaterThan(100);
  return docXml!;
}

/**
 * GET /inventory/pieces 出参行（PieceResponse，inventory_piece_handler.rs:45-67）中
 * 本套件用到的列。注意该响应不含 width/gram_weight/barcode（已列入报告疑点）——
 * 落库回读仅能核到 weight/length 维，幅宽/克重由用例 2 标签正文逐值断言钉。
 * length/weight 为 Decimal（number 或尾零 string 两种序列化形态均如实归一比较）。
 */
interface PieceRow {
  id: number;
  piece_no: string;
  piece_type: string;
  status: string;
  color_no: string | null;
  dye_lot_no: string | null;
  batch_no: string;
  length: number | string;
  weight: number | string | null;
}

/** 按缸号回读该缸全部 dyed 匹（列表键 items、分页信封 PaginatedResponse——缺 items 判红不兜底） */
async function listDyedPiecesByLot(page: Page, dyeLotNo: string): Promise<PieceRow[]> {
  const res = await apiCallRaw<unknown>(
    page,
    'GET',
    `/inventory/pieces?dye_lot_no=${encodeURIComponent(dyeLotNo)}&piece_type=dyed&page=1&page_size=50`
  );
  return pickListArray<PieceRow>(res, 'items', `${dyeLotNo} 缸染色匹回读`);
}

test.describe('05 成品布入库标签打印（#220 内容级）', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('1. 真实链造字段齐全染色匹：委外建档→验布打卷（三实测值必填）→逐列回读', async ({
    page,
  }) => {
    test.setTimeout(420_000);
    await ensureTestEntities(page);
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    if (!productId) {
      throw new Error(
        '[05-label] 前置缺失：ctx.productIds 为空（ensureTestEntities 未产出产品），显式判红'
      );
    }
    const warehouse = await pickDyeableWarehouse(page);
    const dyeLotNo = genDyeLotNo();
    const colorNo = `E2EC-${genCode('LBL')}`;

    // —— 前置链 A：委外染色收回（缸号在 batch_dye_lot 建档的唯一真实 API 入口，
    //    piece_domain_service.rs:484-513 find_or_create；打卷 fetch_dye_lot 强依赖该档案）——
    //    顺带产出：三列 NULL 的 dyed 匹（用例 4 对象）与报工生产匹（用例 5 对象）。
    const recoveredPieces = await seedDyedPieceChain(page, {
      productId,
      warehouseId: warehouse.id,
      colorNo,
      dyeLotNo,
      pieceCount: 1,
      lengthPerPieceMeters: 100,
      context: 'LBL05',
    });
    const recovered = recoveredPieces[0];
    if (!recovered) {
      throw new Error('[05-label] seedDyedPieceChain 未返回收回匹，链断，显式判红');
    }
    S.recovered = recovered;
    console.log(
      `[05-label] 委外链就绪：缸=${dyeLotNo} 收回匹=${recovered.piece_no}(id=${recovered.id})`
    );

    // 同链报工的生产匹（piece_type=greige，匹号含本 run 专属 tag，跨历史数据可唯一定位）
    const greigeRes = await apiCallRaw<unknown>(
      page,
      'GET',
      `/inventory/pieces?piece_type=greige&product_id=${productId}&page=1&page_size=100`
    );
    const greigeRows = pickListArray<Record<string, unknown>>(
      greigeRes,
      'items',
      '生产匹回读 /inventory/pieces'
    );
    const greigeHit = greigeRows.find(r => String(r.piece_no ?? '').includes('LBL05P'));
    if (!greigeHit?.id) {
      throw new Error(
        `[05-label] 委外链报工产出的生产匹（piece_no 含 LBL05P）未从 GET /inventory/pieces 命中——` +
          `真实链断裂（报工落匹/列表回读口径任一断点），显式判红，勿 skip`
      );
    }
    S.greigePiece = { id: Number(greigeHit.id), pieceNo: String(greigeHit.piece_no) };
    console.log(`[05-label] 生产匹就绪：${S.greigePiece.pieceNo}(id=${S.greigePiece.id})`);

    // —— 前置链 B：验布记录创建 → 开始 → 评级 → 打卷（三实测值真实提交，新必填契约）——
    // fabric_width_inches 必传：评级默认四分制（fabric_scoring::FOUR_POINT，
    // fabric_inspection_service.rs:236-246），grade_inspection 对四分制强制要求
    // fabric_width_inches（:476-481，缺列 AppError::business fail-closed 正确，
    // #4671 判责 shard-32 本用例 step1 死于 POST .../grade「业务处理失败」即此门控）；
    // 幅宽是验布录入真实采集列（先例 quality/03-four-point.spec.ts:54 传 '60.00'），
    // 属用例缺前置，不是后端缺陷——不得反向放宽「四分制需幅宽」。
    const inspection = await apiCall<{ id?: number; inspection_no?: string }>(
      page,
      'POST',
      '/production/fabric-inspections',
      {
        inspection_date: new Date().toISOString().slice(0, 10),
        dye_lot_no: dyeLotNo,
        product_id: productId,
        color_no: colorNo,
        fabric_width_inches: '60.00',
        inspector_name: 'E2E-标签测试员',
        remarks: 'E2E 05 成品布入库标签前置验布单',
      }
    );
    const inspectionId = inspection.data?.id;
    if (!inspectionId) {
      throw new Error(
        `[05-label] 验布记录创建未回 id（响应 ${JSON.stringify(inspection).slice(0, 200)}），链断判红`
      );
    }
    await apiCall(page, 'POST', `/production/fabric-inspections/${inspectionId}/start`);
    await apiCall(page, 'POST', `/production/fabric-inspections/${inspectionId}/grade`, {
      inspected_yards: 100,
    });
    const roll = await apiCall<{ id?: number; status?: string }>(
      page,
      'POST',
      `/production/fabric-inspections/${inspectionId}/roll`,
      {
        warehouse_id: warehouse.id,
        roll_length: ROLL_LENGTH_M,
        roll_weight: ROLL_WEIGHT_KG,
        roll_width: ROLL_WIDTH_CM,
        roll_gram_weight: ROLL_GRAM,
      }
    );
    // 打卷返回的是更新后的验布单（InspectionModel，rolled），不含新匹 id——FE-220 报告 §①
    // 同口径：按缸号+dyed 回查定位新匹是契约内正路，不视为绕行。
    expect(
      roll.data?.status,
      `打卷后验布单应流转 rolled，实际 ${JSON.stringify(roll.data).slice(0, 200)}`
    ).toBe('rolled');
    console.log(`[05-label] 打卷成功：验布单 ${inspection.data?.inspection_no} → rolled`);

    // —— 回读逐列核对：新匹 = 该缸 dyed 匹中不在「打卷前已知集合」的行 ——
    const after = await listDyedPiecesByLot(page, dyeLotNo);
    const newPieces = after.filter(p => p.id !== recovered.id);
    if (newPieces.length !== 1) {
      throw new Error(
        `[05-label] 打卷后缸 ${dyeLotNo} 下应恰好新增 1 匹染色匹，实际新增 ${newPieces.length} 匹` +
          `（回读=${newPieces.map(p => p.piece_no).join(',')}）——写后回读不符，显式判红`
      );
    }
    const rolled = newPieces[0];
    expect(rolled.piece_no, `匹号应为 ${dyeLotNo}-xxx 形态`).toMatch(
      new RegExp(`^${dyeLotNo.replace(/-/g, '\\-')}-\\d{3}$`)
    );
    expect(String(rolled.dye_lot_no), '落库缸号应等于种子缸号').toBe(dyeLotNo);
    expect(String(rolled.batch_no), '落库批次应等于缸号（打卷写入方口径）').toBe(dyeLotNo);
    expect(String(rolled.color_no), '落库色号应等于验布单透传的色号').toBe(colorNo);
    expect(rolled.piece_type, '打卷产匹 piece_type 必须为 dyed（词表小写）').toBe('dyed');
    expect(String(rolled.status), '产匹状态应为 AVAILABLE（词表大写）').toBe('AVAILABLE');
    // 列表响应 length/weight 为 Decimal（序列化 number 或尾零字符串均可能，如 "50.25"/"50.2500"
    // ——DB 列标度是存储格式不是值差异），按数值精确等值核对种子；尾零归一后与后端标签
    // 装配 fmt_dec=Decimal::normalize（print_service.rs:5030）同形，作为用例 2 正文断言基准。
    // 另：width/gram_weight 不在 GET /inventory/pieces 出参（PieceResponse，
    // backend/src/handlers/inventory_piece_handler.rs:45-67 无该两列）——e2e 无法从列表回读
    // 这两列落库值，改由用例 2 的标签正文逐值断言覆盖（该缺口已按「疑点」上报，不隐瞒）。
    const lengthNorm = normDec(rolled.length);
    const weightNorm = normDec(rolled.weight);
    expect(lengthNorm, `落库米数应等于打卷提交实测值 ${ROLL_LENGTH_M}`).toBe(
      normDec(ROLL_LENGTH_M)
    );
    expect(weightNorm, `落库重量应等于打卷提交实测值 ${ROLL_WEIGHT_KG}`).toBe(
      normDec(ROLL_WEIGHT_KG)
    );

    S.productId = productId;
    S.warehouseId = warehouse.id;
    S.dyeLotNo = dyeLotNo;
    S.colorNo = colorNo;
    S.rolled = {
      id: rolled.id,
      pieceNo: rolled.piece_no,
      length: lengthNorm,
      weight: weightNorm,
      width: normDec(ROLL_WIDTH_CM),
      gramWeight: normDec(ROLL_GRAM),
    };
    S.ready = true;
    console.log(
      `[05-label] ✅ 用例1 就绪：打卷匹 ${S.rolled.pieceNo}(id=${S.rolled.id}) ` +
        `米=${S.rolled.length} 重=${S.rolled.weight}`
    );
  });

  test('2. 标签 docx 逐值断言：JSZip 解 word/document.xml，8 字段种子值精确命中', async ({
    page,
  }) => {
    test.setTimeout(120_000);
    requireReady('用例2 内容断言');
    const rolled = S.rolled!;
    const docXml = await fetchLabelDocXml(page, rolled.id);
    S.docXml = docXml;
    console.log(`[05-label] document.xml 解包成功 ${docXml.length}B`);

    // 款号/品名 = LEFT JOIN products（print_service.rs PieceLabelView），取真实产品行核对
    const prod = await apiCallRaw<{ code?: string; name?: string }>(
      page,
      'GET',
      `/products/${S.productId}`
    );
    if (!prod.code || !prod.name) {
      throw new Error(
        `[05-label] 产品行缺 code/name（实际 ${JSON.stringify(prod).slice(0, 200)}），` +
          '标签款号/品名富化无基准，显式判红'
      );
    }

    // 逐值断言（种子精确匹配，不许只断非空）：缸号/色号/批次/匹号/米数/重量/幅宽/克重
    const expects: Array<[string, string]> = [
      ['缸号', S.dyeLotNo!],
      ['色号', S.colorNo!],
      // 批次写入方口径 = 缸号同值（fabric_inspection_service.rs:677），值随缸号一并命中
      ['批次(=缸号)', S.dyeLotNo!],
      ['匹号(条码同源)', rolled.pieceNo],
      ['米数', rolled.length],
      ['重量', rolled.weight],
      ['幅宽', rolled.width],
      ['克重', rolled.gramWeight],
      ['等级(quality_status PASS=合格)', '合格'],
      ['款号(products.code)', prod.code],
      ['品名(products.name)', prod.name],
    ];
    for (const [label, value] of expects) {
      expect(
        docXml.includes(value),
        `标签正文应出现${label}的种子值「${value}」——逐值内容匹配（#220 契约：字段全取匹行实测值）`
      ).toBeTruthy();
      console.log(`[05-label] ✅ 内容命中 ${label}=${value}`);
    }
  });

  test('3. 保密反证：标签正文零出现供应商侧编码/成本键', async ({ page }) => {
    test.setTimeout(120_000);
    requireReady('用例3 保密反证');
    // 哨兵值可否真造——已逐写入点核实（本仓全量 supplier_piece_no 产出点）：
    //   backend/src/services/piece_domain_service.rs:113、:566 —— 委外收回/发料路径 Set(None)；
    //   backend/src/handlers/piece_split_handler.rs:220 —— 拆匹子匹 ActiveValue::NotSet；
    //   backend/src/services/fabric_inspection_service.rs:696 —— 打卷产匹 Default::default()；
    //   backend/src/services/bulk_color_approval_service.rs:458 —— 样布路径 Set(None)。
    // 无任何 API 入参可把值写进 inventory_piece.supplier_piece_no/成本列——值级哨兵**真造不出**，
    // 按派工纪律不造假（不经 API 直改 DB 属造假数据链），也不静默 skip：
    // 本用例降级为键名/语义级零出现反证（仍是真实内容断言，钉死 PieceLabelView 泄露面），
    // 「值级哨兵不可造 + 不可造的代码证据」已如实写入交付报告，交回编排方决策是否补 DB 级契约测。
    const docXml = S.docXml ?? (await fetchLabelDocXml(page, S.rolled!.id));
    const lower = docXml.toLowerCase();
    const forbidden: Array<[string, string]> = [
      ['supplier_piece_no 键名', 'supplier_piece_no'],
      ['供应商匹号(英文词根)', 'supplier'],
      ['成本单价键', 'unit_cost'],
      ['成本总额键', 'total_cost'],
    ];
    for (const [label, token] of forbidden) {
      expect(
        lower.includes(token),
        `标签正文禁止出现${label}（token=「${token}」）——保密口径（裁定 §6）零出现反证`
      ).toBe(false);
    }
    for (const zh of ['供应商', '成本']) {
      expect(
        docXml.includes(zh),
        `标签正文禁止出现中文语义「${zh}」——标签字段集不含任何供应商/成本信息`
      ).toBe(false);
    }
    console.log(
      '[05-label] ✅ 保密反证通过：正文零出现供应商侧编码/成本键（值级哨兵不可造，见注释证据）'
    );
  });

  test('4. 缺列 fail-closed：委外收回 NULL 三列的真实 dyed 匹 → 400 且逐列点名', async ({
    page,
  }) => {
    test.setTimeout(120_000);
    requireReady('用例4 缺列拒绝');
    const recovered = S.recovered!;
    // 该匹来自用例 1 的真实委外染整收回确认：写入点 piece_domain_service.rs:560-562 将
    // weight/width/gram_weight 置 None（后端报告 §⑤ 声明的存量真实场景；打卷新必填造不出 NULL 匹，
    // 故取收回路径而非造数放宽——断言对象是产品真实行为，不是测试臆造的坏数据）。
    const fail = await apiCallExpectFail(page, 'GET', `/inventory/pieces/${recovered.id}/print`);
    console.log(
      `[05-label] 缺列匹打印 → status=${fail.status} code=${String(fail.code)} message=${String(fail.message)}`
    );
    expect(fail.status, '缺必填列的 dyed 匹打印应 400').toBe(400);
    expect(
      failureCode(fail),
      `失败信封 code 应为 VALIDATION_ERROR（字段族），实际=${String(fail.code)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    const message = typeof fail.message === 'string' ? fail.message : '';
    expect(
      message,
      'fail-closed 文案不得是脱敏常量「请求参数验证失败」（可外显族必须点名）'
    ).not.toBe('请求参数验证失败');
    for (const col of ['重量(weight)', '幅宽(width)', '克重(gram_weight)']) {
      expect(
        message.includes(col),
        `message 应点名缺失列「${col}」（委外收回匹三列 NULL），实际 message=「${message}」`
      ).toBe(true);
    }
    expect(
      message.includes(recovered.piece_no),
      `message 应带用户可见匹号 ${recovered.piece_no}，实际=「${message}」`
    ).toBe(true);
    expect(
      message.toLowerCase().includes('inventory_piece'),
      `message 不得泄露表名（保密/信息最小化），实际=「${message}」`
    ).toBe(false);
  });

  test('5. 门控：生产匹(非 dyed) → BUSINESS 族（只断 status/code，脱敏出参不断原因）', async ({
    page,
  }) => {
    test.setTimeout(120_000);
    requireReady('用例5 门控拒绝');
    const greige = S.greigePiece!;
    // 样布(SAMPLE)族需大货色审批链才产出、本文件归属不重复建重链；派工口径为
    // 「非 dyed（生产匹）**或**样布」任一即覆盖门控——生产匹即满足族别断言。
    const fail = await apiCallExpectFail(page, 'GET', `/inventory/pieces/${greige.id}/print`);
    console.log(
      `[05-label] 生产匹门控 → status=${fail.status} code=${String(fail.code)}（message 脱敏不断言）`
    );
    expect(fail.status, '非 dyed 匹打印应 400（BUSINESS 族状态码）').toBe(400);
    expect(
      failureCode(fail),
      `门控失败 code 应为 BUSINESS_ERROR（门控先于字段校验：该生产匹同样缺幅宽/克重列，` +
        `若返回 VALIDATION_ERROR 即门控顺序漂移，族别串号，勿放宽）`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
  });
});

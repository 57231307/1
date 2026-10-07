/**
 * 不合格品（缺陷台账）出参形状锁：后端真实构造点 ↔ 前端声明 ↔ 视图绑定三向互证。
 *
 * 功能：
 * 1) 钉住 GET /production/quality-inspection/defects 与 POST .../defects/{id}/process 的
 *    出下载体就是 unqualified_product::Model 整行（服务层直接序列化实体 Vec，
 *    无 DTO、无 into_model、无 json! 手拼补键），故出参键集合以模型字段名为唯一来源；
 * 2) 前端 UnqualifiedProductRecord 的声明键必须是后端键的子集，且不得出现历史上那批
 *    后端从不输出的幽灵键（quantity/defect_type/defect_description/severity/processed/
 *    processed_by/processed_at/record_id）；
 * 3) DDL 里 NOT NULL 的列在前端声明里不得标可选（可选＝把缺键伪装成可空，取不到值时静默）；
 *    后端 Decimal 列在前端必须是 string（rust_decimal 默认序列化为十进制字符串）；
 * 4) 处理状态词表在前端 constants 与后端 status 常量之间逐项相等，且视图门控读的是
 *    handling_status 的真实取值，不再读那个恒真的布尔幽灵键；
 * 5) 检测力自证：把上述四类违例分别注入源码副本，判据必须逐条抓到（防判据静默失效）。
 *
 * 调用方：vitest（tests/unit，CI vitest job）。
 * 入参：仅 fs 读取仓库内源码文本，不执行后端代码、不发网络请求。
 * 传给谁：纯断言，无下游；存什么/存哪里：不落盘、不存储。
 */
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND_ROOT = path.resolve(HERE, '../..');
const REPO_ROOT = path.resolve(FRONTEND_ROOT, '..');

/**
 * 读仓库内源码并统一行尾为 LF：Windows 检出在 core.autocrlf=true 下为 CRLF、CI 检出为 LF，
 * 本锁的字面量夹具 replace 与逐行正则对行尾形态敏感，不归一会让检测力自证被误判成判据失效。
 */
function readRepoSource(absPath: string): string {
  return readFileSync(absPath, 'utf8').replace(/\r\n/g, '\n');
}

const API_QUALITY_SRC = readRepoSource(path.join(FRONTEND_ROOT, 'src/api/quality.ts'));
const DEFECT_TAB_SRC = readRepoSource(
  path.join(FRONTEND_ROOT, 'src/views/quality/tabs/DefectTab.vue')
);
const HANDLING_CONST_SRC = readRepoSource(
  path.join(FRONTEND_ROOT, 'src/constants/quality-unqualified-handling.ts')
);
const UNQUALIFIED_MODEL_SRC = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/models/unqualified_product.rs')
);
const QUALITY_HANDLER_SRC = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/handlers/quality_inspection_handler.rs')
);
const QUALITY_SERVICE_SRC = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/services/quality_inspection_service.rs')
);
const STATUS_MODEL_SRC = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/models/status/quality_dyeing.rs')
);
const UNQUALIFIED_DDL_SRC = readRepoSource(
  path.join(
    REPO_ROOT,
    'backend/migration/src/domain/business/m0013_add_business_process_and_traceability.rs'
  )
);

/** 历史上写进前端声明、后端该端点从不输出的幽灵键：出现即判红 */
const GHOST_KEYS = [
  'quantity',
  'defect_type',
  'defect_description',
  'severity',
  'processed',
  'processed_by',
  'processed_at',
  'record_id',
] as const;

/** 缺陷台账必展示列（删列即判红，不得再退成推导值） */
const REQUIRED_REAL_KEYS = [
  'unqualified_qty',
  'handling_status',
  'handling_method',
  'handling_reason',
] as const;

/** 截取 TS interface 正文；接口被删除或改名即抛错（防判据静默失去覆盖） */
function tsInterfaceBody(src: string, name: string): string {
  const start = src.indexOf(`export interface ${name} {`);
  if (start === -1) {
    throw new Error(`TS 源码中未找到 interface ${name}——结构已漂移，锁需同步修订`);
  }
  const end = src.indexOf('\n}', start);
  if (end === -1) {
    throw new Error(`interface ${name} 块未闭合，无法解析`);
  }
  return src.slice(start, end);
}

type TsField = { key: string; optional: boolean; type: string };

/** 提取 TS 接口块内的字段（键名、是否可选、类型原文） */
function tsInterfaceFields(src: string, name: string): TsField[] {
  const body = tsInterfaceBody(src, name);
  return [...body.matchAll(/^ {2}([a-z][A-Za-z0-9_]*)(\??):\s*([^;]+);/gm)].map(m => ({
    key: m[1],
    optional: m[2] === '?',
    type: m[3].trim(),
  }));
}

function tsInterfaceKeys(src: string, name: string): string[] {
  return tsInterfaceFields(src, name).map(f => f.key);
}

/** 截取 Rust struct 正文；结构体改名/删除即抛错（防判据静默失效） */
function rustStructBody(src: string, name: string): string {
  const start = src.indexOf(`pub struct ${name} {`);
  if (start === -1) {
    throw new Error(`后端源码中未找到 pub struct ${name}——载体已漂移，锁需同步修订`);
  }
  const end = src.indexOf('\n}', start);
  if (end === -1) {
    throw new Error(`pub struct ${name} 块未闭合，无法解析`);
  }
  return src.slice(start, end);
}

type RustField = { key: string; type: string };

/** 提取 Rust struct 字段（serde 序列化键＝字段名，该模型无 rename_all） */
function rustStructFields(body: string): RustField[] {
  return [...body.matchAll(/^ {4}pub ([a-z_][a-z0-9_]*)\s*:\s*(.+?),\s*$/gm)].map(m => ({
    key: m[1],
    type: m[2].trim(),
  }));
}

/** 截取 `pub mod X {` 到其闭合 `}` 的模块正文 */
function rustModBody(src: string, name: string): string {
  const start = src.indexOf(`pub mod ${name} {`);
  if (start === -1) {
    throw new Error(`后端源码中未找到 pub mod ${name}——词表载体已漂移，锁需同步修订`);
  }
  const end = src.indexOf('\n}', start);
  return src.slice(start, end);
}

/** 从模块正文提取 `pub const X: &str = "value";` 的值集合 */
function rustStrConstValues(body: string): string[] {
  return [...body.matchAll(/pub const [A-Z_]+:\s*&str\s*=\s*"([^"]+)"/g)].map(m => m[1]);
}

/** 从 TS 常量对象块提取值集合（形如 `  key: 'value',`，块以行首 `}` 闭合） */
function tsConstObjectValues(src: string, name: string): string[] {
  const start = src.indexOf(`export const ${name} = {`);
  if (start === -1) {
    throw new Error(`前端源码中未找到 export const ${name}——词表已漂移，锁需同步修订`);
  }
  const end = src.indexOf('\n}', start);
  if (end === -1) {
    throw new Error(`export const ${name} 块未闭合，无法解析`);
  }
  return [...src.slice(start, end).matchAll(/^\s+[A-Za-z][A-Za-z0-9_]*:\s*'([^']+)'/gm)].map(
    m => m[1]
  );
}

/** 提取建表 DDL 里显式 NOT NULL 的列名 */
function ddlNotNullColumns(src: string, table: string): string[] {
  const start = src.indexOf(`CREATE TABLE IF NOT EXISTS "${table}" (`);
  if (start === -1) {
    throw new Error(`迁移源码中未找到 ${table} 建表语句——DDL 已漂移，锁需同步修订`);
  }
  const end = src.indexOf('\n);', start);
  const block = src.slice(start, end);
  return [...block.matchAll(/^\s*"([a-z_][a-z0-9_]*)"\s+[A-Z][^\n]*NOT NULL/gm)].map(m => m[1]);
}

const backendFields = rustStructFields(rustStructBody(UNQUALIFIED_MODEL_SRC, 'Model'));
const backendKeys = backendFields.map(f => f.key);
const frontendFields = tsInterfaceFields(API_QUALITY_SRC, 'UnqualifiedProductRecord');
const frontendKeys = frontendFields.map(f => f.key);
const backendStatusValues = rustStrConstValues(
  rustModBody(STATUS_MODEL_SRC, 'quality_handling')
).sort();

describe('不合格品（缺陷台账）出参形状锁（后端构造点 ↔ 前端声明 ↔ 视图门控）', () => {
  it('后端事实基准：defects 两个端点的载荷就是 unqualified_product::Model，出口不补键', () => {
    // 列表与处理两个 handler 的返回类型都直接写实体，没有中间 DTO
    expect(QUALITY_HANDLER_SRC).toContain(
      'Result<Json<ApiResponse<Vec<unqualified_product::Model>>>, AppError>'
    );
    expect(QUALITY_HANDLER_SRC).toContain(
      'Result<Json<ApiResponse<unqualified_product::Model>>, AppError>'
    );
    // 服务层：查询对象是实体本身，返回实体 Vec（既不是 json! 手拼，也没有 into_model::<Dto>()）
    const listFn = QUALITY_SERVICE_SRC.slice(
      QUALITY_SERVICE_SRC.indexOf('pub async fn get_defects_list')
    );
    expect(listFn).toContain('let mut query = unqualified_product::Entity::find();');
    expect(listFn.slice(0, listFn.indexOf('\n    }\n'))).not.toMatch(/json!/);
    expect(listFn.slice(0, listFn.indexOf('\n    }\n'))).not.toContain('into_model');
    // 筛选入参落在这张真实列上，故前端筛选值必须与写入值逐字符相同
    expect(listFn).toContain('unqualified_product::Column::HandlingStatus.eq(status)');
    // 模型键集地板：解析结果非空且规模合理（空集/解析失效判红）
    expect(backendKeys.length).toBeGreaterThanOrEqual(24);
    // 真实数量列与状态列在模型侧确实存在（证明删的是"后端无此出参键"，不是解析失效）
    expect(backendKeys).toContain('unqualified_qty');
    expect(backendKeys).toContain('handling_status');
    for (const key of GHOST_KEYS) {
      expect(backendKeys, `unqualified_products 模型不应含键 ${key}`).not.toContain(key);
    }
  });

  it('前端声明：键集合 ⊆ 后端出参键，零幽灵键，且只有一份该实体的声明', () => {
    expect(frontendKeys.length).toBeGreaterThanOrEqual(24);
    const extra = frontendKeys.filter(k => !backendKeys.includes(k)).sort();
    expect(extra, `前端声明了后端不输出的键：${extra.join(' ')}`).toEqual([]);
    for (const key of GHOST_KEYS) {
      expect(frontendKeys, `UnqualifiedProductRecord 不得声明幽灵键 ${key}`).not.toContain(key);
    }
    for (const key of REQUIRED_REAL_KEYS) {
      expect(frontendKeys, `声明缺少真实列 ${key}`).toContain(key);
    }
    // 同一份出参在同一个 api 模块里只允许一个声明（历史 Defect 与 UnqualifiedProductRecord
    // 双份并存，其中一份漂移成七个幽灵键）
    expect(API_QUALITY_SRC).not.toMatch(/export interface Defect\s*\{/);
    expect(API_QUALITY_SRC.match(/export interface UnqualifiedProductRecord\s*\{/g)?.length).toBe(
      1
    );
    // 列表端点的返回类型必须指向这份真实声明
    expect(API_QUALITY_SRC).toContain('Promise<ApiResponse<UnqualifiedProductRecord[]>>');
  });

  it('可空性与类型：DDL NOT NULL 列不得标可选，后端 Decimal 列必须是 string', () => {
    const notNull = ddlNotNullColumns(UNQUALIFIED_DDL_SRC, 'unqualified_products');
    // DDL 解析地板（解析失效即判红，防止"零列必过"的假绿）
    expect(notNull.length).toBeGreaterThanOrEqual(8);
    for (const col of notNull) {
      const field = frontendFields.find(f => f.key === col);
      // 前端可以完全不声明某列（列表不展示），但声明了就必须非可选
      if (field) {
        expect(field.optional, `NOT NULL 列 ${col} 在前端不得标为可选（掩盖缺键）`).toBe(false);
      }
    }
    // NOT NULL 且界面要绑定的列必须真的被声明（防止"删光断言对象"造成判据空转）
    for (const key of ['unqualified_no', 'unqualified_qty', 'unqualified_reason']) {
      expect(notNull, `DDL 里 ${key} 应为 NOT NULL，判据失效`).toContain(key);
      expect(frontendKeys).toContain(key);
    }
    for (const field of backendFields) {
      if (!/Decimal/.test(field.type)) continue;
      const declared = frontendFields.find(f => f.key === field.key);
      expect(declared, `后端 Decimal 列 ${field.key} 前端必须声明`).toBeDefined();
      expect(
        declared?.type,
        `rust_decimal 出参是十进制字符串，${field.key} 不得声明为 number`
      ).toMatch(/string/);
    }
    expect(frontendFields.find(f => f.key === 'unqualified_qty')?.type).toBe('string');
  });

  it('词表与门控：handling_status 取值双侧相等，视图按真实列判定动作', () => {
    expect(backendStatusValues.length).toBeGreaterThanOrEqual(3);
    const frontendStatusValues = tsConstObjectValues(
      HANDLING_CONST_SRC,
      'QUALITY_HANDLING_STATUS'
    ).sort();
    expect(frontendStatusValues, '前端处理状态词表必须与后端写入方常量逐项相等').toEqual(
      backendStatusValues
    );
    // 处理方式同样是闭合词表（后端按等级校验组合，界面不得自造第四值）
    const methodValues = tsConstObjectValues(HANDLING_CONST_SRC, 'QUALITY_HANDLING_METHOD');
    expect(methodValues.sort()).toEqual(['downgrade_sale', 'rework', 'scrap'].sort());
    for (const m of ['HANDLING_DOWNGRADE_SALE', 'HANDLING_REWORK', 'HANDLING_SCRAP']) {
      expect(QUALITY_SERVICE_SRC).toContain(`pub const ${m}`);
    }

    // 门控：读 handling_status 的真实取值，且不再读那个恒真的布尔幽灵键。
    // D1② 契约演进：处理动作从"按质检记录开单"改为"台账行原地更新"（process-result），
    // 旧的临时判据 `inspection_id !== null` 已删除（防回潮负断言），scrap 行按端点
    // 状态门排除；开单/更新的 {id} 语义分工由 quality-defect-process-result-shape-lock 钉。
    expect(DEFECT_TAB_SRC).toContain('row.handling_status === QUALITY_HANDLING_STATUS.pending &&');
    expect(DEFECT_TAB_SRC).toContain('row.handling_method !== QUALITY_HANDLING_METHOD.scrap');
    expect(DEFECT_TAB_SRC, 'inspection_id 临时门控不得回潮').not.toContain(
      'inspection_id !== null'
    );
    expect(DEFECT_TAB_SRC).not.toMatch(/row\.processed/);
    expect(DEFECT_TAB_SRC).not.toMatch(/v-if="!row\./);
    // 列绑定只能出现在后端真实键上
    const boundProps = [...DEFECT_TAB_SRC.matchAll(/prop="([a-z_][a-z0-9_]*)"/g)].map(m => m[1]);
    expect(boundProps.length).toBeGreaterThanOrEqual(6);
    for (const prop of boundProps) {
      expect(backendKeys, `视图绑定了后端不输出的列 ${prop}`).toContain(prop);
    }
    // 双向锁：必展示列在视图 prop= 绑定中必须出现（删列即判红）
    for (const key of REQUIRED_REAL_KEYS) {
      expect(boundProps, `视图缺少必展示列 ${key}`).toContain(key);
    }
    // 无数据源的列直接删，不得留 ?? '-' 之类的假列占位
    expect(DEFECT_TAB_SRC).not.toContain("?? '-'");
  });

  it('检测力自证：四类违例注入副本后判据必须逐条抓到（防判据静默失效）', () => {
    // 违例 A：把幽灵键写回声明
    const withGhost = API_QUALITY_SRC.replace(
      '  unqualified_qty: string;',
      '  quantity: number;\n  unqualified_qty: string;'
    );
    expect(withGhost, '夹具替换本身必须生效').not.toBe(API_QUALITY_SRC);
    const ghostKeys = tsInterfaceKeys(withGhost, 'UnqualifiedProductRecord').filter(k =>
      GHOST_KEYS.includes(k as (typeof GHOST_KEYS)[number])
    );
    expect(ghostKeys).toContain('quantity');

    // 违例 B：声明了后端没有的键（子集判据必须抓到差集非空）
    const withUnknown = API_QUALITY_SRC.replace(
      '  unqualified_qty: string;',
      '  defect_class_code: string;\n  unqualified_qty: string;'
    );
    const extra = tsInterfaceKeys(withUnknown, 'UnqualifiedProductRecord').filter(
      k => !backendKeys.includes(k)
    );
    expect(extra).toContain('defect_class_code');

    // 违例 C：NOT NULL 列被标成可选
    const withOptional = API_QUALITY_SRC.replace(
      '  unqualified_no: string;',
      '  unqualified_no?: string;'
    );
    const notNull = ddlNotNullColumns(UNQUALIFIED_DDL_SRC, 'unqualified_products');
    const violatedOptional = tsInterfaceFields(withOptional, 'UnqualifiedProductRecord')
      .filter(f => notNull.includes(f.key) && f.optional)
      .map(f => f.key);
    expect(violatedOptional).toContain('unqualified_no');

    // 违例 D：Decimal 列被声明成 number
    const withNumberDecimal = API_QUALITY_SRC.replace(
      '  unqualified_qty: string;',
      '  unqualified_qty: number;'
    );
    const qtyField = tsInterfaceFields(withNumberDecimal, 'UnqualifiedProductRecord').find(
      f => f.key === 'unqualified_qty'
    );
    expect(qtyField?.type).not.toMatch(/string/);

    // 违例 E：门控退回恒真的布尔键写法
    const withGhostGate = DEFECT_TAB_SRC.replace('v-if="canProcess(row)"', 'v-if="!row.processed"');
    expect(withGhostGate, '夹具替换本身必须生效').not.toBe(DEFECT_TAB_SRC);
    expect(withGhostGate).toMatch(/row\.processed/);

    // 违例 F：前端词表少一项（与后端写入值不再相等）
    const trimmedVocab = HANDLING_CONST_SRC.replace(
      "  pending: 'pending',\n  approved: 'approved',\n  rejected: 'rejected',\n} as const;",
      "  pending: 'pending',\n  approved: 'approved',\n} as const;"
    );
    expect(trimmedVocab, '夹具替换本身必须生效').not.toBe(HANDLING_CONST_SRC);
    expect(tsConstObjectValues(trimmedVocab, 'QUALITY_HANDLING_STATUS').sort()).not.toEqual(
      backendStatusValues
    );

    // 反向自证：解析对象被改名/删除时必须抛错，而不是静默给出空集让判据全绿
    expect(() =>
      tsInterfaceBody('export const nothing = 1;\n', 'UnqualifiedProductRecord')
    ).toThrow();
    expect(() => rustStructBody('pub fn nothing() {}\n', 'Model')).toThrow();
    expect(() =>
      ddlNotNullColumns('CREATE TABLE other (a INT);\n', 'unqualified_products')
    ).toThrow();
  });
});

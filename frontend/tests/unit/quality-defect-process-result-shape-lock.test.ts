/**
 * 不合格品台账行「处置结果原地更新」（D1② /process-result）请求侧形状锁：
 * 后端 DTO/路由 ↔ 前端 api 声明 ↔ 视图提交点三方互证。
 *
 * 钉死四件事：
 * 1) 新端点路径逐字符一致：api/quality.ts 的模板串与 backend routes/production.rs
 *    注册的 `{id}/process-result` → process_defect_result 互证（路径写错一个段即 404/403）；
 * 2) 入参键集 == 后端 ProcessResultRequest 结构体字段集（恰 handling_method + reason 两键，
 *    多一键少一键都判红——禁止索引签名吞掉键集失配）；
 * 3) 身份键（user_id/handling_by/updated_by/operator_id 族）在后端 DTO 类型层不存在，
 *    故前端载荷接口与视图提交字面量都不得出现任何一个（操作人只认服务端会话）；
 * 4) 处置方式 token 单源：视图选项引用 constants 词表（QUALITY_HANDLING_METHOD*），
 *    不得出现第二套带引号字面量；前端词表值与后端 HANDLING_* 常量值逐项相等。
 * 另钉 {id} 语义分工的防回潮断言：视图提交走 processDefectRow(row.id, …)，
 * 台账行动作路径参数取 inspection_id 的旧写法出现即判红。
 *
 * 检测力自证：把上述违例分别注入源码副本，判据必须逐条抓到；解析对象缺失即抛错、
 * 解析数为 0 即判红（防判据静默失去覆盖）。
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

/** 读仓库内源码并统一行尾为 LF（Windows CRLF 检出下夹具 replace 对行尾敏感） */
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
const ROUTES_SRC = readRepoSource(path.join(REPO_ROOT, 'backend/src/routes/production.rs'));
const SERVICE_SRC = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/services/quality_inspection_service.rs')
);

/** 操作人由服务端会话派生；这些身份键出现在请求侧任何一层都判红 */
const IDENTITY_KEYS: readonly string[] = [
  'user_id',
  'userId',
  'handling_by',
  'handlingBy',
  'updated_by',
  'updatedBy',
  'operator_id',
  'operatorId',
];

/** 截取 TS interface 正文；接口被删除或改名即抛错（防判据静默失效） */
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

function tsInterfaceFields(src: string, name: string): { key: string; optional: boolean }[] {
  const body = tsInterfaceBody(src, name);
  return [...body.matchAll(/^ {2}([a-z][A-Za-z0-9_]*)(\??):\s*[^;]+;/gm)].map(m => ({
    key: m[1],
    optional: m[2] === '?',
  }));
}

function tsInterfaceKeys(src: string, name: string): string[] {
  return tsInterfaceFields(src, name).map(f => f.key);
}

/** 截取 Rust struct 正文；结构体改名/删除即抛错 */
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

function rustStructFieldNames(body: string): string[] {
  return [...body.matchAll(/^ {4}pub ([a-z_][a-z0-9_]*)\s*:/gm)].map(m => m[1]);
}

const backendPayloadKeys = rustStructFieldNames(
  rustStructBody(SERVICE_SRC, 'ProcessResultRequest')
).sort();
const frontendPayloadKeys = tsInterfaceKeys(API_QUALITY_SRC, 'ProcessDefectResultPayload');

describe('台账行 process-result 请求侧形状锁（后端 DTO/路由 ↔ 前端声明 ↔ 视图提交点）', () => {
  it('地板：三处解析对象都真实存在且规模合理（解析塌缩即判红）', () => {
    expect(backendPayloadKeys.length).toBeGreaterThanOrEqual(2);
    expect(frontendPayloadKeys.length).toBeGreaterThanOrEqual(2);
    // 后端注册行恰好一条 process-result（复制注册/半改写判红）
    const routeHits = ROUTES_SRC.match(/\/quality-inspection\/defects\/\{id\}\/process-result/g);
    expect(routeHits?.length ?? 0).toBe(1);
    expect(ROUTES_SRC).toContain('post(quality_inspection_handler::process_defect_result)');
  });

  it('端点路径互证：api 模板串与后端注册逐字符一致，{id} 为台账行主键', () => {
    const fnStart = API_QUALITY_SRC.indexOf('export function processDefectRow(');
    expect(fnStart, 'api/quality.ts 缺少 processDefectRow——消费端未接线').not.toBe(-1);
    const fnBody = API_QUALITY_SRC.slice(fnStart, API_QUALITY_SRC.indexOf('\n}', fnStart));
    expect(fnBody).toContain(
      'request.post(`/production/quality-inspection/defects/${id}/process-result`, data)'
    );
    expect(fnBody).toContain('Promise<ApiResponse<UnqualifiedProductRecord>>');
    // 后端 nest 前缀 /production（routes 相对路径写法）——两侧拼接后同一绝对路径
    expect(ROUTES_SRC).toContain('"/quality-inspection/defects/{id}/process-result"');
  });

  it('入参键集双侧相等（恰 handling_method + reason），身份键零出现，reason 可选', () => {
    expect(
      frontendPayloadKeys.slice().sort(),
      '前端载荷键集必须与后端 ProcessResultRequest 字段集逐项相等'
    ).toEqual(backendPayloadKeys);
    const fields = tsInterfaceFields(API_QUALITY_SRC, 'ProcessDefectResultPayload');
    expect(fields.find(f => f.key === 'handling_method')?.optional, '必填键不得标可选').toBe(false);
    expect(fields.find(f => f.key === 'reason')?.optional, 'Option<String> 对应可选键').toBe(true);
    for (const key of IDENTITY_KEYS) {
      expect(backendPayloadKeys, `后端 DTO 不应含身份键 ${key}`).not.toContain(key);
      expect(frontendPayloadKeys, `前端载荷不应含身份键 ${key}`).not.toContain(key);
    }
    // 视图提交块（reactive 表单模型）同样不得携带身份键
    const formStart = DEFECT_TAB_SRC.indexOf('const processForm = reactive({');
    expect(formStart, 'DefectTab 未找到 processForm——提交面已漂移').not.toBe(-1);
    const formBlock = DEFECT_TAB_SRC.slice(formStart, DEFECT_TAB_SRC.indexOf('});', formStart));
    for (const key of IDENTITY_KEYS) {
      expect(formBlock, `视图表单模型不应含身份键 ${key}`).not.toContain(key);
    }
  });

  it('{id} 语义分工防回潮：视图按行自身 id 原地更新，不再拿 inspection_id 当动作参数', () => {
    expect(DEFECT_TAB_SRC).toContain('processDefectRow(row.id');
    // 台账行动作调用点传入来源记录 id 的旧契约写法出现即判红
    expect(DEFECT_TAB_SRC).not.toMatch(/processDefectRow\(\s*row\.inspection_id/);
    expect(DEFECT_TAB_SRC).not.toMatch(/inspection_id\s*!==\s*null/);
  });

  it('处置方式 token 单源：视图不写第二套字面量，前端词表与后端常量逐项相等', () => {
    // 视图内带引号的处置方式字面量 = 第二套真相源，判红（词表只能来自 constants）
    expect(DEFECT_TAB_SRC).not.toMatch(/['"](rework|downgrade_sale|scrap)['"]/);
    const tsVocab = [...HANDLING_CONST_SRC.matchAll(/^\s+[A-Za-z][A-Za-z0-9_]*:\s*'([^']+)'/gm)]
      .map(m => m[1])
      .filter(v => ['rework', 'downgrade_sale', 'scrap'].includes(v))
      .sort();
    const backendVocab = [
      ...SERVICE_SRC.matchAll(/pub const HANDLING_[A-Z_]+:\s*&str\s*=\s*"([^"]+)"/g),
    ]
      .map(m => m[1])
      .sort();
    expect(backendVocab.length).toBeGreaterThanOrEqual(3);
    expect(tsVocab, '前端处理方式词表必须与后端 HANDLING_* 常量逐项相等').toEqual(backendVocab);
    // 视图选项确实引用 constants 词表（而非各自拼装）
    expect(DEFECT_TAB_SRC).toContain('QUALITY_HANDLING_METHOD_VALUES');
  });

  it('检测力自证：违例注入副本后判据必须逐条抓到（防判据静默失效）', () => {
    // 违例 A：载荷接口多出一个后端不认的键（键集失配必须被集合相等判据抓到）
    const withExtra = API_QUALITY_SRC.replace(
      '  reason?: string;\n}',
      '  reason?: string;\n  handling_by?: number;\n}'
    );
    expect(withExtra, '夹具替换本身必须生效').not.toBe(API_QUALITY_SRC);
    const extraKeys = tsInterfaceKeys(withExtra, 'ProcessDefectResultPayload')
      .slice()
      .sort()
      .filter(k => !backendPayloadKeys.includes(k));
    expect(extraKeys).toContain('handling_by');

    // 违例 B：删掉 reason 键（后端有、前端少 = 契约收缩，同样必须抓到差集）
    const withoutReason = API_QUALITY_SRC.replace('  reason?: string;\n}', '}');
    expect(withoutReason, '夹具替换本身必须生效').not.toBe(API_QUALITY_SRC);
    const missing = backendPayloadKeys.filter(
      k => !tsInterfaceKeys(withoutReason, 'ProcessDefectResultPayload').includes(k)
    );
    expect(missing).toContain('reason');

    // 违例 C：身份键写进视图表单模型
    const withIdentity = DEFECT_TAB_SRC.replace(
      "  handling_method: '' as DefectHandlingMethod | '',",
      "  handling_method: '' as DefectHandlingMethod | '',\n  user_id: 0,"
    );
    expect(withIdentity, '夹具替换本身必须生效').not.toBe(DEFECT_TAB_SRC);
    const fStart = withIdentity.indexOf('const processForm = reactive({');
    const fBlock = withIdentity.slice(fStart, withIdentity.indexOf('});', fStart));
    expect(fBlock).toContain('user_id');

    // 违例 D：端点路径退回旧开单语义（process-result 被替换成 process）
    const withOldPath = API_QUALITY_SRC.replace(
      '/production/quality-inspection/defects/${id}/process-result',
      '/production/quality-inspection/defects/${id}/process'
    );
    const oldFnStart = withOldPath.indexOf('export function processDefectRow(');
    const oldFnBody = withOldPath.slice(oldFnStart, withOldPath.indexOf('\n}', oldFnStart));
    expect(oldFnBody).not.toContain('process-result');

    // 违例 E：视图自造第二套处置方式字面量
    const withLiteral = DEFECT_TAB_SRC.replace(
      'const processMethodOptions = computed(() =>',
      "const processMethodOptions = computed(() => // eslint-disable-next-line\n  ['rework'] as never; const _unused = () =>"
    );
    expect(withLiteral).toMatch(/['"](rework|downgrade_sale|scrap)['"]/);

    // 违例 F：{id} 语义回潮——动作参数退回来源质检记录 id
    const withInspectId = DEFECT_TAB_SRC.replace(
      'await processDefectRow(row.id,',
      'await processDefectRow(row.inspection_id!,'
    );
    expect(withInspectId).toMatch(/processDefectRow\(\s*row\.inspection_id/);

    // 反向自证：解析对象被删除时必须抛错而不是静默空集放行
    expect(() =>
      tsInterfaceBody('export const nothing = 1;\n', 'ProcessDefectResultPayload')
    ).toThrow();
    expect(() => rustStructBody('pub fn nothing() {}\n', 'ProcessResultRequest')).toThrow();
  });
});

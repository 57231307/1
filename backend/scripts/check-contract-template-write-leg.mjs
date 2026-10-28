/**
 * check-contract-template-write-leg.mjs — 合同模板读写腿静态判定器（切片 B）
 *
 * 判什么（全部是文本面静态判据，不依赖编译与活库，变异自证在本地跑）：
 * R1 路由形态 contract_template 路由文件存在，登记的路径全部以 /contract-templates 开头，
 *    且只出现在 routes/ 下（handler 不得自带 router）。
 * R2 权限词表 "contract-templates" 同时精确出现在 PERMISSION_RESOURCES 注册表与
 *    path_utils 已知资源段表两处；缺任一处即派生死键或白名单 403。
 * R3 词表同源 服务源码去整行注释后不得出现裸状态字面量 "draft" "active" "retired"；
 *    写 ACTIVE 与写 RETIRED 的 Set 语句各恰好一处（activate 与 retire 的目标行），
 *    出现第二处即自动降位或旁路启用通道。
 * R4 端点接线 十四条端点的 handler 符号在路由文件与 handler 文件双向存在。
 * R5 快照同步 route-snapshot.txt 必须已包含 /contract-templates 端点行（生成物滞后即红）。
 *
 * 用法：
 *   node scripts/check-contract-template-write-leg.mjs            # 对真实源码判定
 *   node scripts/check-contract-template-write-leg.mjs --self-test # 变异自证 每条规则喂
 *    合成违例源码必须判红并点名，喂真实源码必须判绿；任一检测力失守退出码非零
 */
import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const HERE = dirname(fileURLToPath(import.meta.url)); // backend/scripts
const BACKEND = join(HERE, '..'); // backend
const ROOT = join(BACKEND, '..'); // 仓库根
const SRC = join(BACKEND, 'src');

const SELF_TEST = process.argv.includes('--self-test');

function readRs(rel) {
  return readFileSync(join(SRC, rel), 'utf8');
}

/** 去掉整行 // 注释（Rust 行注释与文档注释同形，行首 // 即整行注释）。 */
function stripLineComments(text) {
  return text
    .split('\n')
    .filter((l) => !l.trimStart().startsWith('//'))
    .join('\n');
}

function count(hay, needle) {
  return hay.split(needle).length - 1;
}

// ---------------------------------------------------------------------------
// 检测器 每条返回错误数组（空数组=判绿），入参为源码文本，便于合成违例喂入
// ---------------------------------------------------------------------------

function r1RouteShape(routesSrc, handlerSrc) {
  const errs = [];
  if (routesSrc === null) {
    errs.push('R1 路由文件 src/routes/contract_template.rs 缺失');
    return errs;
  }
  const body = stripLineComments(routesSrc);
  const pathLits = [...body.matchAll(/"((?:\/[^"]*)?)"/g)].map((m) => m[1]).filter((p) => p.startsWith('/'));
  if (pathLits.length === 0) {
    errs.push('R1 路由文件解析不到任何路径字面量，判定器失去检测对象');
  }
  for (const p of pathLits) {
    if (!p.startsWith('/contract-templates')) {
      errs.push(`R1 路由文件出现非 contract-templates 前缀路径 ${p}（本域路由不得越界注册）`);
    }
  }
  if (stripLineComments(handlerSrc ?? '').includes('Router::new()')) {
    errs.push('R1 handler 文件自带 Router::new()，注册必须收敛在 routes/ 下');
  }
  return errs;
}

function r2PermissionRegistry(initSrc, pathUtilsSrc) {
  const errs = [];
  if (!initSrc || !initSrc.includes('"contract-templates"')) {
    errs.push('R2 PERMISSION_RESOURCES 缺 "contract-templates" 精确登记，权限目录与鉴权键漂移');
  }
  if (!pathUtilsSrc || !pathUtilsSrc.includes('"contract-templates"')) {
    errs.push('R2 path_utils 已知资源段表缺 "contract-templates"，白名单对全员 fail-closed 403');
  }
  return errs;
}

function r3VocabSingleChannel(serviceSrc) {
  const errs = [];
  const body = stripLineComments(serviceSrc ?? '');
  for (const token of ['"draft"', '"active"', '"retired"']) {
    if (body.includes(token)) {
      errs.push(`R3 服务源码出现状态裸字面量 ${token}，取值必须来自 contract_template_status 常量`);
    }
  }
  const activeSets = count(body, 'Set(contract_template_status::ACTIVE.to_string())');
  if (activeSets !== 1) {
    errs.push(`R3 服务层写 ACTIVE 的 Set 语句应恰好 1 处（activate 目标行），实得 ${activeSets} 处——第二处即自动降位或旁路启用通道`);
  }
  const retiredSets = count(body, 'Set(contract_template_status::RETIRED.to_string())');
  if (retiredSets !== 1) {
    errs.push(`R3 服务层写 RETIRED 的 Set 语句应恰好 1 处（retire 目标行），实得 ${retiredSets} 处`);
  }
  for (const c of ['DRAFT', 'ACTIVE', 'RETIRED']) {
    if (!body.includes(`contract_template_status::${c}`)) {
      errs.push(`R3 服务源码未消费 contract_template_status::${c} 常量，状态机比较点漂走`);
    }
  }
  return errs;
}

const ENDPOINT_SYMBOLS = [
  'list_templates',
  'get_template',
  'create_template',
  'update_template',
  'delete_template',
  'revise_template',
  'approve_template',
  'activate_template',
  'retire_template',
  'list_clauses',
  'add_clause',
  'update_clause',
  'delete_clause',
  'review_clause',
];

function r4Wiring(routesSrc, handlerSrc) {
  const errs = [];
  const body = stripLineComments(routesSrc ?? '');
  const hbody = handlerSrc ?? '';
  for (const sym of ENDPOINT_SYMBOLS) {
    if (!body.includes(sym)) {
      errs.push(`R4 路由文件缺少 handler 符号 ${sym} 的登记`);
    }
    if (!hbody.includes(`pub async fn ${sym}`)) {
      errs.push(`R4 handler 文件缺少 pub async fn ${sym}`);
    }
  }
  return errs;
}

function r5Snapshot(snapshotText) {
  const errs = [];
  const t = snapshotText ?? '';
  if (!t.includes('/contract-templates')) {
    errs.push('R5 route-snapshot.txt 未包含 /contract-templates 端点行，路由已改而快照滞后（重新生成 route-snapshot.mjs --write）');
  }
  return errs;
}

// ---------------------------------------------------------------------------
// 真实源码装载与执行
// ---------------------------------------------------------------------------

function readOpt(p) {
  try {
    return readFileSync(p, 'utf8');
  } catch {
    return null;
  }
}

function runOnRepo() {
  const routesSrc = readOpt(join(SRC, 'routes', 'contract_template.rs'));
  const handlerSrc = readOpt(join(SRC, 'handlers', 'contract_template_handler.rs'));
  const serviceSrc = readOpt(join(SRC, 'services', 'contract_template_service.rs'));
  const initSrc = readOpt(join(SRC, 'services', 'init_service.rs'));
  const pathUtilsSrc = readOpt(join(SRC, 'utils', 'path_utils.rs'));
  const snapshot = readOpt(join(ROOT, 'frontend', 'scripts', 'route-snapshot.txt'));
  const all = [
    ...r1RouteShape(routesSrc, handlerSrc),
    ...r2PermissionRegistry(initSrc, pathUtilsSrc),
    ...r3VocabSingleChannel(serviceSrc),
    ...r4Wiring(routesSrc, handlerSrc),
    ...r5Snapshot(snapshot),
  ];
  return all;
}

// ---------------------------------------------------------------------------
// 变异自证 每条规则至少一组合成违例必须被点名，真实源码必须判绿
// ---------------------------------------------------------------------------

function selfTest() {
  const failures = [];
  const repoErrs = runOnRepo();
  if (repoErrs.length > 0) {
    failures.push(`SELFTEST 真实源码判红，绿前提失守：\n  - ${repoErrs.join('\n  - ')}`);
  }
  const cases = [
    ['R1', r1RouteShape('.route("/other-things", get(h))', ''), /非 contract-templates 前缀/],
    ['R1', r1RouteShape('.route("/contract-templates", get(h))', 'fn x(){ let _=Router::new(); }'), /handler 文件自带/],
    ['R2', r2PermissionRegistry('[]', '"contract-templates"'), /PERMISSION_RESOURCES 缺/],
    ['R2', r2PermissionRegistry('"contract-templates"', ''), /path_utils 缺|已知资源段表缺/],
    ['R3', r3VocabSingleChannel('if s == "active" { Set(contract_template_status::ACTIVE.to_string()) } Set(contract_template_status::RETIRED.to_string()) contract_template_status::DRAFT contract_template_status::ACTIVE contract_template_status::RETIRED'), /裸字面量/],
    ['R3', r3VocabSingleChannel('Set(contract_template_status::ACTIVE.to_string()) Set(contract_template_status::ACTIVE.to_string()) contract_template_status::DRAFT contract_template_status::ACTIVE contract_template_status::RETIRED Set(contract_template_status::RETIRED.to_string())'), /写 ACTIVE 的 Set 语句应恰好 1 处/],
    ['R4', r4Wiring('', ''), /缺少 handler 符号/],
    ['R5', r5Snapshot('GET\t/api/v1/erp/nothing'), /快照/],
  ];
  for (const [rule, errs, pattern] of cases) {
    if (errs.length === 0) {
      failures.push(`SELFTEST ${rule} 的合成违例未被判红，检测力失守`);
    } else if (!errs.some((e) => pattern.test(e))) {
      failures.push(`SELFTEST ${rule} 判红消息未点名预期成因：${errs[0]}`);
    }
  }
  if (failures.length > 0) {
    console.error('_CTPL_WL_SELFTTEST_RED_');
    for (const f of failures) console.error(f);
    process.exit(1);
  }
  console.log('_CTPL_WL_SELFTTEST_OK_');
  return;
}

if (SELF_TEST) {
  selfTest();
} else {
  const errs = runOnRepo();
  if (errs.length > 0) {
    console.error('_CTPL_WL_RED_');
    for (const e of errs) console.error('  - ' + e);
    process.exit(1);
  }
  console.log('_CTPL_WL_OK_');
}

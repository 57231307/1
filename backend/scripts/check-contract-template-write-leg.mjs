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
 * R6 固化单点 合同条款快照写入通道全 src 只有 contract_clause_service.rs 一处
 *    （`contract_clause::ActiveModel` 出现面）；sales 与 purchase 两个创建 service 文件
 *    各自引用 render_and_pin_for_new_contract 且不出现快照 ActiveModel。
 * R7 渲染接线 两侧创建请求结构含 template_id clause_values contract_market tax_inclusive
 *    四字段；固化腿服务只读 contract_clauses 不 UPDATE 快照
 *    （src 面 `contract_clause::Entity::update` 零出现）；m0119 迁移不回填（无 UPDATE）。
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

import { readdirSync, statSync } from 'fs';

/** 递归收集 src 树内 .rs 文本（跳过注释行后返回拼接体）。 */
function collectRs(dir) {
  const out = [];
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) {
      out.push(...collectRs(p));
    } else if (name.endsWith('.rs')) {
      out.push({ file: p, body: stripLineComments(readFileSync(p, 'utf8')) });
    }
  }
  return out;
}

function r6FreezeSingleSource(srcFiles) {
  const errs = [];
  const writers = srcFiles
    .filter((f) => f.body.includes('contract_clause::ActiveModel'))
    .map((f) => f.file.replace(/\\/g, '/').split('/src/')[1]);
  if (writers.length !== 1 || writers[0] !== 'services/contract_clause_service.rs') {
    errs.push(`R6 快照写入通道应只有 services/contract_clause_service.rs 一处，实得 ${JSON.stringify(writers)}`);
  }
  for (const leg of ['services/sales_contract_service.rs', 'services/purchase_contract_service.rs']) {
    const f = srcFiles.find((x) => x.file.replace(/\\/g, '/').endsWith(leg));
    if (!f || !f.body.includes('render_and_pin_for_new_contract')) {
      errs.push(`R6 ${leg} 未消费渲染固化腿入口 render_and_pin_for_new_contract（合同创建腿没接线）`);
    }
  }
  const updaters = srcFiles
    .filter((f) => f.body.includes('contract_clause::Entity::update') || f.body.includes('contract_clause::ActiveModel'))
    .filter((f) => f.body.includes('Entity::update_many'))
    .map((f) => f.file);
  if (updaters.length > 0) {
    errs.push(`R6 出现改写既有快照行的通道 ${JSON.stringify(updaters)}——固化行只增不改，改版不回溯`);
  }
  return errs;
}

function r7RenderWiring(srcFiles, m0119Text) {
  const errs = [];
  for (const leg of ['services/sales_contract_service.rs', 'services/purchase_contract_service.rs']) {
    const f = srcFiles.find((x) => x.file.replace(/\\/g, '/').endsWith(leg));
    const body = f ? f.body : '';
    for (const field of ['template_id', 'clause_values', 'contract_market', 'tax_inclusive']) {
      if (!body.includes(`pub ${field}:`)) {
        errs.push(`R7 ${leg} 创建请求缺渲染字段 ${field}`);
      }
    }
  }
  const mig = m0119Text ?? '';
  if (!mig) {
    errs.push('R7 m0119 迁移文件缺失（快照来源指针列未落）');
  } else {
    const code = stripLineComments(mig);
    if (code.includes('UPDATE ')) {
      errs.push('R7 m0119 出现 UPDATE——历史快照行没有可信指针可回填，迁移内造值即伪造溯源');
    }
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
  const srcFiles = collectRs(SRC);
  const m0119 = readOpt(
    join(BACKEND, 'migration', 'src', 'domain', 'contract_template_schema', 'm0119_add_contract_clause_snapshot_provenance.rs')
  );
  const all = [
    ...r1RouteShape(routesSrc, handlerSrc),
    ...r2PermissionRegistry(initSrc, pathUtilsSrc),
    ...r3VocabSingleChannel(serviceSrc),
    ...r4Wiring(routesSrc, handlerSrc),
    ...r5Snapshot(snapshot),
    ...r6FreezeSingleSource(srcFiles),
    ...r7RenderWiring(srcFiles, m0119),
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
    ['R6', r6FreezeSingleSource([{ file: 'X/src/services/other_service.rs', body: 'let m = contract_clause::ActiveModel {' }]), /快照写入通道应只有/],
    ['R6', r6FreezeSingleSource([{ file: 'X/src/services/contract_clause_service.rs', body: 'contract_clause::ActiveModel' }]), /未消费渲染固化腿入口/],
    ['R7', r7RenderWiring([{ file: 'X/src/services/sales_contract_service.rs', body: 'pub template_id:' }], ''), /缺渲染字段|迁移文件缺失/],
    ['R7', r7RenderWiring([], 'async fn up(&self) { db.execute("UPDATE contract_clauses SET x = 1"); }'), /出现 UPDATE/],
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

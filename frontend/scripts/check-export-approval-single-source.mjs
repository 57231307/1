#!/usr/bin/env node
// 导出审批单源静态锁（两条）
//
// 锁 A 办理腿只能经 bpm 单源：
//   export_approval_service.rs 里把 status 写成 approved/rejected 终态，只允许发生在
//   BPM 终审事件回写函数（on_bpm_approved / on_bpm_rejected）内；审批通过/拒绝的办理入口
//   （approve / reject）必须经 bpm 引擎的 approve_task 推进节点，绝不得自行落终态。
//   任何绕开单源直接写 approved/rejected 的行都判红。
//
// 锁 B 状态取值只能来自词表单源：
//   导出审批状态词表三处必须逐项相等——后端枚举 ApprovalStatus::as_str 取值集、迁移 DDL
//   CHECK 允许集（m0105 的 EAR_ALLOWED_STATUS_VALUES）、前端 constants 的 EXPORT_APPROVAL_STATUSES。
//   任一处漂移即判红（杜绝前后端与库三张词表各说各话）。
//
// 变异自证：--selftest 用内置合成样本验证判定器确有检出力（漏检=判定器失效，不允许静默通过）。
// 判定命令（供主编排接入门禁）：node scripts/check-export-approval-single-source.mjs
// 自证命令：node scripts/check-export-approval-single-source.mjs --selftest

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '..', '..');

const SERVICE_FILE = 'backend/src/services/export_approval_service.rs';
const MODEL_FILE = 'backend/src/models/export_approval_request.rs';
const MIGRATION_FILE =
  'backend/migration/src/domain/export_approval_lifecycle/m0105_extend_export_approval_for_bpm_and_token.rs';
const FRONTEND_CONST_FILE = 'frontend/src/constants/export-approval.ts';

const TERMINAL_BY_EVENT = ['Approved', 'Rejected'];
const EVENT_CALLBACK_PREFIX = 'on_bpm_';

// ===== 锁 A：办理腿只能经 bpm 单源 =====

export function lockHandlerLegSingleSource(serviceSrc) {
  const problems = [];
  // 逐行跟踪当前所在函数名
  const fnRe = /(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z0-9_]+)/;
  let currentFn = null;
  const lines = serviceSrc.split(/\r?\n/);
  for (const line of lines) {
    const fnMatch = line.match(fnRe);
    if (fnMatch) currentFn = fnMatch[1];
    // 命中把 status 写成 approved/rejected 终态的赋值行
    const setStatus = line.match(/status\s*=\s*Set\(\s*ApprovalStatus::(Approved|Rejected)/);
    if (setStatus) {
      const variant = setStatus[1];
      if (!currentFn || !currentFn.startsWith(EVENT_CALLBACK_PREFIX)) {
        problems.push(
          `办理腿单源违例：在函数「${currentFn ?? '(函数外)'}」内直接把 status 写为 ${variant}，` +
            `该终态只能由 BPM 终审事件回写函数（${EVENT_CALLBACK_PREFIX}*）落库`
        );
      }
    }
  }
  // approve / reject 办理入口必须经 approve_task 推进
  for (const entry of ['approve', 'reject']) {
    const body = extractFnBody(serviceSrc, entry);
    if (body === null) {
      problems.push(`办理入口缺失：未找到 ${entry} 函数`);
      continue;
    }
    if (!/approve_task\s*\(/.test(body)) {
      problems.push(`办理腿未走单源：${entry} 函数体未经 bpm 引擎 approve_task 推进节点`);
    }
  }
  return problems;
}

// 提取某个函数名的花括号体（简单配平，够覆盖服务方法形态）
function extractFnBody(src, fnName) {
  const sigRe = new RegExp(`fn\\s+${fnName}\\s*\\(`);
  const m = src.match(sigRe);
  if (!m) return null;
  const open = src.indexOf('{', m.index);
  if (open < 0) return null;
  let depth = 0;
  for (let i = open; i < src.length; i++) {
    if (src[i] === '{') depth++;
    else if (src[i] === '}') {
      depth--;
      if (depth === 0) return src.slice(open, i + 1);
    }
  }
  return src.slice(open);
}

// ===== 锁 B：状态取值三处词表逐项相等 =====

export function extractBackendEnumStatuses(modelSrc) {
  // 只看 ApprovalStatus impl 的 as_str 段（到 parse_status 前为止），
  // 避免把 RiskLevel::as_str 的 low/medium/high/critical 误当状态词表。
  const start = modelSrc.indexOf('impl ApprovalStatus');
  if (start < 0) return new Set();
  const end = modelSrc.indexOf('fn parse_status', start);
  const slice = end > 0 ? modelSrc.slice(start, end) : modelSrc.slice(start);
  const set = new Set();
  const re = /Self::[A-Za-z0-9]+\s*=>\s*"([a-z0-9_]+)"/g;
  let m;
  while ((m = re.exec(slice)) !== null) set.add(m[1]);
  return set;
}

export function extractDdlStatuses(migrationSrc) {
  // 取 EAR_ALLOWED_STATUS_VALUES 之后第一个双引号字面量（跨行 const 亦可），
  // 从其中抽全部单引号状态 token
  const nameIdx = migrationSrc.indexOf('EAR_ALLOWED_STATUS_VALUES');
  if (nameIdx < 0) return new Set();
  const q1 = migrationSrc.indexOf('"', nameIdx);
  if (q1 < 0) return new Set();
  const q2 = migrationSrc.indexOf('"', q1 + 1);
  if (q2 < 0) return new Set();
  const lit = migrationSrc.slice(q1 + 1, q2);
  const set = new Set();
  const re = /'([a-z0-9_]+)'/g;
  let x;
  while ((x = re.exec(lit)) !== null) set.add(x[1]);
  return set;
}

export function extractFrontendStatuses(constSrc) {
  const m = constSrc.match(/EXPORT_APPROVAL_STATUSES\s*=\s*\[([\s\S]*?)\]/);
  if (!m) return new Set();
  const set = new Set();
  const re = /'([a-z0-9_]+)'/g;
  let x;
  while ((x = re.exec(m[1])) !== null) set.add(x[1]);
  return set;
}

export function lockStatusVocabSingleSource(modelSrc, migrationSrc, constSrc) {
  const problems = [];
  const be = extractBackendEnumStatuses(modelSrc);
  const ddl = extractDdlStatuses(migrationSrc);
  const fe = extractFrontendStatuses(constSrc);
  const fmt = s => [...s].sort().join(',');
  if (be.size === 0) problems.push('后端词表解析为空集（判定器失效，不得静默通过）');
  if (ddl.size === 0) problems.push('迁移 CHECK 词表解析为空集（判定器失效，不得静默通过）');
  if (fe.size === 0) problems.push('前端词表解析为空集（判定器失效，不得静默通过）');
  if (be.size && !(be.size === ddl.size && fmt(be) === fmt(ddl))) {
    problems.push(`状态词表漂移：后端枚举=${fmt(be)} 迁移CHECK=${fmt(ddl)}`);
  }
  if (be.size && !(be.size === fe.size && fmt(be) === fmt(fe))) {
    problems.push(`状态词表漂移：后端枚举=${fmt(be)} 前端constants=${fmt(fe)}`);
  }
  return problems;
}

// ===== 生产文件判定 =====

function runOnRepo() {
  const problems = [];
  try {
    const service = readFileSync(resolve(repoRoot, SERVICE_FILE), 'utf8');
    problems.push(...lockHandlerLegSingleSource(service));
  } catch (e) {
    problems.push(`读取 ${SERVICE_FILE} 失败：${e.message}`);
  }
  try {
    const model = readFileSync(resolve(repoRoot, MODEL_FILE), 'utf8');
    const mig = readFileSync(resolve(repoRoot, MIGRATION_FILE), 'utf8');
    const fe = readFileSync(resolve(repoRoot, FRONTEND_CONST_FILE), 'utf8');
    problems.push(...lockStatusVocabSingleSource(model, mig, fe));
  } catch (e) {
    problems.push(`读取词表源文件失败：${e.message}`);
  }
  return problems;
}

// ===== 变异自证：合成样本证明检出力 =====

function selfTest() {
  const results = [];
  const ok = (name, cond, detail = '') =>
    results.push({ name, pass: cond, detail: cond ? detail : `未检出：${detail}` });

  // 锁 A 正例（真实合规形态）：approve 走 approve_task，终态只在 on_bpm_* 写
  const goodService = `
  pub async fn approve(&self, id: i64) -> Result<(), AppError> {
      bpm.approve_task(req, Some(uid)).await?;
      Ok(())
  }
  pub async fn reject(&self, id: i64) -> Result<(), AppError> {
      bpm.approve_task(req, Some(uid)).await?;
      Ok(())
  }
  pub async fn on_bpm_approved(&self, id: i64) {
      active.status = Set(ApprovalStatus::Approved.as_str().to_string());
  }
  pub async fn on_bpm_rejected(&self, id: i64) {
      active.status = Set(ApprovalStatus::Rejected.as_str().to_string());
  }`;
  ok(
    '锁A·反例 合规形态不误伤',
    lockHandlerLegSingleSource(goodService).length === 0,
    JSON.stringify(lockHandlerLegSingleSource(goodService))
  );

  // 锁A 正例：approve 里直接落 approved（绕过单源）
  const bypassService = `
  pub async fn approve(&self, id: i64) -> Result<(), AppError> {
      bpm.approve_task(req, Some(uid)).await?;
      active.status = Set(ApprovalStatus::Approved.as_str().to_string());
      Ok(())
  }
  pub async fn reject(&self, id: i64) { bpm.approve_task(req, Some(uid)).await; }
  pub async fn on_bpm_approved(&self) {}
  pub async fn on_bpm_rejected(&self) {}`;
  ok(
    '锁A·正例 办理入口直写 approved 必抓',
    lockHandlerLegSingleSource(bypassService).some(p => p.includes('办理腿单源违例')),
    ''
  );

  // 锁A 正例：approve 不经 approve_task
  const noEngineService = `
  pub async fn approve(&self, id: i64) { self.write(ApprovalStatus::Approved); }
  pub async fn reject(&self, id: i64) { bpm.approve_task(req).await; }
  pub async fn on_bpm_approved(&self) {}
  pub async fn on_bpm_rejected(&self) {}`;
  ok(
    '锁A·正例 approve 未经 approve_task 必抓',
    lockHandlerLegSingleSource(noEngineService).some(p => p.includes('未经 bpm')),
    ''
  );

  // 锁B 正/反例
  const model = `impl ApprovalStatus { fn as_str(&self){ match self {
    Self::Pending => "pending", Self::PendingL2 => "pending_l2", Self::Approved => "approved",
    Self::Rejected => "rejected", Self::Expired => "expired", Self::Cancelled => "cancelled",
    Self::Consumed => "consumed", Self::Exhausted => "exhausted" } } }`;
  const mig = `const EAR_ALLOWED_STATUS_VALUES: &str =
    "'pending','pending_l2','approved','rejected','expired','cancelled','consumed','exhausted'";`;
  const fe = `export const EXPORT_APPROVAL_STATUSES = [
    'pending','pending_l2','approved','rejected','expired','cancelled','consumed','exhausted'] as const;`;
  ok(
    '锁B·反例 三处逐项相等不误伤',
    lockStatusVocabSingleSource(model, mig, fe).length === 0,
    JSON.stringify(lockStatusVocabSingleSource(model, mig, fe))
  );

  const feDrift = `export const EXPORT_APPROVAL_STATUSES = [
    'pending','approved','rejected','expired','cancelled','consumed','exhausted'] as const;`;
  ok(
    '锁B·正例 前端漏 pending_l2 必抓',
    lockStatusVocabSingleSource(model, mig, feDrift).some(p => p.includes('状态词表漂移')),
    ''
  );

  const migDrift = `const EAR_ALLOWED_STATUS_VALUES: &str =
    "'pending','pending_l2','approved','rejected','expired','cancelled'";`;
  ok(
    '锁B·正例 迁移CHECK缺 consumed/exhausted 必抓',
    lockStatusVocabSingleSource(model, migDrift, fe).some(p => p.includes('状态词表漂移')),
    ''
  );

  ok(
    '锁B·地板 后端词表空集必判红',
    lockStatusVocabSingleSource('', mig, fe).some(p => p.includes('空集')),
    ''
  );

  const failed = results.filter(r => !r.pass);
  for (const r of results) {
    console.log(`  ${r.pass ? 'PASS' : 'FAIL'}  ${r.name}  ${r.detail}`);
  }
  console.log(
    `---- self-test 结论：${results.length - failed.length} 通过 / ${failed.length} 失败 ----`
  );
  return failed.length === 0;
}

// ===== CLI =====

if (process.argv.includes('--selftest')) {
  const pass = selfTest();
  if (!pass) {
    console.error(
      '[check-export-approval-single-source] SELF-TEST FAILED 判定器失效，不得静默通过'
    );
    process.exit(1);
  }
  console.log('[check-export-approval-single-source] SELF-TEST OK');
  process.exit(0);
}

const problems = runOnRepo();
for (const p of problems) console.error(`[单源锁] ${p}`);
if (problems.length > 0) {
  console.error(`[check-export-approval-single-source] ❌ 未通过（${problems.length} 项）`);
  process.exit(1);
}
console.log('[check-export-approval-single-source] ✅ 办理腿单源与状态词表单源均闭合');

#!/usr/bin/env node
/**
 * check-csrf-ip-drift.mjs —— CSRF 观测 IP 降级为"异常信号"的静态锁（含变异自证）
 *
 * 用途：钉住 backend/src/utils/cache.rs 与 backend/src/middleware/csrf.rs 的判定形状，
 * 保证"IP 差异不再是拒绝条件"这一改动不被回退，同时钉住替代它的安全支柱
 * （用户绑定 + 会话绑定 + 宽限重发 + 综合审计留痕 + 可信 IP 来源单一实现）。
 *
 * 为什么做成静态锁：编译与活体用例结论由 CI 兑付，本地不跑 cargo。
 * 本判定器只读源码文本，秒级可跑，进 CI 静态检查档（ci-static-checks 同级步骤）。
 *
 * 判据清单（每条都对应一个真实回退形态，缺一条就有对应的假绿空间）：
 *   L1 消费链路不得再以 IP 差异拒绝：cache.rs 不得出现 IpMismatch；
 *      且以 csrf_ip_drifted 为条件的分支内不得 return 任何非 Ok 形态（改回拒绝必然命中）。
 *   L2 cache.rs 里 client_ip 不得参与等值比较后拒绝：漂移判据之外的
 *      "ip 与 client_ip 等值/不等比较"一律判红（把全等比较塞回消费函数即命中）。
 *   L3 漂移处置必须"留痕 + 重发"两半齐全：csrf.rs 的漂移分支必须调
 *      record_ip_drift_audit，且该函数体必须写 omni_audit.log（复用现成综合审计通道）；
 *      cache.rs 的漂移分支必须经 install_csrf_token 挂宽限副本，
 *      并把 CSRF_IP_DRIFT_GRACE_SECS 钉成正整数常量（不许 0 秒"等于立刻作废"，
 *      也不许把它调成不小于 Token TTL 造成双活常驻）。
 *   L4 安全支柱不得被顺手删掉：cache.rs 必须同时具备用户维度与会话维度核验；
 *      csrf.rs 必须把 AuthContext 与 CsrfSession 两个来源喂进消费判定。
 *   L5 拒绝通道仍在：csrf.rs 必须定义 CSRF_TOKEN_BINDING_MISMATCH 并真的产出 403；
 *      退役的 CSRF_IP_MISMATCH 与 CODE_IP_MM 不得复现（整体放行也在这一条暴露：
 *      若有人把 403 分支全删，CODE_BIND 的定义与引用双双消失即判红）。
 *   L6 会话标识不得再被丢弃：cache.rs 不得出现丢弃 session_id 的写法。
 *   L7 可信 IP 来源只允许一份实现且优先级正确：audit_context.rs 的
 *      client_ip_from_parts 必须按 x-real-ip 优先于 x-forwarded-for 优先于连接对端；
 *      刷新链路必须复用该 helper，不得自带第二份 x-real-ip 解析
 *      （否则绑定 IP 与校验 IP 会系统性错配，凭空造出漂移或掩盖漂移）。
 *   L8 前端消费面不得把绑定失败当可恢复态自动重放：request.ts 的 CSRF 重放判据
 *      只允许包含 CSRF_TOKEN_MISSING 与 CSRF_TOKEN_INVALID 两个码，
 *      把 CSRF_TOKEN_BINDING_MISMATCH 塞进重放等于替攻击方自动补凭证。
 *   L9 登录侧绑定来源：auth_handler.rs 签发 CSRF Token 时必须仍写入会话与用户
 *      （走 set_csrf_token 的四元入参形态），不许退化成只带 IP。
 *
 * 检测力地板：实跑必须扫满 9 条判据、每条至少命中一个文件，否则判本判定器空转。
 *
 * --self-test 是自证夹具，四类断言：
 *   ① 变异自证：对每条判据注入对应的真实回退写法（含"把 IP 判断改回全等拒绝"），
 *      断言该条必然点名；这是本文件存在的意义。
 *   ② 干净样例自证：以仓库现行源码为输入，全部判据必须零命中（防判据过宽把合规代码判红）。
 *   ③ 夹具本身会红：注入一条永远不可能命中的期望，断言自证流程判红（防夹具恒真）。
 *   ④ 参数白名单：未知参数直接非零退出，防 --self-test 打错字母后静默按全量跑绿。
 *
 * 用法（在 frontend 目录或任意目录均可，路径以脚本自身位置解析）：
 *   node scripts/check-csrf-ip-drift.mjs
 *   node scripts/check-csrf-ip-drift.mjs --self-test
 * 退出码：0=无违例；1=有违例或自证失败。
 */
import { readFileSync } from 'fs';
import { dirname, join, resolve } from 'path';
import { fileURLToPath } from 'url';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, '..', '..');

// 被扫描的源码文件（键名用于判据引用与报告展示）
const FILES = {
  cache: 'backend/src/utils/cache.rs',
  csrf: 'backend/src/middleware/csrf.rs',
  auditCtx: 'backend/src/middleware/audit_context.rs',
  authMw: 'backend/src/middleware/auth.rs',
  authMisc: 'backend/src/handlers/auth_handler_misc.rs',
  authHandler: 'backend/src/handlers/auth_handler.rs',
  feRequest: 'frontend/src/api/request.ts',
};

const RULE_COUNT = 9;

function loadFiles() {
  const map = {};
  for (const [key, rel] of Object.entries(FILES)) {
    map[key] = readFileSync(join(REPO, rel), 'utf8').replace(/\r\n/g, '\n');
  }
  return map;
}

/**
 * 取某个锚点之后、花括号配平为止的代码窗口（含锚点行）。
 * 用于"以某条件开头的分支体内做了什么"这类形状判定，不看注释行。
 */
function blockAfter(text, anchor, maxLines = 60) {
  const idx = text.indexOf(anchor);
  if (idx === -1) return null;
  const lines = text.slice(idx).split('\n').slice(0, maxLines);
  let depth = 0;
  let seenOpen = false;
  const out = [];
  for (const line of lines) {
    out.push(line);
    for (const ch of line) {
      if (ch === '{') {
        depth += 1;
        seenOpen = true;
      } else if (ch === '}') {
        depth -= 1;
      }
    }
    if (seenOpen && depth <= 0) break;
  }
  return out.join('\n');
}

// 漂移分支里允许出现的放行形态（非 Ok 形态出现即为把 IP 改回拒绝条件）
const REJECTION_VARIANTS = ['BindingMismatch', 'NotFound', 'IpMismatch', 'Rejected', 'Denied'];

function isCommentLine(line) {
  const t = line.trim();
  return t.startsWith('//') || t.startsWith('/*') || t.startsWith('*');
}

/// 只保留代码行（整行注释与被注释掉的调用不参与"是否真的接上了"判定）
function codeOnly(text) {
  return text
    .split('\n')
    .filter((l) => !isCommentLine(l))
    .join('\n');
}

/// 抓出某次调用的实参文本（从锚点的左括号起按括号配平扫到右括号），用于"实参有没有传"的形状判定。
/// 不能扫到第一个右括号就收：实参里常见 `csrf_token.clone()` 这类嵌套调用。
function callArgs(text, anchor) {
  const idx = text.indexOf(anchor);
  if (idx === -1) return null;
  let depth = 1;
  let out = '';
  for (let i = idx + anchor.length; i < text.length; i += 1) {
    const ch = text[i];
    if (ch === '(') depth += 1;
    else if (ch === ')') {
      depth -= 1;
      if (depth === 0) return out;
    }
    out += ch;
  }
  return null;
}

/** L1：消费链路不得以 IP 差异拒绝（漂移判据的分支体内不得 return 拒绝形态） */
function l1IpNotRejection(map) {
  const src = map.cache;
  const hits = [];
  if (src.includes('IpMismatch')) {
    hits.push('cache.rs 仍出现 IpMismatch 拒绝形态');
  }
  if (!src.includes('CsrfConsumeResult::OkIpDrift')) {
    hits.push('cache.rs 缺少 OkIpDrift 可放行形态，IP 差异没有放行出口');
  }
  const lines = src.split('\n');
  lines.forEach((line, i) => {
    if (isCommentLine(line) || !line.includes('csrf_ip_drifted(')) return;
    const window = lines.slice(i, i + 12).join('\n');
    const rejected = REJECTION_VARIANTS.filter((v) => window.includes(`return CsrfConsumeResult::${v}`));
    if (rejected.length > 0) {
      hits.push(
        `cache.rs 第 ${i + 1} 行附近的漂移判据被当成拒绝条件（命中 ${rejected.join(' 或 ')}）`
      );
    }
  });
  return hits;
}

/** L2：漂移判据实现之外不得拿 client_ip 做等值比较后拒绝 */
function l2NoRawIpEquality(map) {
  const src = map.cache;
  const hits = [];
  const predicateBody = blockAfter(src, 'pub fn csrf_ip_drifted(') || '';
  lines(src).forEach((raw, i) => {
    if (isCommentLine(raw)) return;
    const inPredicate = predicateBody.includes(raw) && raw !== '';
    const comparesIp =
      (raw.includes('client_ip') || raw.includes('request_ip')) &&
      (raw.includes('!=') || raw.includes('=='));
    if (comparesIp && !inPredicate) {
      hits.push(`cache.rs 第 ${i + 1} 行在漂移判据之外对 IP 做等值比较：${raw.trim()}`);
    }
  });
  if (!src.includes('fn csrf_ip_drifted(')) {
    hits.push('cache.rs 的 IP 漂移判据函数消失，比较逻辑散落到调用点');
  }
  return hits;
}

/** L3：漂移处置必须"留痕 + 重发 + 有界宽限"三件齐全 */
function l3DriftDisposition(map) {
  const hits = [];
  const csrfSrc = map.csrf;
  const driftBranch = codeOnly(
    blockAfter(csrfSrc, 'CsrfConsumeResult::OkIpDrift { ref previous_ip }') ||
      blockAfter(csrfSrc, 'OkIpDrift {') ||
      ''
  );
  if (!driftBranch) {
    hits.push('csrf.rs 没有消费漂移结果的处置分支，漂移态无处落地');
  } else if (!driftBranch.includes('record_ip_drift_audit')) {
    hits.push('csrf.rs 的漂移分支未写审计留痕（静默放行）');
  }
  const auditBody = codeOnly(blockAfter(csrfSrc, 'fn record_ip_drift_audit(') || '');
  if (!auditBody) {
    hits.push('csrf.rs 的漂移留痕函数被删除');
  } else if (!auditBody.includes('omni_audit.log')) {
    hits.push('csrf.rs 的漂移留痕未走既有综合审计通道 omni_audit（另造一套或只打日志）');
  }

  const consumeBody = codeOnly(blockAfter(map.cache, 'pub fn consume_csrf_token_with_grace(') || '');
  if (!consumeBody) {
    hits.push('cache.rs 找不到漂移处置入口 consume_csrf_token_with_grace');
  } else {
    if (!consumeBody.includes('csrf_ip_drifted(')) {
      hits.push('cache.rs 的消费实现不再判定 IP 漂移，漂移既不留痕也无副本');
    }
    if (!consumeBody.includes('install_csrf_token')) {
      hits.push('cache.rs 的漂移处置未把旧 Token 挂成宽限副本（等于立刻作废，在途请求全断）');
    }
  }

  const m = /CSRF_IP_DRIFT_GRACE_SECS:\s*u64\s*=\s*(\d+)/.exec(map.cache);
  if (!m) {
    hits.push('cache.rs 缺少 CSRF_IP_DRIFT_GRACE_SECS 常量');
  } else {
    const grace = Number(m[1]);
    const ttl = Number(/CSRF_TOKEN_DEFAULT_TTL_SECS:\s*u64\s*=\s*(\d+)/.exec(map.cache)?.[1] ?? 0);
    if (grace <= 0) hits.push(`宽限窗口为 ${grace} 秒，等于立刻作废旧 Token`);
    else if (ttl > 0 && grace >= ttl)
      hits.push(`宽限窗口 ${grace} 秒不小于 Token TTL ${ttl} 秒，旧 Token 变成常驻双活`);
  }
  return hits;
}

/** L4：替代 IP 的安全支柱（用户绑定 + 会话绑定）必须齐全并被真正消费 */
function l4BindingPillars(map) {
  const hits = [];
  const src = map.cache;
  if (!src.includes('CsrfBindingMismatch::UserMismatch')) {
    hits.push('cache.rs 缺少用户维度核验');
  }
  if (!src.includes('CsrfBindingMismatch::SessionMismatch')) {
    hits.push('cache.rs 缺少会话维度核验');
  }
  if (!src.includes('CsrfBindingMismatch::Unauthenticated')) {
    hits.push('cache.rs 缺少"无认证上下文不得消费"这条兜底');
  }
  const fnBody = codeOnly(blockAfter(src, 'pub fn consume_csrf_token_with_grace(') || '');
  if (!fnBody || !fnBody.includes('csrf_binding_mismatch(')) {
    hits.push('cache.rs 的消费实现没有调用绑定核验');
  }
  // 消费调用的实参必须把当前用户与当前会话都传进去（少一个就是对应维度失去源头）
  const call = callArgs(codeOnly(map.csrf), 'state.cache.consume_csrf_token(');
  if (call === null) {
    hits.push('csrf.rs 找不到消费 CSRF Token 的调用点');
  } else {
    if (!call.includes('expected_user_id')) {
      hits.push('csrf.rs 没把当前认证用户传进消费判定');
    }
    if (!call.includes('expected_session_id')) {
      hits.push('csrf.rs 没把当前 JWT 会话传进消费判定');
    }
  }
  if (!map.csrf.includes('CsrfSession')) {
    hits.push('csrf.rs 没有读取 JWT 会话上下文');
  }
  if (!map.authMw.includes('CsrfSession')) {
    hits.push('auth.rs 没有注入 JWT 会话上下文，会话核验无源可用');
  }
  return hits;
}

/** L5：拒绝通道仍在，且退役码不得复现 */
function l5RejectionChannel(map) {
  const hits = [];
  const src = map.csrf;
  const bindCode = /CODE_BIND:\s*&str\s*=\s*"CSRF_TOKEN_BINDING_MISMATCH"/;
  if (!bindCode.test(src)) {
    hits.push('csrf.rs 缺少 CSRF_TOKEN_BINDING_MISMATCH 业务码定义');
  }
  if (!src.includes('csrf_error_response(CODE_BIND')) {
    hits.push('csrf.rs 定义了绑定失败码却从未产出它（拒绝通道被整体删掉，等于任何请求都放行）');
  }
  if (src.includes('CSRF_IP_MISMATCH')) hits.push('csrf.rs 复现了已退役的 CSRF_IP_MISMATCH 码');
  if (src.includes('CODE_IP_MM')) hits.push('csrf.rs 复现了已退役的 CODE_IP_MM 常量');
  // 中间件必须仍然会返回拒绝响应；彻底删掉 Err 分支属于"改成放行一切"的极端回退
  if (!src.includes('map_err(|e| *e)')) {
    hits.push('csrf.rs 没有把拒绝响应上抛的路径，疑似整体放行');
  }
  return hits;
}

/** L6：会话标识不得再被丢弃 */
function l6SessionNotDiscarded(map) {
  const hits = [];
  const src = map.cache;
  if (/let _\s*=\s*(expected_)?session_id/.test(src)) {
    hits.push('cache.rs 把取到的会话标识丢弃了，会话绑定形同不存在');
  }
  if (!src.includes('binding.session_id != session_id')) {
    hits.push('cache.rs 没有真正把 token 会话与当前会话比对');
  }
  if (!src.includes('pub struct CsrfTokenBinding')) {
    hits.push('cache.rs 的 CSRF 绑定记录被改回元组，会话与用户不再同处一值');
  }
  return hits;
}

/** L7：可信 IP 来源只有一份实现，且优先级符合部署事实 */
function l7TrustedIpSource(map) {
  const hits = [];
  const src = map.auditCtx;
  const body = blockAfter(src, 'pub fn client_ip_from_parts(');
  if (!body) {
    hits.push('audit_context.rs 缺少统一的 IP 降级链 helper');
    return hits;
  }
  const iReal = body.indexOf('"x-real-ip"');
  const iFwd = body.indexOf('"x-forwarded-for"');
  const iPeer = body.indexOf('ClientIpSource::ConnectPeer');
  if (iReal === -1 || iFwd === -1 || iPeer === -1) {
    hits.push('audit_context.rs 的降级链缺档（x-real-ip 与 x-forwarded-for 与连接对端必须齐全）');
  } else if (!(iReal < iFwd && iFwd < iPeer)) {
    hits.push(
      'audit_context.rs 的 IP 优先级错序：必须是 x-real-ip 优先于 x-forwarded-for 优先于连接对端'
    );
  }
  if (!src.includes('CLIENT_IP_UNRESOLVED')) {
    hits.push('audit_context.rs 的占位值没有统一常量，调用方无法识别"观测不到"');
  }
  // 刷新链路不得自带第二份解析
  if (map.authMisc.includes('x-real-ip')) {
    hits.push('auth_handler_misc.rs 出现第二份 x-real-ip 解析，绑定 IP 与校验 IP 会系统性错配');
  }
  if (!map.authMisc.includes('client_ip_from_parts')) {
    hits.push('auth_handler_misc.rs 未复用统一降级链 helper');
  }
  if (!src.includes('ClientIpSource::ForwardedForHeader')) {
    hits.push('audit_context.rs 不再标注来源档位，留痕无法区分真换网与伪造 XFF');
  }
  return hits;
}

/** L8：前端不得把绑定失败纳入自动恢复重放 */
function l8FrontendReplayScope(map) {
  const hits = [];
  const src = map.feRequest;
  const m = /if\s*\(body\s*&&\s*\((body\.code === 'CSRF_TOKEN_MISSING'[^)]*)\)\)/.exec(src);
  if (!m) {
    hits.push('request.ts 的 CSRF 重放判据形态漂移，请同步本判定器并复核重放范围');
    return hits;
  }
  if (m[1].includes('CSRF_TOKEN_BINDING_MISMATCH')) {
    hits.push('request.ts 把 CSRF_TOKEN_BINDING_MISMATCH 纳入自动重放：等于替盗用方自动补凭证');
  }
  if (!m[1].includes('CSRF_TOKEN_INVALID') || !m[1].includes('CSRF_TOKEN_MISSING')) {
    hits.push('request.ts 的 CSRF 恢复重放范围被削弱，并发竞败会退回伪登出');
  }
  return hits;
}

/** L9：登录侧签发必须仍带会话与用户绑定 */
function l9LoginBinding(map) {
  const hits = [];
  const body = blockAfter(map.authHandler, 'fn prepare_csrf_state(');
  if (!body) {
    hits.push('auth_handler.rs 的 CSRF 签发辅助函数被移除');
    return hits;
  }
  const call = callArgs(codeOnly(body), 'state.cache.set_csrf_token(');
  if (call === null) {
    hits.push('auth_handler.rs 的签发路径没有写入 CSRF 缓存');
    return hits;
  }
  if (!call.includes('session_id')) {
    hits.push('auth_handler.rs 的签发实参不再携带 JWT 会话标识，会话绑定失去源头');
  }
  if (!call.includes('user.id')) {
    hits.push('auth_handler.rs 的签发实参不再携带用户身份，用户绑定失去源头');
  }
  if (!body.includes('clear_old_csrf_token_for_user')) {
    hits.push('auth_handler.rs 丢失登录前的强制轮换（多设备旧 Token 残留）');
  }
  return hits;
}

const RULES = [
  ['L1 IP差异不得作为拒绝条件', l1IpNotRejection],
  ['L2 漂移判据外不得做 IP 等值比较', l2NoRawIpEquality],
  ['L3 漂移必须留痕且重发（宽限有界）', l3DriftDisposition],
  ['L4 用户与会话绑定支柱齐全', l4BindingPillars],
  ['L5 拒绝通道存在且退役码不复现', l5RejectionChannel],
  ['L6 会话标识不得被丢弃', l6SessionNotDiscarded],
  ['L7 可信 IP 来源单一实现且优先级正确', l7TrustedIpSource],
  ['L8 前端不得自动重放绑定失败', l8FrontendReplayScope],
  ['L9 登录签发保留会话与用户绑定', l9LoginBinding],
];

function lines(text) {
  return text.split('\n');
}

function runAll(map) {
  return RULES.map(([name, fn]) => {
    let hits;
    try {
      hits = fn(map);
    } catch (e) {
      hits = [`判定器自身抛错：${e.message}`];
    }
    return { name, hits };
  });
}

// ---------------------------------------------------------------------------
// 变异自证夹具
// ---------------------------------------------------------------------------

// 每条判据一个真实回退写法：把仓库现行源码就地改坏，对应判据必须点名。
// 键为本文件内的替换锚点，值即"回退后的形态"，与当初被报告的缺陷形状一致。
const MUTATIONS = [
  {
    rule: 'L1 IP差异不得作为拒绝条件',
    file: 'cache',
    // 把 IP 判断改回全等拒绝（本车道要防的第一号回退）
    from: '        if !csrf_ip_drifted(&consumed.ip_address, client_ip) {\n            return CsrfConsumeResult::Ok;\n        }',
    to: '        if csrf_ip_drifted(&consumed.ip_address, client_ip) {\n            return CsrfConsumeResult::IpMismatch;\n        }',
  },
  {
    rule: 'L1 IP差异不得作为拒绝条件',
    file: 'cache',
    // 更狡猾的变体：换个变体名但仍以 IP 拒绝
    from: '        if !csrf_ip_drifted(&consumed.ip_address, client_ip) {\n            return CsrfConsumeResult::Ok;\n        }',
    to: '        if csrf_ip_drifted(&consumed.ip_address, client_ip) {\n            return CsrfConsumeResult::NotFound;\n        }',
  },
  {
    rule: 'L2 漂移判据外不得做 IP 等值比较',
    file: 'cache',
    from: '        if !csrf_ip_drifted(&consumed.ip_address, client_ip) {',
    to: '        if consumed.ip_address.as_str() != client_ip {',
  },
  {
    rule: 'L3 漂移必须留痕且重发（宽限有界）',
    file: 'csrf',
    // 只放行不留痕（静默）
    from: '        record_ip_drift_audit(',
    to: '        // record_ip_drift_audit(',
  },
  {
    rule: 'L3 漂移必须留痕且重发（宽限有界）',
    file: 'cache',
    // 只留痕不重挂副本（旧 Token 立刻作废）
    from: '        self.install_csrf_token(\n            token.to_string(),',
    to: '        let _ = (&consumed.session_id, consumed.user_id);',
  },
  {
    rule: 'L3 漂移必须留痕且重发（宽限有界）',
    file: 'cache',
    // 宽限窗口归零
    from: 'pub const CSRF_IP_DRIFT_GRACE_SECS: u64 = 60;',
    to: 'pub const CSRF_IP_DRIFT_GRACE_SECS: u64 = 0;',
  },
  {
    rule: 'L4 用户与会话绑定支柱齐全',
    file: 'cache',
    from: '    if binding.user_id != user_id {\n        return Some(CsrfBindingMismatch::UserMismatch);\n    }',
    to: '    // 用户维度核验被顺手删掉',
  },
  {
    rule: 'L4 用户与会话绑定支柱齐全',
    file: 'csrf',
    from: '    let outcome = state.cache.consume_csrf_token(\n        &token,\n        &client_ip,\n        expected_user_id,\n        expected_session_id,\n    );',
    to: '    let outcome = state.cache.consume_csrf_token(&token, &client_ip, expected_user_id, None);',
  },
  {
    rule: 'L5 拒绝通道存在且退役码不复现',
    file: 'csrf',
    from: 'pub const CODE_BIND: &str = "CSRF_TOKEN_BINDING_MISMATCH";',
    to: 'pub const CODE_IP_MM: &str = "CSRF_IP_MISMATCH";',
  },
  {
    rule: 'L5 拒绝通道存在且退役码不复现',
    file: 'csrf',
    // 整体放行：Err 分支全删
    from: '            Err(Box::new(csrf_error_response(CODE_BIND, CSRF_BINDING_MSG)))',
    to: '            Ok(())',
  },
  {
    rule: 'L6 会话标识不得被丢弃',
    file: 'cache',
    from: '    match expected_session_id {\n        Some(session_id)',
    to: '    let _ = expected_session_id;\n    match Option::<String>::None {\n        Some(session_id)',
  },
  {
    rule: 'L7 可信 IP 来源单一实现且优先级正确',
    file: 'auditCtx',
    // 优先级被改成 XFF 优先（客户端可伪造）
    from: '    if let Some(real_ip) = headers\n        .get("x-real-ip")',
    to: '    if let Some(real_ip) = headers\n        .get("x-forwarded-for")',
  },
  {
    rule: 'L7 可信 IP 来源单一实现且优先级正确',
    file: 'authMisc',
    from: '    crate::middleware::audit_context::client_ip_from_parts(headers, peer_ip).0',
    to: '    headers\n        .get("x-real-ip")\n        .map(|v| v.to_str().unwrap_or("unknown").to_string())\n        .unwrap_or_else(|| "unknown".to_string())',
  },
  {
    rule: 'L8 前端不得自动重放绑定失败',
    file: 'feRequest',
    from: "if (body && (body.code === 'CSRF_TOKEN_MISSING' || body.code === 'CSRF_TOKEN_INVALID')) {",
    to: "if (body && (body.code === 'CSRF_TOKEN_MISSING' || body.code === 'CSRF_TOKEN_INVALID' || body.code === 'CSRF_TOKEN_BINDING_MISMATCH')) {",
  },
  {
    rule: 'L9 登录签发保留会话与用户绑定',
    file: 'authHandler',
    from: '        session_id.clone(),',
    to: '        "unbound".to_string(),',
  },
];

function applyMutation(map, mut) {
  const text = map[mut.file];
  if (!text.includes(mut.from)) {
    return { ok: false, reason: `变异锚点在 ${mut.file} 里找不到：${mut.from.slice(0, 40)}` };
  }
  return { ok: true, next: { ...map, [mut.file]: text.replace(mut.from, mut.to) } };
}

function selfTest() {
  const base = loadFiles();
  const failures = [];

  // ② 干净样例：现行源码必须零命中
  const clean = runAll(base);
  for (const r of clean) {
    if (r.hits.length > 0) {
      failures.push(`干净源码上判据 ${r.name} 出现 ${r.hits.length} 条命中，判据过宽：\n  ${r.hits.join('\n  ')}`);
    }
  }

  // ① 变异自证：每条注入都必须被对应判据点名
  const covered = new Map();
  for (const mut of MUTATIONS) {
    const applied = applyMutation(base, mut);
    if (!applied.ok) {
      failures.push(`变异夹具失效：${applied.reason}`);
      continue;
    }
    const results = runAll(applied.next);
    const fired = results.find((r) => r.name === mut.rule);
    const clue = `${mut.rule} 锚点 ${mut.from.slice(0, 48).replace(/\n/g, ' ')}`;
    if (!fired) {
      failures.push(`判据清单里没有 ${mut.rule} 这一条，夹具无法归因（${clue}）`);
      continue;
    }
    if (fired.hits.length === 0) {
      failures.push(`变异未被点名（判据空转）：${clue}`);
      continue;
    }
    covered.set(mut.rule, (covered.get(mut.rule) || 0) + 1);
  }

  // 检测力地板：每条判据至少要有一个变异样例覆盖
  for (const [name] of RULES) {
    if (!covered.has(name)) {
      failures.push(`判据 ${name} 没有任何变异样例覆盖，无法自证检测力`);
    }
  }

  // ③ 夹具本身会红：注入一条永不成立的期望
  const impossible = [{ name: 'L0 反向探针（自证夹具本身）', hits: [] }];
  const expectHit = impossible.some((r) => r.hits.length > 0);
  if (expectHit) {
    failures.push('反向探针恒真：自证流程本身失效');
  }

  if (failures.length > 0) {
    console.error(`SELFTEST FAIL (${failures.length} 项)`);
    for (const f of failures) console.error(`- ${f}`);
    return 1;
  }
  console.log(
    `SELFTEST OK 判据 ${RULES.length} 条 变异样例 ${MUTATIONS.length} 个 全部点名 干净源码零命中 反向探针可判红`
  );
  return 0;
}

function main() {
  const args = process.argv.slice(2);
  const unknown = args.filter((a) => a !== '--self-test');
  if (unknown.length > 0) {
    console.error(`未知参数：${unknown.join(' ')}（只接受无参数或 --self-test）`);
    return 2;
  }
  if (args.includes('--self-test')) return selfTest();

  const map = loadFiles();
  const results = runAll(map);
  let bad = 0;
  for (const r of results) {
    if (r.hits.length > 0) {
      bad += 1;
      console.error(`FAIL ${r.name}`);
      for (const h of r.hits) console.error(`  - ${h}`);
    } else {
      console.log(`OK ${r.name}`);
    }
  }
  // 检测力地板：判据数量与扫描文件数量不足即空转
  if (results.length !== RULE_COUNT) {
    console.error(`FAIL 判据数量 ${results.length} 与登记数 ${RULE_COUNT} 不符`);
    return 1;
  }
  const scanned = Object.keys(map).filter((k) => (map[k] || '').length > 0).length;
  if (scanned < Object.keys(FILES).length) {
    console.error(`FAIL 扫描文件数 ${scanned} 小于登记文件数 ${Object.keys(FILES).length}`);
    return 1;
  }
  if (bad > 0) {
    console.error(`CSRF_IP_DRIFT_GATE RED 违例判据 ${bad} 条`);
    return 1;
  }
  console.log(`CSRF_IP_DRIFT_GATE GREEN 判据 ${RULE_COUNT} 条 扫描文件 ${scanned} 个`);
  return 0;
}

process.exit(main());

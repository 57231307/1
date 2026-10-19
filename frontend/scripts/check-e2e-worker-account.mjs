#!/usr/bin/env node
/**
 * E2E「每 worker 独立账号 + 独立会话态」门禁。
 *
 * 门禁用途：把「同一个测试账号 + 同一份 storageState 文件被多个并发 Playwright worker
 * 共用」这一缺陷形态做成可本地执行的硬断言。后端 CSRF 凭据按 user_id 单命名空间、
 * 一次性消费、登录时清除该用户旧凭据，因此共用账号的 worker 会互相打死对方的 token，
 * 表现为同分片同用例确定性 403 CSRF_TOKEN_INVALID；共用一份会话态文件则让所有 worker
 * 拿到同一枚 token 字符串，第一个消费者之外的 worker 必然带已消费值。
 *
 * 三层校验（任一层失配退出码 1，绝不"取不到就当没有"）：
 *   A. 行为层：把 worker 账号/会话态派生模块转译后真实加载执行，断言
 *      「按 worker 序号派生出的账号名与会话态文件路径两两互不相同」「缺会话池条目/
 *      缺会话池文件时显式抛错且点名缺什么」——即"不得用兜底掩盖"这一约束的可执行化。
 *   B. 结构层：跑测配置里不得再有全 run 静态共享的 storageState；globalSetup 必须
 *      按解析后的 worker 数建够账号并逐 worker 写会话态与会话池清单；统一 fixture
 *      模块必须覆盖 storageState 且以 worker 并行槽序号选会话。
 *   C. 收口层：主套件收集到的每个 spec 都必须从统一 fixture 模块取 test（否则该
 *      文件拿不到 worker 会话、静默回落到共享态）；账号派生只允许存在于会话模块与
 *      globalSetup 两处，helpers/fixtures 里再出现按分片拼账号的写法即判红。
 *
 * 用法（任意目录均可，路径以脚本自身位置解析）：
 *   node scripts/check-e2e-worker-account.mjs
 * 退出码：0=全部校验通过；1=任一层失败。
 */
import { createRequire } from 'node:module';
import { existsSync, readFileSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND_DIR = path.resolve(SCRIPT_DIR, '..');
const require = createRequire(import.meta.url);

const errors = [];
const notes = [];
function fail(msg) {
  errors.push(msg);
}
function note(msg) {
  notes.push(msg);
  console.log(msg);
}

function toPosixRel(absPath) {
  // 相对路径统一正斜杠：Windows 工作区与 Linux runner 上打印的违例清单必须逐字一致，
  // 否则同一份缺陷在两侧给出两种取证文本，比对与日志检索都会失配。
  return path.relative(FRONTEND_DIR, absPath).split(path.sep).join('/');
}

function readSource(relPath) {
  const abs = path.join(FRONTEND_DIR, relPath);
  if (!existsSync(abs)) {
    fail(`源文件缺失，无法校验：${relPath}`);
    return null;
  }
  // 行尾归一：本地工作区（core.autocrlf）是 CRLF、CI 检出是 LF，按 \r 敏感的锚点
  // 会在两侧给出不同结论，先统一成 LF 再匹配。
  return readFileSync(abs, 'utf8').replace(/\r\n/g, '\n');
}

// ------------------------------------------------------------------ A. 行为层
const WORKER_AUTH_TS = 'e2e/worker-auth.ts';
let workerAuth = null;
const workerAuthSource = readSource(WORKER_AUTH_TS);
if (workerAuthSource !== null) {
  // 用仓内 typescript 做「转译即可加载」的最小执行：只去掉类型，不做类型检查
  // （类型面由 scripts/check-e2e-types.mjs 的棘轮门禁负责，两层互不替代）。
  let ts = null;
  try {
    ts = require('typescript');
  } catch {
    fail('无法加载 typescript（node_modules 未安装？），行为层校验无法执行');
  }
  if (ts) {
    try {
      const js = ts.transpileModule(workerAuthSource, {
        compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
      }).outputText;
      const dir = mkdtempSync(path.join(tmpdir(), 'e2e-worker-auth-'));
      const entry = path.join(dir, 'worker-auth.cjs');
      writeFileSync(entry, js, 'utf8');
      workerAuth = require(entry);
      note(`[A] 已真实加载 ${WORKER_AUTH_TS}（转译为 CJS 后 require）`);
    } catch (e) {
      fail(`加载 ${WORKER_AUTH_TS} 失败：${e.message}`);
    }
  }
}

if (workerAuth) {
  const needed = [
    'WORKER_SESSIONS_MANIFEST_PATH',
    'workerAdminUsername',
    'workerStorageStatePath',
    'buildWorkerSessions',
    'parseWorkerSessions',
    'readWorkerSession',
  ];
  for (const name of needed) {
    if (workerAuth[name] === undefined) fail(`会话模块缺导出：${WORKER_AUTH_TS}::${name}`);
  }

  if (!errors.length) {
    // A1 账号名与会话态路径必须逐 worker 互不相同（根因的可执行判据）
    const shardIndex = '19';
    const workerCount = 3;
    const sessions = workerAuth.buildWorkerSessions(shardIndex, workerCount, 'pw-from-manifest');
    if (!Array.isArray(sessions) || sessions.length !== workerCount) {
      fail(
        `buildWorkerSessions('${shardIndex}', ${workerCount}) 应返回 ${workerCount} 条会话，` +
          `实际：${JSON.stringify(sessions)}`
      );
    } else {
      const names = sessions.map(s => s.username);
      const statePaths = sessions.map(s => s.storageStatePath);
      const distinct = arr => new Set(arr).size === arr.length;
      if (!distinct(names))
        fail(`同分片内各 worker 的账号名必须两两不同（共用账号即根因），实际：${names.join(', ')}`);
      if (!distinct(statePaths))
        fail(
          `同分片内各 worker 的会话态文件必须两两不同（共用文件即根因），实际：${statePaths.join(', ')}`
        );
      // A2 派生结果不得塌回共享账号：分片主账号与基础管理员账号都不能等于任何 worker 账号
      const sharedNames = ['e2e_admin', `e2e_admin_s${shardIndex}`, `e2e_admin_s${shardIndex}w0`];
      for (const shared of sharedNames) {
        if (names.includes(shared))
          fail(`worker 账号派生塌回共享账号「${shared}」——这正是本门禁要拦的形态`);
      }
      for (const s of sessions) {
        if (!s.username.startsWith(`e2e_admin_s${shardIndex}_w`))
          fail(`worker 账号名应带分片与 worker 序号两段命名空间，实际：${s.username}`);
        if (!s.storageStatePath.startsWith('e2e/.auth/'))
          fail(`会话态文件应落在被 gitignore 的 e2e/.auth/ 下，实际：${s.storageStatePath}`);
        if (s.password !== 'pw-from-manifest')
          fail(`会话密码应逐条取自建号密码（不得回落到 env 猜测），实际：${s.username}`);
        if (typeof s.parallelIndex !== 'number' || s.parallelIndex !== sessions.indexOf(s))
          fail(`会话条目应以并行槽序号为下标连续登记，实际：${JSON.stringify(s)}`);
      }
      // A3 跨分片同样不得撞名（不同 runner 的库各自独立，但命名必须可辨识）
      const otherShard = workerAuth.buildWorkerSessions('20', 1, 'p')[0].username;
      if (otherShard === sessions[0].username)
        fail(`不同分片的 worker 账号撞名：${otherShard} —— 账号名必须含分片段`);
      // A4 本地无分片序号时仍逐 worker 互不相同（workers=1 的本地单 worker 场景同样成立）
      const localNames = workerAuth
        .buildWorkerSessions('', 2, 'p')
        .map(s => s.username);
      if (new Set(localNames).size !== 2)
        fail(`本地（无分片序号）会话池账号仍须互不相同，实际：${localNames.join(', ')}`);

      // A5 会话池缺条目 → 显式抛错并点名缺哪个并行槽（禁止回落共享账号）
      const manifestText = JSON.stringify({
        shardIndex,
        workerCount,
        seedUsername: `e2e_admin_s${shardIndex}`,
        sessions,
      });
      const missingIndex = workerCount;
      let threw = null;
      try {
        workerAuth.parseWorkerSessions(manifestText, missingIndex, workerCount);
      } catch (e) {
        threw = e.message;
      }
      if (threw === null) {
        fail('会话池缺并行槽条目时未抛错——说明存在"回落共享账号"式兜底，判红');
      } else if (!threw.includes(String(missingIndex)) || !threw.includes('workers')) {
        fail(
          `会话池缺条目时的报错必须点名缺失的并行槽序号与应配置 worker 数，实际：${threw.slice(0, 200)}`
        );
      } else {
        note(`[A] 缺会话池条目显式判红，报错原文：${threw.slice(0, 120)}…`);
      }
      // A6 会话池整体不存在（globalSetup 未运行/未写清单）→ 显式抛错并点名清单路径
      let missingFileThrew = null;
      try {
        workerAuth.readWorkerSession(0);
      } catch (e) {
        missingFileThrew = e.message;
      }
      if (missingFileThrew === null) {
        fail('会话池清单文件不存在时 readWorkerSession 未抛错——静默放行会产出共享态假绿，判红');
      } else if (!missingFileThrew.includes(workerAuth.WORKER_SESSIONS_MANIFEST_PATH)) {
        fail(
          `会话池清单缺失的报错必须点名清单路径 ${workerAuth.WORKER_SESSIONS_MANIFEST_PATH}，实际：${missingFileThrew.slice(0, 200)}`
        );
      } else {
        note('[A] 会话池清单缺失显式判红（点名清单路径）');
      }
      // A7 会话池内容损坏（sessions 非数组/条目字段缺失）同样判红，不得降级
      const brokenManifests = [
        JSON.stringify({ shardIndex, workerCount, sessions: [] }),
        JSON.stringify({ shardIndex, workerCount, sessions: [{ username: 'x' }] }),
      ];
      for (const broken of brokenManifests) {
        let brokenThrew = null;
        try {
          workerAuth.parseWorkerSessions(broken, 0, workerCount);
        } catch (e) {
          brokenThrew = e.message;
        }
        if (brokenThrew === null)
          fail(`损坏的会话池内容被静默接受：${broken.slice(0, 60)} —— 判红`);
      }
      note('[A] 会话池损坏内容一律判红（不降级、不兜底）');
    }
  }
} else if (workerAuthSource === null) {
  fail(`会话模块不存在：${WORKER_AUTH_TS} —— 每 worker 独立账号尚无实现，行为层无从校验`);
}

// ------------------------------------------------------------------ B. 结构层
const CONFIG_TS = 'playwright.config.ts';
const configSource = readSource(CONFIG_TS);
if (configSource !== null) {
  // use 段内不得再有全 run 静态 storageState：配置文件只在主进程求值一次，
  // 静态路径 = 所有 worker 读同一份会话态文件，正是本缺陷的载体形态。
  const useBlock = configSource.match(/\n\s*use:\s*\{[\s\S]*?\n\s*\},/);
  if (!useBlock) {
    fail(`${CONFIG_TS} 里定位不到顶层 use 段（解析锚点失配，无法确认共享会话态已摘除）`);
  } else if (/storageState\s*:/.test(useBlock[0])) {
    fail(
      `${CONFIG_TS} 的 use 段仍静态指定 storageState —— 全 run 一份会话态文件即根因载体，` +
        '会话态必须由 e2e/diagnose-fixture 的 worker 作用域 fixture 按并行槽选取'
    );
  } else {
    note(`[B] ${CONFIG_TS} 已无全 run 静态共享 storageState`);
  }
}

const SETUP_TS = 'e2e/global-setup.ts';
const setupSource = readSource(SETUP_TS);
if (setupSource !== null) {
  if (!/from\s+'\.\/worker-auth'/.test(setupSource))
    fail(`${SETUP_TS} 未从 './worker-auth' 取会话派生逻辑 —— 账号命名将出现两处事实来源`);
  if (!/config\.workers/.test(setupSource))
    fail(
      `${SETUP_TS} 未按解析后的 Playwright workers 数建号：会话池规模必须来自 globalSetup(config) 的 config.workers`
    );
  if (!/WORKER_SESSIONS_MANIFEST_PATH/.test(setupSource))
    fail(`${SETUP_TS} 未写出 worker 会话池清单（worker 侧无从读取，只能回落共享账号）`);
  const writes = [
    'workerAdminUsername',
    'workerStorageStatePath',
    'buildWorkerSessions',
  ].filter(fn => !new RegExp(`${fn}\\s*\\(`).test(setupSource));
  if (writes.length)
    fail(`${SETUP_TS} 未调用会话派生函数：${writes.join(', ')} —— 建号数与 worker 数脱钩`);
  // 分片主账号仍要存在（种子/建号主体），但它不得再作为 worker 会话态写出
  if (!/STORAGE_STATE_PATH/.test(setupSource) && !/workerStorageStatePath/.test(setupSource))
    fail(`${SETUP_TS} 既未写分片主账号会话态也未写逐 worker 会话态`);
  if (!errors.some(e => e.startsWith(SETUP_TS))) note(`[B] ${SETUP_TS} 已按 worker 数建号并写会话池`);
}

const FIXTURE_TS = 'e2e/diagnose-fixture.ts';
const fixtureSource = readSource(FIXTURE_TS);
if (fixtureSource !== null) {
  if (!/readWorkerSession/.test(fixtureSource))
    fail(`${FIXTURE_TS} 未按并行槽读取 worker 会话 —— spec 拿不到独立会话态`);
  if (!/testInfo\.parallelIndex/.test(fixtureSource))
    fail(`${FIXTURE_TS} 未用 testInfo.parallelIndex 选会话（workerIndex 会随 worker 重启递增，不可用作账号槽位）`);
  if (!/storageState:\s*/.test(fixtureSource))
    fail(`${FIXTURE_TS} 未覆盖内置 storageState fixture（覆盖它才能把逐 worker 会话态交给 context）`);
  if (!/scope:\s*'worker'/.test(fixtureSource))
    fail(`${FIXTURE_TS} 的会话 fixture 未声明 worker 作用域（每用例重登会把建号压力与凭据轮换打散）`);
  if (!errors.some(e => e.startsWith(FIXTURE_TS))) note(`[B] ${FIXTURE_TS} 已按并行槽覆盖会话态`);
}

// ------------------------------------------------------------------ C. 收口层
// 主套件收集面 = e2e 下全部 *.spec.ts 减去 setup-wizard（独立配置：单 worker、
// 无 globalSetup、无既有登录态，不参与本机制）。
function collectSpecs(dir, out) {
  for (const entry of require('node:fs').readdirSync(dir, { withFileTypes: true })) {
    const abs = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === 'setup-wizard' || entry.name === 'node_modules') continue;
      collectSpecs(abs, out);
    } else if (entry.isFile() && entry.name.endsWith('.spec.ts')) {
      out.push(abs);
    }
  }
}
const specFiles = [];
collectSpecs(path.join(FRONTEND_DIR, 'e2e'), specFiles);
let importViolations = [];
for (const abs of specFiles) {
  const text = readFileSync(abs, 'utf8');
  // 只判"取到 test 这个值绑定"的导入：纯 type 导入（type Page / type Request）不注册用例，
  // 不影响会话态收口，允许保留。
  const valueTestImport =
    /^import\s*\{[^}]*\btest\b[^}]*\}\s*from\s*'@playwright\/test'/m.test(text) ||
    /^import\s+test\s+from\s*'@playwright\/test'/m.test(text);
  if (valueTestImport) importViolations.push(toPosixRel(abs));
}
if (importViolations.length) {
  fail(
    `${importViolations.length} 个 spec 仍从 '@playwright/test' 直接取 test（不经过 worker 会话 fixture，` +
      `该文件将拿不到独立会话态而静默共享账号）：\n  ` +
      importViolations.slice(0, 30).join('\n  ') +
      (importViolations.length > 30 ? `\n  …共 ${importViolations.length} 个` : '')
  );
} else {
  note(`[C] 主套件 ${specFiles.length} 个 spec 全部经统一 fixture 模块取 test`);
}

// 按分片拼账号的写法只允许存在于会话模块与 globalSetup 两处
const DERIVE_ALLOWED = new Set(['e2e/worker-auth.ts', 'e2e/global-setup.ts']);
const shardConcat = /`e2e_admin_s\$\{/;
const deriveViolations = [];
function walkTs(dir, out) {
  for (const entry of require('node:fs').readdirSync(dir, { withFileTypes: true })) {
    const abs = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === 'node_modules' || entry.name === '.auth') continue;
      walkTs(abs, out);
    } else if (entry.isFile() && /\.(ts|mjs)$/.test(entry.name)) {
      out.push(abs);
    }
  }
}
const tsFiles = [];
walkTs(path.join(FRONTEND_DIR, 'e2e'), tsFiles);
for (const abs of tsFiles) {
  const rel = toPosixRel(abs);
  if (DERIVE_ALLOWED.has(rel)) continue;
  const text = readFileSync(abs, 'utf8');
  if (shardConcat.test(text))
    deriveViolations.push(`${rel}（在会话模块之外按分片拼 e2e_admin_s* 账号）`);
}
if (deriveViolations.length) {
  fail(
    `账号派生出现第二处事实来源（必须统一走 e2e/worker-auth.ts）：\n  ` +
      deriveViolations.join('\n  ')
  );
} else {
    note(`[C] 账号派生单点：扫描 ${tsFiles.length} 个 e2e ts 文件，无第二处拼账号写法`);
}

// 会话态与凭证文件必须继续被 gitignore（内含 token 与密码，入库即凭证外泄）
const gitignore = readSource('.gitignore') ?? readSource('../.gitignore') ?? '';
const feGitignore = (() => {
  const p = path.join(FRONTEND_DIR, '.gitignore');
  return existsSync(p) ? readFileSync(p, 'utf8') : '';
})();
if (!/e2e\/\.auth\/?/.test(`${gitignore}\n${feGitignore}`))
  fail('e2e/.auth/ 未被 gitignore —— 逐 worker 会话态与密码文件会入库，判红');
else note('[C] e2e/.auth/ 仍在 gitignore 覆盖内');

// 会话池清单路径必须与门禁常量一致（防漂移：globalSetup 写、fixture 读、门禁校验三处同源）
if (workerAuth && !/e2e\/\.auth\/worker-sessions\.json/.test(workerAuthSource ?? ''))
  fail('会话池清单路径不在 e2e/.auth/ 下 —— 该目录之外无法保证 gitignore 覆盖');

// ------------------------------------------------------------------ 汇总（计数自证）
console.log('\n──────── E2E worker 账号与会话态门禁 ────────');
note(`扫描 spec 文件 ${specFiles.length} 个 / e2e ts 文件 ${tsFiles.length} 个 / 校验失败项 ${errors.length} 条`);
if (errors.length) {
  for (const e of errors) console.error(`❌ ${e}`);
  console.error(`\n共 ${errors.length} 项失败（会话池规模 = 解析后的 Playwright workers 数）`);
  process.exit(1);
}
console.log('✅ 每 worker 独立账号与独立会话态校验通过');

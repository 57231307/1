#!/usr/bin/env node
/**
 * check-e2e-feature-pairing.mjs — 新增后端端点必须配套 E2E 用例的合并前门禁
 *
 * 功能：把"本支新增的后端端点"与"前端 e2e 用例语料"做配对判定。新增端点在
 *       frontend/e2e 下找不到引用即判红，杜绝"功能合进主干但验证面一条没加"这类
 *       存量漏测的继续累积。
 * 调用方：.github/workflows/e2e-pairing.yml 在 pull_request 上跑；本地推送前门组与
 *       开发者手工跑 `node frontend/scripts/check-e2e-feature-pairing.mjs`；
 *       反向自证跑同文件加 --self-test。
 * 入参：--baseline <文本快照路径> 覆盖基线（缺省 frontend/scripts/route-snapshot.txt，
 *       即主干已认端点集）；--corpus <目录> 覆盖用例语料根（缺省 frontend/e2e）；
 *       --exempt <json> 覆盖豁免台账；--floor <数> 覆盖端点数地板；--self-test 跑反向探针。
 * 传给谁：结论只写标准输出与退出码，不回调任何服务；CI 按退出码判负。
 * 存什么：不写仓内任何既有文件；自证模式在 frontend/scripts 下临时生成基线与台账夹具，
 *       结束时逐个删除，删不掉的会点名。
 * 存哪里：临时夹具只落 frontend/scripts，运行后清除。
 *
 * 判据语义（写死在此，回报同口径）：
 *   1 端点集来自 walkBackendRoutes()，与 check-api-paths 和 route-snapshot 同一套
 *     nest/merge 语义，不另起第二套解析口径（两套口径必然各自数出一套端点）。
 *   2 新增集 = 当前端点集 − 基线端点集，按 "METHOD PATH" 逐字对比。
 *   3 配对判据用静态前缀：/a/b/* 取 "/a/b/"，要求该前缀字面量出现在语料某文件里。
 *     这是不依赖编译时能取到的最强判据；再窄会误杀模板串与字符串拼接的合法写法。
 *   4 前缀命中但该文件全文没有该 METHOD 记号时只点名为 WEAK 不判红：用例经封装的
 *     请求助手发请求时方法名不在 spec 里出现属正常形态，判红会把人推向"为过门硬塞
 *     方法名"的假用例。
 *   5 豁免台账每条必须带非空 reason；台账里存在但已不在新增集的条目判红（过期锁），
 *     否则同一位次再发时没人能发现这条豁免早已失去意义。
 *   6 基线、当前集或语料任一解析量低于地板时退 9：判定器瞎了的时候它的绿不可信。
 *
 * 退出码：0 通过；1 有判红项；9 判定器或输入不可用。
 */
import { readFileSync, readdirSync, existsSync, writeFileSync, unlinkSync } from 'fs';
import { join, dirname, relative } from 'path';
import { fileURLToPath } from 'url';
import { walkBackendRoutes } from './check-api-paths.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = join(HERE, '..', '..');
const argv = process.argv.slice(2);
const flag = (name, dflt) => {
  const i = argv.indexOf('--' + name);
  return i >= 0 && argv[i + 1] && !argv[i + 1].startsWith('--') ? argv[i + 1] : dflt;
};
const SELF_TEST = argv.includes('--self-test');
const BASELINE = flag('baseline', join(HERE, 'route-snapshot.txt'));
const CORPUS = flag('corpus', join(REPO, 'frontend', 'e2e'));
const EXEMPT = flag('exempt', join(HERE, 'e2e-feature-pairing-exempt.json'));
const FLOOR = Number(flag('floor', 1000));

const say = m => process.stdout.write(m + '\n');

/** 读基线快照：每行 METHOD\tPATH\thandler\tfile，# 开头是头部注释 */
function readBaseline(file) {
  if (!existsSync(file)) return { keys: null, error: '缺基线文件：' + file };
  const keys = new Set();
  for (const line of readFileSync(file, 'utf8').split(/\r?\n/)) {
    if (!line || line.startsWith('#')) continue;
    const parts = line.split('\t');
    if (parts.length < 2) continue;
    keys.add(parts[0] + ' ' + parts[1]);
  }
  return { keys, error: null };
}

/** 当前端点集：与基线同一谓词，键为规范形 "METHOD PATH" */
function currentKeys() {
  const { handlers } = walkBackendRoutes();
  const out = new Map();
  for (const [key, info] of handlers) {
    // walkBackendRoutes 的键是 "PATH METHOD"（与 route-snapshot 的取值次序一致：
    // 快照行 = METHOD<TAB>PATH）。读反会让"路径前缀"退化成方法名，
    // 于是任何含 DELETE/GET 字样的文件都算命中，门会恒绿。
    const i = key.lastIndexOf(' ');
    const path = key.slice(0, i);
    const method = key.slice(i + 1);
    out.set(method + ' ' + path, { method, path, handler: info.handler });
  }
  return out;
}

/**
 * 用例语料清单：只收真用例文件。
 * 排除 helper/fixture/清单类文件是这道门有效的前提——e2e 目录下存在把整张路由表
 * 抄进常量的枚举文件，一旦收进语料，任何按前缀判的配对门都会对全部端点判命中。
 */
function corpusFiles(dir) {
  const acc = [];
  const skip = new Set([
    'node_modules',
    'dist',
    '.git',
    'playwright-report',
    'test-results',
    'blob-report',
  ]);
  (function walk(d) {
    for (const e of readdirSync(d, { withFileTypes: true })) {
      if (e.isDirectory()) {
        if (!skip.has(e.name)) walk(join(d, e.name));
      } else if (/\.spec\.[cm]?[jt]s$/.test(e.name)) {
        acc.push(join(d, e.name));
      }
    }
  })(dir);
  return acc;
}

/** 静态前缀：路径里第一个 * 之前的部分；无 * 时取整串 */
function staticPrefix(path) {
  const star = path.indexOf('*');
  return star < 0 ? path : path.slice(0, star);
}

function loadExempt(file) {
  if (!existsSync(file)) return { map: new Map(), error: null };
  try {
    const arr = JSON.parse(readFileSync(file, 'utf8'));
    const map = new Map();
    for (const it of Array.isArray(arr) ? arr : []) {
      if (!it || !it.endpoint) continue;
      map.set(it.endpoint, String(it.reason || '').trim());
    }
    return { map, error: null };
  } catch (e) {
    return { map: new Map(), error: '豁免台账不是合法 JSON：' + e.message };
  }
}

/**
 * 判定主体。
 * @param {object} over 自证用的入参覆盖（baseline corpus exempt floor）
 * @returns {{code:number}} code 与退出码同义
 */
function judge(over = {}) {
  const baselineFile = over.baseline || BASELINE;
  const corpusDir = over.corpus || CORPUS;
  const exemptFile = over.exempt || EXEMPT;
  const floor = over.floor || FLOOR;
  const b = readBaseline(baselineFile);
  if (b.error) return { code: 9, head: 'TOOL_BUG ' + b.error };
  if (b.keys.size < floor) {
    return {
      code: 9,
      head:
        'TOOL_BUG 基线只解析出 ' +
        b.keys.size +
        ' 枚端点，低于地板 ' +
        floor +
        '，其"零新增"结论不可信',
    };
  }
  const cur = currentKeys();
  if (cur.size < floor) {
    return {
      code: 9,
      head:
        'TOOL_BUG 当前路由面只解析出 ' +
        cur.size +
        ' 枚端点，低于地板 ' +
        floor +
        '，判定器无检测力',
    };
  }
  const files = corpusFiles(corpusDir);
  if (files.length < 10) {
    return {
      code: 9,
      head: 'TOOL_BUG 用例语料只看到 ' + files.length + ' 个文件，扫面过窄，配对结论不可信',
    };
  }
  const texts = files.map(f => ({
    rel: relative(corpusDir, f).replace(/\\/g, '/'),
    body: readFileSync(f, 'utf8'),
  }));
  const ex = loadExempt(exemptFile);
  if (ex.error) return { code: 9, head: 'TOOL_BUG ' + ex.error };

  const added = [...cur.keys()].filter(k => !b.keys.has(k)).sort();
  let fail = 0;
  let paired = 0;
  let weak = 0;
  let exempted = 0;
  for (const key of added) {
    const { method, path } = cur.get(key);
    const prefix = staticPrefix(path);
    const hits = texts.filter(t => t.body.includes(prefix));
    if (hits.length > 0) {
      if (hits.some(t => new RegExp('\\b' + method + '\\b').test(t.body))) paired += 1;
      else {
        weak += 1;
        say(
          '  PAIRING_WEAK ' + key + ' 前缀命中 ' + hits[0].rel + ' 但该文件无 ' + method + ' 记号'
        );
      }
      continue;
    }
    if (ex.map.has(key)) {
      const reason = ex.map.get(key);
      if (!reason) {
        fail += 1;
        say('  EXEMPT_NO_REASON ' + key + ' 豁免没写理由');
      } else {
        exempted += 1;
        say('  PAIRING_EXEMPT ' + key + '（' + reason + '）');
      }
      continue;
    }
    fail += 1;
    say(
      '  PAIRING_RED ' + key + ' 新增端点在 frontend/e2e 下零引用，须配套用例或显式登记带理由的豁免'
    );
  }
  const addedSet = new Set(added);
  for (const key of ex.map.keys()) {
    if (!addedSet.has(key)) {
      fail += 1;
      say('  EXEMPT_STALE ' + key + ' 已不在新增集里，豁免条目该删（留着会掩盖同位再发）');
    }
  }
  return {
    code: fail > 0 ? 1 : 0,
    head:
      'PAIRING baseline=' +
      b.keys.size +
      ' current=' +
      cur.size +
      ' added=' +
      added.length +
      ' paired=' +
      paired +
      ' weak=' +
      weak +
      ' exempt=' +
      exempted +
      ' unpaired=' +
      fail +
      ' corpusFiles=' +
      files.length,
  };
}

/** 临时夹具：写文件并登记待删 */
function makeTemp(name, content, cleanup) {
  const p = join(HERE, name);
  writeFileSync(p, content);
  cleanup.push(p);
  return p;
}

/**
 * 反向自证：用真谓词与真语料，靠"从基线里挖掉一条真端点"造出新增，验四条——
 * 无引用必判红、有引用必判绿、空基线必判 9（不许报"零新增"当过）、
 * 带理由的豁免能救红且缺台账时不放行。任一没命中就判判定器无检测力。
 */
function selfTest() {
  const cleanup = [];
  const cur = currentKeys();
  const base = readBaseline(BASELINE);
  if (base.error) return { code: 9, head: 'TOOL_BUG 自证前置不成立：' + base.error };
  const texts = corpusFiles(CORPUS).map(f => readFileSync(f, 'utf8'));
  const covered = [];
  const uncovered = [];
  for (const key of cur.keys()) {
    const prefix = staticPrefix(cur.get(key).path);
    (texts.some(t => t.includes(prefix)) ? covered : uncovered).push(key);
  }
  say(
    'SELFTEST_POOL current=' +
      cur.size +
      ' covered=' +
      covered.length +
      ' uncovered=' +
      uncovered.length
  );
  if (covered.length === 0 || uncovered.length === 0) {
    return {
      code: 9,
      head:
        uncovered.length === 0
          ? 'TOOL_BUG 全部端点都判命中：判据过宽或语料里混进了枚举整张路由表的清单文件，这道门恒绿无意义'
          : 'TOOL_BUG 全部端点都判未命中：前缀匹配或语料根不对，这道门恒红也无意义',
    };
  }
  const dropBaseline = dropKey => {
    const kept = [...base.keys]
      .filter(k => k !== dropKey)
      .sort()
      .map(k => {
        const sp = k.indexOf(' ');
        return k.slice(0, sp) + '\t' + k.slice(sp + 1) + '\tprobe\thandler';
      });
    return makeTemp(
      '.selftest-baseline-' + cleanup.length + '.txt',
      '# layers=0 endpoints=' + kept.length + '\n' + kept.join('\n') + '\n',
      cleanup
    );
  };
  const emptyBaseline = makeTemp(
    '.selftest-baseline-empty.txt',
    '# layers=0 endpoints=0\n',
    cleanup
  );
  const emptyExempt = makeTemp('.selftest-exempt-empty.json', '[]\n', cleanup);
  const okExempt = makeTemp(
    '.selftest-exempt-ok.json',
    JSON.stringify([{ endpoint: uncovered[0], reason: '自证夹具：内部端点无界面入口' }], null, 2) +
      '\n',
    cleanup
  );
  const noReasonExempt = makeTemp(
    '.selftest-exempt-noreason.json',
    JSON.stringify([{ endpoint: uncovered[1] || uncovered[0], reason: '' }], null, 2) + '\n',
    cleanup
  );

  const probes = [];
  probes.push([
    'unpairedMustBeRed',
    judge({ baseline: dropBaseline(uncovered[0]), exempt: emptyExempt }).code,
    1,
  ]);
  probes.push([
    'pairedMustBeGreen',
    judge({ baseline: dropBaseline(covered[0]), exempt: emptyExempt }).code,
    0,
  ]);
  probes.push(['emptyBaselineMustBeToolBug', judge({ baseline: emptyBaseline }).code, 9]);
  probes.push([
    'exemptWithReasonMustBeGreen',
    judge({ baseline: dropBaseline(uncovered[0]), exempt: okExempt }).code,
    0,
  ]);
  probes.push([
    'exemptWithoutReasonMustBeRed',
    judge({ baseline: dropBaseline(uncovered[1] || uncovered[0]), exempt: noReasonExempt }).code,
    1,
  ]);

  let caught = 0;
  const missed = [];
  for (const [name, got, want] of probes) {
    if (got === want) caught += 1;
    else missed.push(name + '(want=' + want + ' got=' + got + ')');
  }
  for (const p of cleanup) {
    try {
      unlinkSync(p);
    } catch (e) {
      say('  SELFTEST_CLEANUP_FAILED ' + p + ' ' + e.message);
    }
  }
  say(
    'SELFTEST caught=' +
      caught +
      '/' +
      probes.length +
      (missed.length ? ' missed:' + missed.join(',') : '')
  );
  return caught === probes.length
    ? { code: 0, head: 'PAIRING_DETECTOR_OK ' + probes.length + ' 道反向探针全部按预期命中' }
    : { code: 9, head: 'TOOL_BUG 反向探针有没命中的道，判定器无检测力，其绿不可信' };
}

const out = SELF_TEST ? selfTest() : judge();
say(out.head || '');
if (!SELF_TEST) {
  if (out.code === 0) say('PAIRING_GREEN 新增端点全部有用例配对或已带理由豁免');
  if (out.code === 1) say('PAIRING_RED 有新增端点未配套 E2E 用例');
  if (out.code === 9) say('PAIRING_TOOLBUG 判定器或输入不可用，本道结论按不通过处理');
} else if (out.code === 0) {
  say('PAIRING_SELFTEST_OK');
}
process.exit(out.code);

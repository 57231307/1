#!/usr/bin/env node
/**
 * check-route-perm-ratchet.mjs — 前端路由权限覆盖差集棘轮锁（只拦新增）
 *
 * 功能：三条规则。① 静态解析 src/router/index.ts 的全路由路径→meta.permission（含纯别名
 * redirect 链继承），解析口径逐字移植自 e2e/traversal/permission-model.ts 的
 * parseRouterPermissions——两处必须同源，另写一套正则会因别名继承与嵌套块窗口口径不同
 * 把差集数算错；② 找出"有门控但未被 ROUTE_PERMISSIONS 人工快照覆盖"的路由集合，逐条生成
 * (路由 id, route 路径, 权限值) 三元组指纹。指纹绝不含行号：行号会随排版漂移把无关重排
 * 伪造成"新增"，棘轮因此爆红失去只拦新增的语义；③ 与基线对账——基线外新增指纹判红，
 * 基线内已消失的条目只提示"应删行缩短"、不判红（与注释棘轮同语义）。
 * "有门控"的判定镜像 hasRoutePermission 的 `!required` 分支：null 与空串 = 登录即可达、
 * 不入差集；空数组 [] 在守卫里为真值且恒拒绝，属最强门控、必须入差集。
 *
 * 调用方：前端门禁链（本地与 CI 静态检查 step）与人工校验；本锁不参与 e2e 派生，
 * 派生输入与 ROUTE_PERMISSIONS↔router 防漂移核对仍归 permission-model.ts（44 号 spec）。
 * 入参：命令行白名单——无参 = 全量对账；--regen = 重算差集并写基线（唯一合法基线来源）；
 * --self-test = 内联夹具双向自证判定器本身。未知或拼错的参数直接拒绝执行，
 * 防止"--self-test 打错一个字母＝门禁照常绿、自证零执行"的假绿通道。
 * 传给谁：判定结果只面向 CI 退出码与 stdout/stderr 真实计数，不被任何运行时代码消费。
 * 存什么：--regen 落盘当前差集全量 {total, entries:[{id, route, perm}]}；条目只代表
 * "登记时该路由有门控且未进快照"的历史存量债，不代表已评审放行。
 * 存哪里：基线 frontend/scripts/route-perm-coverage-baseline.json；判定器本文件；
 * 只读输入 src/router/index.ts、e2e/traversal/modules.config.ts、
 * e2e/traversal/permission-model.ts（三者本轮均不由本脚本改写）。
 *
 * 硬约束口径：
 * - ROUTE_PERMISSIONS 是人工评审产物，本锁不自动生成、不改写它，只做差集守护与候选呈现；
 * - 双向核（快照↔路由反向全等）的前置条件是存量差集清零，本锁现阶段只拦新增；
 * - 别名目标缺失 / 成环 / 非单引号字面量 / 同路径门控冲突一律抛错判红交人工，
 *   禁止静默按"登录即可达"降级（与 permission-model.ts 同纪律）。
 *
 * usage: node scripts/check-route-perm-ratchet.mjs [--regen | --self-test]
 */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND = path.resolve(HERE, '..');
const ROUTER_FILE = path.join(FRONTEND, 'src', 'router', 'index.ts');
const MODULES_FILE = path.join(FRONTEND, 'e2e', 'traversal', 'modules.config.ts');
const MODEL_FILE = path.join(FRONTEND, 'e2e', 'traversal', 'permission-model.ts');
const BASELINE = path.join(HERE, 'route-perm-coverage-baseline.json');

// ==================== router 源解析（口径逐字移植 permission-model.ts） ====================

const ROUTES_START = 'const routes: RouteRecordRaw[]';
const ROUTES_END = 'createRouter(';

/**
 * 从 router 源文本解析 原始路由表 与 纯别名表（两个中间产物，供对账与别名诊断共用一次扫描）。
 *
 * 口径：router/index.ts 为"单一 children 数组扁平登记"，path 值不带前导 '/' 时按 '/'
 * 拼接为完整路径；每个路由对象的块位于本条 `path:` 与下一条 `path:` 之间；
 * `permission:` 兼容字符串与数组；`redirect:` 仅当块内不含 component/children（纯别名）
 * 时生效。解析不出边界、别名目标缺失/成环/非字符串字面量、同路径门控冲突时抛错判红，
 * 绝不静默降级。移植纪律：本函数与 parseRouterPermissions、extractBlockPermission、
 * extractBlockRedirectAlias、stripQueryAndHash、inheritRedirectPermissions 合起来必须与
 * permission-model.ts 同名实现行为一致，改动任何一处都要同步另一处。
 */
function buildRouterTable(source) {
  const start = source.indexOf(ROUTES_START);
  const end = source.indexOf(ROUTES_END);
  if (start < 0 || end < 0 || end <= start) {
    throw new Error(
      `无法在 src/router/index.ts 中定位 routes 数组边界（缺少 "${ROUTES_START}" / "${ROUTES_END}" 标记）` +
        `——router 结构已变，解析口径需人工复核后同步更新本文件与 permission-model.ts，禁止静默放行。`
    );
  }
  const body = source.slice(start, end);
  const pathRe = /path:\s*'([^']*)'/g;
  const hits = [];
  let m;
  while ((m = pathRe.exec(body)) !== null) {
    hits.push({ raw: m[1], from: m.index, to: pathRe.lastIndex });
  }
  if (hits.length === 0) {
    throw new Error('router 源文本中未解析到任何 `path:` 路由项——解析口径失效，判红交人工。');
  }

  const map = {};
  /** 纯别名路由：path → redirect 字面量目标（完整路径） */
  const aliases = {};
  for (let i = 0; i < hits.length; i++) {
    const blockEnd = i + 1 < hits.length ? hits[i + 1].from : body.length;
    const block = body.slice(hits[i].to, blockEnd);
    const fullPath = hits[i].raw.startsWith('/') ? hits[i].raw : `/${hits[i].raw}`;
    const perm = extractBlockPermission(block);
    if (fullPath in map) {
      if (JSON.stringify(map[fullPath]) !== JSON.stringify(perm)) {
        throw new Error(
          `路由 ${fullPath} 在 router 中出现多次且门控不一致（${JSON.stringify(
            map[fullPath]
          )} vs ${JSON.stringify(perm)}）——解析输入有歧义，判红交人工。`
        );
      }
    } else {
      map[fullPath] = perm;
    }
    const alias = extractBlockRedirectAlias(block, fullPath);
    if (alias !== undefined) {
      if (fullPath in aliases && aliases[fullPath] !== alias) {
        throw new Error(
          `别名路由 ${fullPath} 出现多次且 redirect 目标不一致（${aliases[fullPath]} vs ${alias}）` +
            `——解析输入有歧义，判红交人工。`
        );
      }
      aliases[fullPath] = alias;
    }
  }
  return { map, aliases, pathHits: hits.length };
}

/** 有效门控表 = 原始表经 redirect 链继承后的结果（与 permission-model.ts 同语义） */
function parseRouterPermissions(table) {
  return inheritRedirectPermissions(table.map, table.aliases);
}

function extractBlockPermission(block) {
  const arr = block.match(/permission:\s*\[([^\]]*)\]/);
  if (arr) {
    return [...arr[1].matchAll(/'([^']*)'/g)].map(x => x[1]);
  }
  const single = block.match(/permission:\s*'([^']*)'/);
  return single ? single[1] : null;
}

/**
 * 纯别名（path + redirect、无 component/children）的 redirect 目标提取。
 * 返回 undefined = 本块不是纯别名（无 redirect，或带自身组件的重定向）。
 */
function extractBlockRedirectAlias(block, fullPath) {
  if (!block.includes('redirect:')) return undefined;
  if (/\b(component|children):/.test(block)) return undefined;
  const lit = block.match(/redirect:\s*'([^']*)'/);
  if (!lit) {
    throw new Error(
      `别名路由 ${fullPath} 的 redirect 目标不是单引号字符串字面量（函数/对象/命名路由形态）` +
        `——静态解析无法跟随，禁止把它按"登录即可达"静默派生，需人工复核本解析口径。`
    );
  }
  if (!lit[1].startsWith('/')) {
    throw new Error(
      `别名路由 ${fullPath} 的 redirect 目标 '${lit[1]}' 不是绝对路径——相对目标的解析父级无法由扁平文本口径确定，需人工复核。`
    );
  }
  return lit[1];
}

/** query/hash 属目标页面的页签状态，不参与路由表键匹配；门控继承目标路径本身的 meta */
function stripQueryAndHash(target) {
  const sep = target.search(/[?#]/);
  return sep === -1 ? target : target.slice(0, sep);
}

/** redirect 链继承：别名路由的有效门控 = 链尾真实路由的 meta.permission；异常一律抛错 */
function inheritRedirectPermissions(map, aliases) {
  const effective = { ...map };
  for (const [startPath, firstTarget] of Object.entries(aliases)) {
    const seen = new Set([startPath]);
    let current = stripQueryAndHash(firstTarget);
    for (;;) {
      if (seen.has(current)) {
        throw new Error(
          `别名路由 ${startPath} 的 redirect 链成环（${[...seen, current].join(' → ')}）` +
            `——router 配置异常，判红交人工。`
        );
      }
      seen.add(current);
      if (!(current in map)) {
        throw new Error(
          `别名路由 ${startPath} 的 redirect 目标 ${current} 不在 router 已解析路由表中` +
            `——无法继承目标门控；按"登录即可达"派生会复现该族伪红机制，判红交人工。`
        );
      }
      const next = aliases[current];
      if (next) {
        current = stripQueryAndHash(next);
        continue;
      }
      effective[startPath] = map[current];
      break;
    }
  }
  return effective;
}

// ==================== 人工快照与遍历清单解析（只读） ====================

/**
 * 从 permission-model.ts 源文本提取 ROUTE_PERMISSIONS 对象字面量并按受限文法解析。
 * 文法：键 = 裸标识符或单引号串；值 = 单引号串 / null / 单引号串数组；允许行注释与块注释、
 * 尾随逗号。刻意不用 new Function/eval 求值——快照是人工录入产物，跨行键名、重复键、
 * 数字或对象等越界形态都必须被本解析器显式拒绝并点名判红，而不是被 JS 静默吞掉后一个键。
 */
function parseRoutePermSnapshot(source) {
  const m = source.match(
    /export const ROUTE_PERMISSIONS: Record<string, RoutePermValue> = \{([\s\S]*?)\n\};/
  );
  if (!m) {
    throw new Error(
      '无法在 e2e/traversal/permission-model.ts 中定位 ROUTE_PERMISSIONS 对象字面量' +
        '——表已改名或换结构，解析口径需人工复核后同步更新本文件，禁止静默放行。'
    );
  }
  return parseStrictPermObjectBody(m[1]);
}

/** 受限文法解析器本体（见 parseRoutePermSnapshot 的文法声明）；越界形态一律抛错 */
function parseStrictPermObjectBody(body) {
  const result = {};
  let i = 0;
  const skip = () => {
    for (;;) {
      while (i < body.length && /\s/.test(body[i])) i++;
      if (body.startsWith('//', i)) {
        const nl = body.indexOf('\n', i);
        i = nl === -1 ? body.length : nl;
        continue;
      }
      if (body.startsWith('/*', i)) {
        const end = body.indexOf('*/', i);
        if (end === -1) throw new Error('ROUTE_PERMISSIONS 字面量内块注释未闭合——判红交人工。');
        i = end + 2;
        continue;
      }
      break;
    }
  };
  const readQuoted = what => {
    const begin = i;
    i++;
    let out = '';
    for (;;) {
      if (i >= body.length) throw new Error(`ROUTE_PERMISSIONS 中${what}字符串未闭合（自 ${begin} 起）`);
      const ch = body[i];
      if (ch === '\\') {
        out += body[i + 1];
        i += 2;
        continue;
      }
      if (ch === "'") {
        i++;
        break;
      }
      if (ch === '\n') throw new Error(`ROUTE_PERMISSIONS 中${what}字符串跨行——值必须单行字面量`);
      out += ch;
      i++;
    }
    return out;
  };
  skip();
  while (i < body.length) {
    let key;
    if (body[i] === "'") key = readQuoted('键');
    else if (/[A-Za-z_]/.test(body[i])) {
      let s = '';
      while (i < body.length && /[A-Za-z0-9_-]/.test(body[i])) s += body[i++];
      key = s;
    } else {
      throw new Error(
        `ROUTE_PERMISSIONS 键起始字符非法：${JSON.stringify(body[i])}——判红交人工。`
      );
    }
    skip();
    if (body[i] !== ':') throw new Error(`快照键 ${key} 后缺少冒号——文法越界，判红交人工。`);
    i++;
    skip();
    let value;
    if (body[i] === "'") value = readQuoted('值');
    else if (body.startsWith('null', i)) {
      value = null;
      i += 4;
    } else if (body[i] === '[') {
      i++;
      const arr = [];
      skip();
      if (body[i] === ']') {
        i++;
      } else {
        for (;;) {
          skip();
          if (body[i] === ']') {
            i++;
            break;
          }
          if (body[i] !== "'")
            throw new Error(`快照键 ${key} 的数组项不是单引号字符串——文法越界，判红交人工。`);
          arr.push(readQuoted('数组项'));
          skip();
          if (body[i] === ',') {
            i++;
            continue;
          }
          if (body[i] === ']') {
            i++;
            break;
          }
          throw new Error(`快照键 ${key} 数组未正常闭合——判红交人工。`);
        }
      }
      value = arr;
    } else {
      throw new Error(
        `快照键 ${key} 的值形态非法（只允许单引号字符串 / null / 字符串数组）——判红交人工。`
      );
    }
    if (Object.prototype.hasOwnProperty.call(result, key)) {
      throw new Error(`ROUTE_PERMISSIONS 存在重复键：${key}——人工快照录入事故，判红交人工。`);
    }
    result[key] = value;
    skip();
    if (body[i] === ',') {
      i++;
      skip();
      continue;
    }
    if (i >= body.length) break;
    throw new Error(
      `快照键值对之后期望逗号或表结束，实际 ${JSON.stringify(body[i])}——文法越界，判红交人工。`
    );
  }
  return result;
}

/**
 * 从 modules.config.ts 源文本提取 TRAVERSAL_MODULES 的 (id, route) 登记对。
 * 只认 `id: '...'` 后紧跟 `route: '...'` 的登记条目；接口声明里的
 * `id: string;` 无引号，天然不匹配。id 重复即配置事故，抛错判红。
 */
function parseTraversalModules(source) {
  const re = /\{\s*id:\s*'([^']+)'\s*,\s*route:\s*'([^']+)'\s*,/g;
  const modules = [];
  const seen = new Set();
  let m;
  while ((m = re.exec(source)) !== null) {
    if (seen.has(m[1])) {
      throw new Error(`TRAVERSAL_MODULES 重复 id：${m[1]}——遍历清单录入事故，判红交人工。`);
    }
    seen.add(m[1]);
    modules.push({ id: m[1], route: m[2] });
  }
  if (modules.length === 0) {
    throw new Error(
      'modules.config.ts 中未解析到任何 (id, route) 登记——解析口径失效，判红交人工。'
    );
  }
  return modules;
}

// ==================== 差集与指纹 ====================

/** 未进遍历清单的路由由路径派生稳定 id：去掉前导 '/'、段间 '/' 换 '-'；根路径用固定 token */
function slugIdOfRoute(route) {
  const slug = route.replace(/^\//, '').replace(/\//g, '-');
  return slug === '' ? 'index' : slug;
}

/** 指纹三元组的序列化形态；绝不含行号（见文件头②） */
function fingerprint(entry) {
  return JSON.stringify([entry.id, entry.route, entry.perm]);
}

/**
 * 计算差集：有效门控非空（守卫口径 `!required` 之外）且未被快照覆盖的路由。
 * 覆盖判据 = 存在某模块 route 等于该路径且其 id 在 ROUTE_PERMISSIONS 有键；
 * 快照只按"是否覆盖"参与，值是否漂移由 permission-model.ts 的防漂移门禁负责，两锁不重复判。
 * 返回条目附 inList（已进遍历清单）与 viaAlias（纯别名继承而来，非本路由块自带门控）
 * 两个诊断标记，供回报点名差异归属；二者不进指纹。
 * modules 由调用方传入（main 读真实 modules.config.ts，--self-test 传夹具），
 * 判定器不持有全局缓存，保证夹具与实跑共用同一函数体而不互染。
 */
function computeGapEntries(table, snapshot, modules) {
  const effective = parseRouterPermissions(table);
  const listIdByRoute = new Map();
  for (const mod of modules) {
    if (!listIdByRoute.has(mod.route)) listIdByRoute.set(mod.route, mod.id);
  }
  const covered = new Set();
  for (const [route, id] of listIdByRoute) {
    if (id in snapshot) covered.add(route);
  }
  const gap = [];
  for (const [route, perm] of Object.entries(effective)) {
    if (!perm) continue;
    if (covered.has(route)) continue;
    const inList = listIdByRoute.has(route);
    const viaAlias = route in table.aliases && !table.map[route];
    gap.push({
      id: inList ? listIdByRoute.get(route) : slugIdOfRoute(route),
      route,
      perm,
      inList,
      viaAlias,
    });
  }
  gap.sort((a, b) => (fingerprint(a) < fingerprint(b) ? -1 : 1));
  return gap;
}

/** 差集对基线的双向差；判红只看 added（只拦新增语义），removed 仅提示 */
function diffEntries(current, baselineEntries) {
  const curKeys = new Set(current.map(fingerprint));
  const baseKeys = new Set(baselineEntries.map(fingerprint));
  return {
    added: current.filter(e => !baseKeys.has(fingerprint(e))),
    removed: baselineEntries.filter(e => !curKeys.has(fingerprint(e))),
  };
}

/** 判红组装：main 与 --self-test 共用这一份实现，两处各写口径必然漂移 */
function verdict(added, removed) {
  const lines = [];
  for (const e of added) {
    lines.push(
      `  + ${e.route}（id=${e.id}，perm=${JSON.stringify(e.perm)}${e.viaAlias ? '，别名继承' : ''}）`
    );
  }
  for (const e of removed) {
    lines.push(`  - 应删行缩短：${e.route}（id=${e.id}，perm=${JSON.stringify(e.perm)}）`);
  }
  return { red: added.length > 0, lines };
}

// ==================== 实跑装配（main 与 --regen 共用） ====================

/** 读三份只读输入并产出（路由表, 快照, 差集, 计数）；任何解析异常原样上抛由调用方判红 */
function collectReal() {
  const routerSrc = fs.readFileSync(ROUTER_FILE, 'utf8');
  const modelSrc = fs.readFileSync(MODEL_FILE, 'utf8');
  const modulesSrc = fs.readFileSync(MODULES_FILE, 'utf8');
  const table = buildRouterTable(routerSrc);
  const snapshot = parseRoutePermSnapshot(modelSrc);
  const modules = parseTraversalModules(modulesSrc);
  const gap = computeGapEntries(table, snapshot, modules);
  const effective = parseRouterPermissions(table);
  const gated = Object.values(effective).filter(p => !!p).length;
  const coveredRoutes = new Set(modules.filter(m => m.id in snapshot).map(m => m.route));
  return {
    table,
    snapshot,
    modules,
    gap,
    counts: {
      paths: Object.keys(effective).length,
      gated,
      snapshotKeys: Object.keys(snapshot).length,
      covered: coveredRoutes.size,
      gap: gap.length,
      gapInList: gap.filter(e => e.inList).length,
      gapNotInList: gap.filter(e => !e.inList).length,
    },
  };
}

/** 真实计数行：main 打印与 --self-test 地板断言共用字段名，防止两套口径各说各话 */
function fmtCounters(c) {
  return (
    `router 解析路由 ${c.paths} 条（path 命中 ${c.pathHits} 处，去重后）｜` +
    `有门控 ${c.gated}｜快照 ${c.snapshotKeys} 键（覆盖 ${c.covered} 路径）｜` +
    `差集 ${c.gap}（已进清单 ${c.gapInList} / 未进清单 ${c.gapNotInList}）`
  );
}

// ==================== --self-test：判定器双向自证 ====================

/** 夹具路由块：与真实 router 同形态（根别名、纯别名、带组件的 redirect、数组门控、覆盖键） */
const FIXTURE_BLOCKS = {
  rootAlias: `  {\n    path: '/',\n    redirect: '/home',\n  },`,
  login: `  {\n    path: '/login',\n    component: () => import('Login.vue'),\n    meta: { title: '登录' },\n  },`,
  home: `  {\n    path: 'home',\n    component: () => import('Home.vue'),\n    meta: { permission: 'dashboard:read' },\n  },`,
  pureAlias: `  {\n    // 纯别名：块内无 component/children，门控应继承 /home\n    path: 'legacy-entry',\n    redirect: '/home',\n  },`,
  selfGated: `  {\n    // 带组件的 redirect 非纯别名，门控以本块 meta 为准\n    path: 'self-gated',\n    component: () => import('Self.vue'),\n    redirect: '/home',\n    meta: { permission: 'finance:read' },\n  },`,
  covered: `  {\n    path: 'covered-by-snapshot',\n    component: () => import('Covered.vue'),\n    meta: { permission: 'users:read' },\n  },`,
  gapArray: `  {\n    path: 'gap-array',\n    component: () => import('Gap.vue'),\n    meta: { permission: ['a:read', 'b:read'] },\n  },`,
  aliasToMissing: `  {\n    path: 'alias-to-missing',\n    redirect: '/nowhere-in-table',\n  },`,
  qAlias: `  {\n    path: 'legacy-query',\n    redirect: '/home?tab=x',\n  },`,
};

/** 由块名列表拼出可被 buildRouterTable 解析的最小 router 源文本 */
function routerFixture(blockNames) {
  return [`${ROUTES_START} = [`, ...blockNames.map(n => FIXTURE_BLOCKS[n]), ROUTES_END].join('\n');
}

const FIXTURE_OK_BLOCKS = ['rootAlias', 'login', 'home', 'pureAlias', 'selfGated', 'covered', 'gapArray'];

const FIXTURE_MODULES = [
  `export const TRAVERSAL_MODULES: TraversalModule[] = [`,
  `  { id: 'home', route: '/home', domain: 'x', tier: 'C', noCreate: true },`,
  `  { id: 'covered-by-snapshot', route: '/covered-by-snapshot', domain: 'x', tier: 'C' },`,
  `  {`,
  `    id: 'gap-array',`,
  `    route: '/gap-array',`,
  `    domain: 'x',`,
  `    tier: 'C',`,
  `  },`,
  `];`,
].join('\n');

const FIXTURE_SNAPSHOT = [
  `export const ROUTE_PERMISSIONS: Record<string, RoutePermValue> = {`,
  `  // 人工评审快照夹具`,
  `  'covered-by-snapshot': 'users:read',`,
  `};`,
].join('\n');

/**
 * 夹具走真实代码路径（buildRouterTable→computeGapEntries→diffEntries→verdict），
 * 不落仓库文件。断言族：
 * 正例——别名继承必入差集；差集路由 perm 值变化必以"基线外新增 1"现形；
 * 基线缺一条必须判红（即检测力①，伪造门控指纹注入基线外的对账方向）；
 * 别名目标缺失必须抛错而不是静默按登录可达降级；快照重复键必须抛错。
 * 反例——无门控路由不误伤；快照已覆盖路由不误伤；基线全等时新增必为 0（检测力③的夹具版）。
 */
function runSelfTest() {
  const checks = [];
  const push = (name, fn) => checks.push({ name, fn });

  const snapshot = parseRoutePermSnapshot(FIXTURE_SNAPSHOT);
  const fixtureModules = parseTraversalModules(FIXTURE_MODULES);

  push('夹具装配：modules 解析出 3 条、快照解析出 1 键', () => {
    assert(fixtureModules.length === 3, `modules 应 3 条，实际 ${fixtureModules.length}`);
    assert(Object.keys(snapshot).length === 1, `快照应 1 键，实际 ${Object.keys(snapshot).length}`);
  });

  const tableOk = buildRouterTable(routerFixture(FIXTURE_OK_BLOCKS));
  const gap = computeGapEntries(tableOk, snapshot, fixtureModules);
  const gapRoutes = gap.map(e => e.route).sort();
  push('正例：差集恰为 /、/gap-array、/legacy-entry、/self-gated、/home（/login 与已覆盖者不误伤）', () => {
    assert(
      JSON.stringify(gapRoutes) ===
        JSON.stringify(['/gap-array', '/home', '/legacy-entry', '/self-gated', '/'].sort()),
      `差集集合异常：${JSON.stringify(gapRoutes)}`
    );
  });
  push('正例：纯别名 /legacy-entry 与根别名 / 以继承值入差集且标 viaAlias', () => {
    const alias = gap.find(e => e.route === '/legacy-entry');
    assert(alias && alias.perm === 'dashboard:read' && alias.viaAlias, JSON.stringify(alias));
    const root = gap.find(e => e.route === '/');
    assert(root && root.perm === 'dashboard:read' && root.viaAlias, JSON.stringify(root));
  });
  push('正例：带 component 的 redirect（/self-gated）不继承、按本块 meta 计', () => {
    const e = gap.find(x => x.route === '/self-gated');
    assert(e && e.perm === 'finance:read' && !e.viaAlias, JSON.stringify(e));
  });
  push('正例：数组门控逐值入指纹（gap-array 值为数组而非拼串）', () => {
    const e = gap.find(x => x.route === '/gap-array');
    assert(Array.isArray(e.perm) && e.perm.join(',') === 'a:read,b:read', JSON.stringify(e));
  });
  push('正例：id 取数——已进清单用模块 id，未进清单用路径 slug（根路径为 index）', () => {
    const home = gap.find(e => e.route === '/home');
    assert(home.id === 'home' && home.inList, JSON.stringify(home));
    const arr = gap.find(e => e.route === '/gap-array');
    assert(arr.id === 'gap-array' && arr.inList, JSON.stringify(arr));
    const legacy = gap.find(e => e.route === '/legacy-entry');
    assert(legacy.id === 'legacy-entry' && !legacy.inList, JSON.stringify(legacy));
    const root = gap.find(e => e.route === '/');
    assert(root.id === 'index' && !root.inList, JSON.stringify(root));
  });
  push('正例：别名目标带 query（/home?tab=x）剥离后继承目标门控', () => {
    const t = buildRouterTable(routerFixture(['home', 'qAlias']));
    const eff = parseRouterPermissions(t);
    assert(eff['/legacy-query'] === 'dashboard:read', JSON.stringify(eff['/legacy-query']));
  });

  push('正例：数组尾随逗号、空数组与 null 值均合文法', () => {
    const s = parseRoutePermSnapshot(
      `export const ROUTE_PERMISSIONS: Record<string, RoutePermValue> = {\n  a: ['x:read',],\n  b: [],\n  c: null,\n};`
    );
    assert(
      JSON.stringify(s.a) === '["x:read"]' && Array.isArray(s.b) && s.b.length === 0 && s.c === null,
      JSON.stringify(s)
    );
  });

  // 检测力①：基线外注入一条伪造门控路由指纹 → diff 必 added≥1 且 verdict 判红
  push('检测力①：基线内删掉一条指纹（等价伪造指纹注入基线外），必须 added=1 判红', () => {
    const base = gap.filter(e => e.route !== '/legacy-entry').map(stripMarks);
    const d = diffEntries(gap, base);
    assert(d.added.length === 1 && d.added[0].route === '/legacy-entry', JSON.stringify(d.added));
    assert(d.removed.length === 0, JSON.stringify(d.removed));
    assert(verdict(d.added, d.removed).red, 'verdict 必须判红');
  });

  // 检测力③（夹具版）：基线与差集全等 → added=0 removed=0 判绿
  push('检测力③：基线与当前差集全等时，新增与消失必须双 0', () => {
    const base = gap.map(stripMarks);
    const d = diffEntries(gap, base);
    assert(d.added.length === 0 && d.removed.length === 0, JSON.stringify(d));
    assert(!verdict(d.added, d.removed).red, 'verdict 必须判绿');
  });

  // 值漂移：同一 (id, route) 的 perm 变化必须现形为 新增1+消失1
  push('正例：差集路由门控值漂移（read→write）必须报"基线外新增 1"且提示删行', () => {
    const base = gap.map(e =>
      e.route === '/home' ? { id: e.id, route: e.route, perm: 'dashboard:write' } : stripMarks(e)
    );
    const d = diffEntries(gap, base);
    assert(d.added.length === 1 && d.added[0].route === '/home', JSON.stringify(d.added));
    assert(d.removed.length === 1 && d.removed[0].perm === 'dashboard:write', JSON.stringify(d.removed));
  });

  // removed-only：只清存量债不判红（只拦新增语义）
  push('反例：基线内条目已消失（removed-only）只提示删行、绝不判红', () => {
    const cur = gap.filter(e => e.route !== '/home').map(stripMarks);
    const base = gap.map(stripMarks);
    const d = diffEntries(cur, base);
    assert(d.added.length === 0, JSON.stringify(d.added));
    assert(d.removed.length === 1 && d.removed[0].route === '/home', JSON.stringify(d.removed));
    assert(!verdict(d.added, d.removed).red, 'removed-only 必须不判红');
  });

  // 指纹无行号：序列化只由三元组决定，重复计算必须逐字一致
  push('反例：指纹仅由 (id, route, perm) 决定，两次独立计算逐字一致', () => {
    const again = computeGapEntries(
      buildRouterTable(routerFixture(FIXTURE_OK_BLOCKS)),
      snapshot,
      fixtureModules
    );
    assert(
      JSON.stringify(again.map(fingerprint)) === JSON.stringify(gap.map(fingerprint)),
      '指纹不稳定'
    );
  });

  // 抛错族：别名目标缺失 / 路由边界缺失 / 快照重复键 / 清单空解析
  push('抛错：别名目标不在表中（禁止静默按登录可达降级）', () => {
    const broken = buildRouterTable(routerFixture([...FIXTURE_OK_BLOCKS, 'aliasToMissing']));
    expectThrow(() => parseRouterPermissions(broken), /不在 router 已解析路由表中/);
  });
  push('抛错：缺少 routes 边界标记', () => {
    expectThrow(() => buildRouterTable('const nope = [];'), /无法在 src\/router\/index\.ts 中定位/);
  });
  push('抛错：ROUTE_PERMISSIONS 重复键', () => {
    expectThrow(
      () =>
        parseRoutePermSnapshot(
          `export const ROUTE_PERMISSIONS: Record<string, RoutePermValue> = {\n  dup: 'a:read',\n  dup: 'b:read',\n};`
        ),
      /重复键/
    );
  });
  push('抛错：modules 源解析不到任何登记条目（判空即判红）', () => {
    expectThrow(() => parseTraversalModules('export const X = [];'), /未解析到任何/);
  });

  // 扫描面地板：真仓库解析必须扫出非零路由与差集，空操作一律判自证失败
  push('地板：真实三份输入的扫描数必须 > 0（防空操作假绿）', () => {
    const real = collectReal();
    assert(real.counts.paths > 100, `router 路由数异常低：${real.counts.paths}`);
    assert(real.counts.gap > 0, `差集为空说明快照已全覆盖或解析空转：${JSON.stringify(real.counts)}`);
    assert(real.counts.snapshotKeys === 29, `快照键数与人工表 29 键口径不符：${real.counts.snapshotKeys}`);
  });

  let pass = 0;
  let fail = 0;
  console.log('=== self-test：check-route-perm-ratchet 差集棘轮双向自证 ===');
  for (const c of checks) {
    try {
      c.fn();
      pass++;
      console.log(`  PASS  ${c.name}`);
    } catch (err) {
      fail++;
      console.error(`  FAIL  ${c.name} -> ${err.message}`);
    }
  }
  console.log(`---- self-test 结论：${pass} 通过 / ${fail} 失败 ----`);
  if (fail) {
    console.error('[route-perm-ratchet] ✗ 判定器自证失败：差集/对账存在漏报或误伤，门禁不可信');
    process.exit(1);
  }
  console.log('[route-perm-ratchet] ✅ 自证通过（基线外新增必抓、全等不误伤、removed-only 不判红）');
  process.exit(0);
}

/** 差集诊断标记不进指纹与基线：写基线/对账前统一剥掉 */
function stripMarks(e) {
  return { id: e.id, route: e.route, perm: e.perm };
}

function assert(cond, msg) {
  if (!cond) throw new Error(msg);
}

function expectThrow(fn, re) {
  let thrown = null;
  try {
    fn();
  } catch (err) {
    thrown = err;
  }
  assert(thrown !== null, '期望抛错判红但实际静默通过');
  assert(re.test(thrown.message), `抛错文案不符预期（${re}）：${thrown.message}`);
}

// ==================== 主流程 ====================

function main() {
  const args = process.argv.slice(2);
  const ALLOWED = new Set(['--regen', '--self-test']);
  const unknown = args.filter(a => !ALLOWED.has(a));
  if (unknown.length) {
    console.error(
      `[route-perm-ratchet] ✗ 未知参数 ${unknown.join(' ')} —— 只接受无参数(全量对账)、--regen 或 --self-test；已拒绝执行。`
    );
    process.exit(1);
  }
  if (args.includes('--self-test')) {
    runSelfTest();
    return;
  }

  const { gap, counts, table } = collectReal();
  console.log(`[route-perm-ratchet] ${fmtCounters({ ...counts, pathHits: table.pathHits })}`);

  if (args.includes('--regen')) {
    const payload = { total: gap.length, entries: gap.map(stripMarks) };
    fs.writeFileSync(BASELINE, JSON.stringify(payload, null, 2) + '\n', 'utf8');
    console.log(
      `[route-perm-ratchet] 基线已写入 ${path.relative(FRONTEND, BASELINE).replace(/\\/g, '/')}：${gap.length} 条指纹`
    );
    process.exit(0);
  }

  if (!fs.existsSync(BASELINE)) {
    console.error(
      `[route-perm-ratchet] ✗ 缺基线 scripts/route-perm-coverage-baseline.json（唯一合法来源是本脚本 --regen 实跑产物）`
    );
    process.exit(1);
  }
  let baseline;
  try {
    baseline = JSON.parse(fs.readFileSync(BASELINE, 'utf8'));
  } catch (err) {
    console.error(`[route-perm-ratchet] ✗ 基线 JSON 解析失败：${err.message}`);
    process.exit(1);
  }
  if (!Array.isArray(baseline.entries) || baseline.total !== baseline.entries.length) {
    console.error(
      `[route-perm-ratchet] ✗ 基线 schema 不符（需 {total, entries[]} 且 total=entries.length）——判红交人工，禁止按空基线放行`
    );
    process.exit(1);
  }

  const { added, removed } = diffEntries(gap, baseline.entries);
  console.log(
    `[route-perm-ratchet] 基线 ${baseline.entries.length} 条｜基线外新增 ${added.length}｜已消失待删 ${removed.length}`
  );
  const v = verdict(added, removed);
  for (const line of v.lines) console.log(line);
  if (v.red) {
    console.error(
      `\n[route-perm-ratchet] ✗ 出现 ${added.length} 条基线外新增的"有门控但未进 ROUTE_PERMISSIONS 快照"路由指纹。`
    );
    console.error('处置（三选一，均须人工完成，禁止 --regen 顺手洗基线换绿）：');
    console.error('  A. 该路由已进遍历清单：人工核对门控后在 permission-model.ts 的 ROUTE_PERMISSIONS 补录键值；');
    console.error('  B. 未进清单的新页面：先在 modules.config.ts 登记，再按 A 补录快照；');
    console.error('  C. 纯别名按 workflow 先例钉锚（快照记目标路由的门控值）。');
    console.error('补录使存量下降后，另跑 --regen 缩短基线并提交。');
    process.exit(1);
  }
  if (removed.length > 0) {
    console.log(
      `[route-perm-ratchet] 提示：${removed.length} 条基线内指纹在现差集中已不存在（应删行缩短，本锁不判红），` +
        `收口时跑 node scripts/check-route-perm-ratchet.mjs --regen 并提交基线 diff。`
    );
  }
  console.log('[route-perm-ratchet] ✅ 无新增差集指纹');
  process.exit(0);
}

main();

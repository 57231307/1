/**
 * 角色可见性矩阵的取数与判据支撑（flow 专用，不改公共 flow/helpers 与 fixtures/relay）
 *
 * 功能：
 *   1. 从后端角色种子文件 role.rs 读出本项目真实播种的角色码与其 data_scope 取值；
 *   2. 从后端授权矩阵文件 permission.rs 读出每个角色码被定义授予的资源与动作对，
 *      并叠加同文件的应用外壳码常量（两处判据与生产写库链完全同源，不在本侧另抄矩阵副本）；
 *   3. 从在册资源表 init_service.rs 的 PERMISSION_RESOURCES 判定资源码是否可授予；
 *   4. 从权威端点快照 route-snapshot.txt 生成端点探针表，探针的资源段按后端
 *      extract_resource_info 的集合列表形态派生（模块前缀词表与消歧映射都从
 *      path_utils.rs 现读，不复制第二份清单）；
 *   5. 镜像后端唯一判据 matches_permission 的资源段等值加星号通配与动作段等值加星号通配，
 *      供把「该角色应否被放行」做成可复算的期望集；
 *   6. 镜像前端路由守卫的可见性判据（含粗域展开），与第 5 条分开，因为本项目里
 *      「前端菜单可见」与「后端放行」本就是两件事。
 *
 * 调用方：flow 下 200 到 203 的角色矩阵 spec。
 * 入参：仓库内源文件文本与端点快照文本，全部由本模块自己按相对 frontend 目录的路径读取。
 * 传给谁：只交给上述 spec 做期望集派生与失败信息点名，不发请求、不写库。
 * 存什么：内存里的角色清单、授权对集合、端点探针表与派生函数。
 * 存哪里：不落任何文件，也不写 e2e/.auth，矩阵的制品由各 spec 自己按需落。
 *
 * 与后端判据的一致性靠断言而不是靠注释声明：
 *   spec 会对「快照里存在但本模块派生不出资源段的在册列表端点」点名，
 *   并对「角色种子读出的角色数为零」与「授权矩阵读出的角色码数为零」直接抛错，
 *   使任何一个提取器失效时矩阵立即判红，而不是退化成空集通过。
 */
import { existsSync, readFileSync } from 'fs';

/** 相对 frontend 工作目录的源文件路径，与既有 traversal 模块同一口径 */
const ROLE_SEED_SRC = '../backend/src/services/init_service_ops/role.rs';
const PERMISSION_MATRIX_SRC = '../backend/src/services/init_service_ops/permission.rs';
const INIT_SERVICE_SRC = '../backend/src/services/init_service.rs';
const PATH_UTILS_SRC = '../backend/src/utils/path_utils.rs';
const ROUTE_SNAPSHOT_SRC = 'scripts/route-snapshot.txt';
const ROUTER_SRC = 'src/router/index.ts';

function readRepoSource(relPath: string, subject: string): string {
  if (!existsSync(relPath)) {
    throw new Error(
      `${subject} 的源文件不存在：${relPath}（相对 Playwright worker 工作目录 frontend）` +
        `——派生侧无输入，矩阵无法执行，判红而非按空集放行`
    );
  }
  return readFileSync(relPath, 'utf-8').replace(/\r\n/g, '\n');
}

/** 一个角色的授权定义：资源与动作对，外加从种子读到的行级数据范围 */
export interface RoleSeedEntry {
  code: string;
  name: string;
  dataScope: string;
}

export interface PermPairWire {
  resource: string;
  action: string;
}

/** 端点探针：一条集合列表 GET 与它按后端派生规则落到的权限资源段 */
export interface EndpointProbe {
  resource: string;
  path: string;
  method: 'GET';
  /**
   * 该端点在 RBAC 之外还有一道 handler 层角色门，值为通过该门所需的角色码集合。
   * 判据来源不是推测，而是 handler 源文件的明文（例如审计日志查询要求 admin 或 auditor，
   * 见 backend src handlers audit_log_handler.rs 的 require_admin_role）。
   * 有此字段的探针，期望值取 RBAC 门与该角色门的合取，缺省即只有 RBAC 一道门。
   */
  handlerRoleGate?: string[];
}

// ==================== 1 角色种子与 data_scope ====================

/**
 * 读出后端播种的全部角色码及其 data_scope。
 * 判据形态：admin 由建角分支单独写死 all，其余角色来自三处元组表的第四项。
 * 读不到任何角色即抛错，绝不返回空数组让「角色底数为零」被当成通过。
 */
export function readSeededRoles(): RoleSeedEntry[] {
  const src = readRepoSource(ROLE_SEED_SRC, '角色种子');
  const entries: RoleSeedEntry[] = [];
  const tupleRe = /\(\s*"([^"]+)",\s*"([^"]+)",\s*"([^"]+)",\s*"([^"]+)"\s*,?\s*\)/g;
  let m: RegExpExecArray | null;
  while ((m = tupleRe.exec(src)) !== null) {
    entries.push({ name: m[1], code: m[2], dataScope: m[4] });
  }
  // admin 与 manager 与 operator 不写在元组里，各由一段 role::ActiveModel 字面量建行：
  // 按字面量块取同块内的 code 与 data_scope，两把键在块内配对，不做跨行顺序猜测。
  // 锚点只认「行首缩进后的字面量」与「let 变量等于字面量」两种形态，
  // 函数返回类型位置那个同词形态（fn 括号右箭头 role::ActiveModel 左花括号）必须排除，
  // 否则同一角色的字面量会被函数体与内层块各数一次，把 42 个数成 44 个。
  const blockRe = /= role::ActiveModel \{|\n[ \t]*role::ActiveModel \{/g;
  let bm: RegExpExecArray | null;
  while ((bm = blockRe.exec(src)) !== null) {
    const block = sliceBalancedBraces(src, src.indexOf('{', bm.index));
    const codeLiteral = /code:\s*Set\(\s*"([a-z0-9_]+)"/.exec(block)?.[1];
    const codeIsAdminConstant = /code:\s*Set\(\s*ADMIN_ROLE_CODE/.test(block);
    const scopeLiteral = /data_scope:\s*Set\(\s*"([a-z]+)"/.exec(block)?.[1];
    if (codeIsAdminConstant && !scopeLiteral) {
      throw new Error('超管建行段用了角色码常量却读不到同段 data_scope，判红');
    }
    const code = codeLiteral ?? (codeIsAdminConstant ? 'admin' : undefined);
    if (code && scopeLiteral) entries.push({ name: code, code, dataScope: scopeLiteral });
  }
  const byCode = new Map<string, RoleSeedEntry>();
  for (const e of entries) {
    if (byCode.has(e.code)) {
      throw new Error(
        `角色种子解析出重复角色码 ${e.code}（元组与建行段两处都写了它），期望集归属有歧义，判红`
      );
    }
    byCode.set(e.code, e);
  }
  const deduped = [...byCode.values()];
  if (deduped.length === 0) {
    throw new Error(
      `角色种子文件 ${ROLE_SEED_SRC} 解析出零个角色——提取口径已随源码改版失效，` +
        `此时「每个角色都测到」无从谈起，判红而非按空集通过`
    );
  }
  if (!deduped.some(e => e.code === 'admin')) {
    throw new Error('角色种子解析结果里没有 admin，建角链判据无从核对，判红');
  }
  return deduped.sort((a, b) => a.code.localeCompare(b.code));
}

/** 从开括号位置起取花括号配平的整段文本（含两端花括号），不配平即判红 */
function sliceBalancedBraces(text: string, openAt: number): string {
  if (openAt < 0 || text[openAt] !== '{') {
    throw new Error('花括号起点不是开括号，ActiveModel 字面量块无法定位，判红');
  }
  let depth = 0;
  for (let i = openAt; i < text.length; i++) {
    if (text[i] === '{') depth++;
    else if (text[i] === '}') {
      depth--;
      if (depth === 0) return text.slice(openAt, i + 1);
    }
  }
  throw new Error('ActiveModel 字面量块花括号不配平，判红');
}

// ==================== 2 授权矩阵定义 ====================

/**
 * 读出后端授权矩阵里每个角色码被授予的资源动作对，并叠加应用外壳码。
 * 叠加口径与生产写库链一致：permission.rs 在逐角色下发前把 SHELL_PERMISSIONS 补进集合，
 * 因此期望集必须同样补，否则会把每个角色都少算两枚键而误判「多项」。
 */
export function readMatrixGrants(): Map<string, PermPairWire[]> {
  const src = readRepoSource(PERMISSION_MATRIX_SRC, '授权矩阵');
  const shellBlock = src.match(/const SHELL_PERMISSIONS:\s*&\[PermPair\]\s*=\s*&\[(.*?)\];/s);
  if (!shellBlock) {
    throw new Error(`授权矩阵文件缺少 SHELL_PERMISSIONS 常量定义，外壳码期望集无从派生，判红`);
  }
  const shell: PermPairWire[] = [...shellBlock[1].matchAll(/\(\s*"([^"]+)",\s*"([^"]+)"\s*\)/g)].map(
    mm => ({ resource: mm[1], action: mm[2] })
  );
  if (shell.length === 0) {
    throw new Error('SHELL_PERMISSIONS 解析出零项，提取口径失效，判红');
  }

  const groupRe = /\(\s*\n\s*"([a-z0-9_]+)",\s*\n\s*&\[(.*?)\n\s+\],\s*\n/gs;
  const grants = new Map<string, PermPairWire[]>();
  let g: RegExpExecArray | null;
  while ((g = groupRe.exec(src)) !== null) {
    const pairs: PermPairWire[] = [
      ...g[2].matchAll(/\(\s*"([a-z0-9_\-]+)",\s*"([^"]+)"\s*\)/g),
    ].map(mm => ({ resource: mm[1], action: mm[2] }));
    for (const s of shell) {
      if (!pairs.some(p => p.resource === s.resource && p.action === s.action)) pairs.push(s);
    }
    if (grants.has(g[1])) {
      throw new Error(`授权矩阵里角色码 ${g[1]} 出现多个分组，期望集无法确定归属，判红交人工`);
    }
    grants.set(g[1], pairs);
  }
  if (grants.size === 0) {
    throw new Error(
      `授权矩阵文件 ${PERMISSION_MATRIX_SRC} 解析出零个角色分组——提取口径失效，判红而非按空集通过`
    );
  }
  return grants;
}

/** 在册资源码集合：矩阵里授予了却不在册的资源是死码，任何角色都授予不了 */
export function readRegisteredResources(): Set<string> {
  const src = readRepoSource(INIT_SERVICE_SRC, '在册资源表');
  const block = src.match(/PERMISSION_RESOURCES:\s*&\[&str\]\s*=\s*&\[(.*?)\];/s);
  if (!block) throw new Error('在册资源表 PERMISSION_RESOURCES 定义缺失，判红');
  const list = [...block[1].matchAll(/"([^"]+)"/g)].map(mm => mm[1]);
  if (list.length === 0) throw new Error('在册资源表解析出零项，提取口径失效，判红');
  return new Set(list);
}

// ==================== 3 端点探针表 ====================

function parseRustStrLiteralsWithin(text: string): string[] {
  return [...text.matchAll(/"([^"]+)"/g)].map(mm => mm[1]);
}

/** 从 path_utils.rs 现读模块前缀词表（两个 matches! 函数体），不复制第二份清单 */
function readModulePrefixes(): Set<string> {
  const src = readRepoSource(PATH_UTILS_SRC, '模块前缀词表');
  const mods = new Set<string>();
  for (const fn of ['is_system_module_prefix', 'is_business_module_prefix']) {
    const at = src.indexOf(`fn ${fn}`);
    if (at < 0) throw new Error(`path_utils.rs 中找不到 ${fn}，模块前缀词表无法派生，判红`);
    const macro = src.indexOf('matches!', at);
    if (macro < 0) throw new Error(`${fn} 体内未定位到 matches! 宏，判红`);
    const open = src.indexOf('(', macro);
    if (open < 0) throw new Error(`${fn} 的 matches! 缺少左括号，判红`);
    let depth = 0;
    let close = -1;
    for (let i = open; i < src.length; i++) {
      if (src[i] === '(') depth++;
      else if (src[i] === ')') {
        depth--;
        if (depth === 0) {
          close = i;
          break;
        }
      }
    }
    if (close < 0) throw new Error(`${fn} 的 matches! 括号不配平，判红`);
    const found = parseRustStrLiteralsWithin(src.slice(open, close));
    if (found.length === 0) throw new Error(`${fn} 的 matches! 体内解析出零个字面量，判红`);
    for (const lit of found) mods.add(lit);
  }
  if (mods.size === 0) throw new Error('模块前缀词表解析出零项，提取口径失效，判红');
  return mods;
}

/** 从 path_utils.rs 现读资源消歧映射（模块前缀加 URL 段到权限资源名） */
function readResourceNameMap(): Map<string, string> {
  const src = readRepoSource(PATH_UTILS_SRC, '资源消歧映射');
  const at = src.indexOf('RESOURCE_NAME_MAP');
  if (at < 0) throw new Error('path_utils.rs 中找不到 RESOURCE_NAME_MAP，判红');
  const tail = src.slice(at);
  const end = tail.indexOf('\n];');
  const body = tail.slice(0, end < 0 ? tail.length : end);
  const map = new Map<string, string>();
  for (const triple of body.matchAll(
    /\(\s*"([^"]+)"\s*,\s*"([^"]+)"\s*,\s*"([^"]+)"\s*,?\s*\)/g
  )) {
    map.set(`${triple[1]}|${triple[2]}`, triple[3]);
  }
  if (map.size === 0) throw new Error('资源消歧映射解析出零条，提取口径失效，判红');
  return map;
}

export interface SnapshotRow {
  method: string;
  path: string;
  handler: string;
  module: string;
}

/** 读权威端点快照（制表符四列，首行是 layers 与 endpoints 计数注释） */
export function readRouteSnapshot(): SnapshotRow[] {
  const src = readRepoSource(ROUTE_SNAPSHOT_SRC, '端点快照');
  const rows: SnapshotRow[] = [];
  for (const line of src.split('\n')) {
    if (line.trim() === '' || line.startsWith('#')) continue;
    const cols = line.split('\t');
    if (cols.length !== 4) {
      throw new Error(`端点快照行不是四列制表符结构：${line.slice(0, 120)}——快照格式已变，判红`);
    }
    rows.push({ method: cols[0], path: cols[1], handler: cols[2], module: cols[3] });
  }
  if (rows.length === 0) throw new Error('端点快照解析出零条路由，判红');
  return rows;
}

/**
 * 生成集合列表端点探针表。
 * 形态限定：路径恰为 api v1 erp 之后一段或两段，第二段不含通配与记录 id，
 * 派生资源名走后端同一条规则（第四段是模块前缀时取第五段并经消歧映射，否则取第四段），
 * 且只有落在授权矩阵实际引用过的资源名上的探针才收录，保证每条探针都有可判的期望。
 * 检测力自证由调用方 spec 负责：在册列表端点一条都派生不出来时当场判红。
 */
export function buildEndpointProbes(grantedResources: Set<string>): EndpointProbe[] {
  const mods = readModulePrefixes();
  const nameMap = readResourceNameMap();
  const probes = new Map<string, EndpointProbe>();
  for (const row of readRouteSnapshot()) {
    if (row.method !== 'GET') continue;
    const parts = row.path.split('/').filter(p => p !== '');
    if (parts.slice(0, 3).join('/') !== 'api/v1/erp') continue;
    if (parts.some(p => p === '*' || p === ':id')) continue;
    let resource: string | undefined;
    if (parts.length === 4) {
      // 后端 extract_resource_info 只在「段数不少于五且第四段是模块前缀」时才往下取第五段，
      // 四段路径的第四段恒为资源名本身（audit-logs 与 quotations 这类既是模块前缀
      // 又是资源本身的段名在此不生效），此处照抄该判据顺序，不提前套用前缀表。
      resource = parts[3];
    } else if (parts.length === 5 && mods.has(parts[3])) {
      resource = nameMap.get(`${parts[3]}|${parts[4]}`) ?? parts[4];
    }
    if (!resource || !grantedResources.has(resource)) continue;
    if (probes.has(resource)) continue;
    probes.set(resource, { resource, path: `/${parts.slice(3).join('/')}`, method: 'GET' });
  }
  // handler 层角色门的在册位点：审计日志查询在 RBAC 之外另有一道 admin 或 auditor 门
  const audit = probes.get('audit-logs');
  if (audit) audit.handlerRoleGate = ['admin', 'auditor'];
  return [...probes.values()].sort((a, b) => a.resource.localeCompare(b.resource));
}

/** 把快照里的路径拼成 apiCallExpectFail 需要的相对路径（去掉 api v1 erp 前缀段） */
export function probePathFromSnapshot(path: string): string {
  const parts = path.split('/').filter(p => p !== '');
  return `/${parts.slice(3).join('/')}`;
}

// ==================== 4 后端 RBAC 判据镜像 ====================

/**
 * 镜像后端 matches_permission 的 RBAC 判定：资源段等值或任一侧为星号，
 * 动作段等值或授权侧为星号。刻意不含前端才有的 read 与 view 等价、也不含粗域展开，
 * 因为后端没有这两条——把前端的宽容规则套到后端期望上，会把真实的 403 判成应放行。
 */
export function backendGrants(
  keys: readonly string[],
  resource: string,
  action: string
): boolean {
  return keys.some(raw => {
    const at = raw.indexOf(':');
    const res = at > 0 ? raw.slice(0, at) : raw;
    const act = at > 0 ? raw.slice(at + 1) : '';
    const resourceMatch = res === resource || res === '*' || resource === '*';
    return resourceMatch && (act === action || act === '*');
  });
}

/**
 * 后端放行面期望：持通配或该资源的读授权即应放行；带 handler 角色门的探针再与门取合取。
 * 返回期望类与判据说明，失败信息里逐条外显，便于把红归到具体一道门。
 */
export function expectedBackendClass(
  probe: EndpointProbe,
  keys: readonly string[],
  roleCode: string
): { allow: boolean; basis: string } {
  const rbac = backendGrants(keys, probe.resource, 'read');
  if (probe.handlerRoleGate) {
    const gate = probe.handlerRoleGate.includes(roleCode);
    return {
      allow: rbac && gate,
      basis: `RBAC 读=${rbac} 且 handler 角色门（需 ${probe.handlerRoleGate.join(' 或 ')}）=${gate}`,
    };
  }
  return { allow: rbac, basis: `RBAC 读=${rbac}` };
}

// ==================== 5 前端守卫判据镜像 ====================

export type RoutePermValue = string | string[] | null;

/**
 * 从 router 源文件解析每条路由的 meta.permission，口径与 traversal 的
 * parseRouterPermissions 一致（本模块不复用它，避免 flow 依赖 traversal 的内部解析）。
 */
export function parseRouterPermissions(source: string): Record<string, RoutePermValue> {
  const start = source.indexOf('const routes: RouteRecordRaw[]');
  const end = source.indexOf('createRouter(');
  if (start < 0 || end < 0 || end <= start) {
    throw new Error(
      `无法在 ${ROUTER_SRC} 定位 routes 数组边界（缺 const routes 或 createRouter 标记）` +
        `——router 结构已变，本模块解析口径需人工复核，禁止静默按空表派生`
    );
  }
  const body = source.slice(start, end);
  const hits: Array<{ raw: string; from: number; to: number }> = [];
  const pathRe = /path:\s*'([^']*)'/g;
  let m: RegExpExecArray | null;
  while ((m = pathRe.exec(body)) !== null) hits.push({ raw: m[1], from: m.index, to: pathRe.lastIndex });
  if (hits.length === 0) throw new Error('router 源文本中未解析到任何 path 项，判红交人工');

  const map: Record<string, RoutePermValue> = {};
  const aliases: Record<string, string> = {};
  for (let i = 0; i < hits.length; i++) {
    const blockEnd = i + 1 < hits.length ? hits[i + 1].from : body.length;
    const block = body.slice(hits[i].to, blockEnd);
    const fullPath = hits[i].raw.startsWith('/') ? hits[i].raw : `/${hits[i].raw}`;
    const arr = block.match(/permission:\s*\[([^\]]*)\]/);
    const perm: RoutePermValue = arr
      ? [...arr[1].matchAll(/'([^']*)'/g)].map(x => x[1])
      : block.match(/permission:\s*'([^']*)'/)?.[1] ?? null;
    if (fullPath in map) {
      if (JSON.stringify(map[fullPath]) !== JSON.stringify(perm)) {
        throw new Error(
          `路由 ${fullPath} 在 router 中出现多次且门控不一致，派生输入有歧义，判红交人工`
        );
      }
    } else {
      map[fullPath] = perm;
    }
    if (block.includes('redirect:') && !/\b(component|children):/.test(block)) {
      const lit = block.match(/redirect:\s*'([^']*)'/);
      if (lit && lit[1].startsWith('/')) aliases[fullPath] = lit[1].split(/[?#]/)[0];
    }
  }
  for (const [aliasPath, target] of Object.entries(aliases)) {
    if (target in map) map[aliasPath] = map[target];
  }
  return map;
}

/** 读 router 现值（Playwright worker 的 cwd 是 frontend，与 traversal 同口径） */
export function loadRouterPermissions(): Record<string, RoutePermValue> {
  return parseRouterPermissions(readRepoSource(ROUTER_SRC, '路由门控'));
}

/**
 * 从 navigation.ts 现读侧栏叶子与分组，得到「用户看得见的那棵树」的声明面。
 * 本模块只取 path 与 i18n 键，可见性判定仍由守卫真值加角色真实权限码决定。
 */
export function readNavLeaves(): Array<{ path: string; labelKey: string }> {
  const src = readRepoSource('src/config/navigation.ts', '侧栏导航树');
  const leaves = [...src.matchAll(/L\(\s*'([^']+)'\s*,\s*'([^']+)'\s*\)/g)].map(mm => ({
    path: mm[1],
    labelKey: mm[2],
  }));
  if (leaves.length === 0) throw new Error('侧栏导航树解析出零个叶子，提取口径失效，判红');
  const seen = new Set<string>();
  for (const leaf of leaves) {
    if (seen.has(leaf.path)) throw new Error(`侧栏导航树叶子 path ${leaf.path} 重复登记，判红`);
    seen.add(leaf.path);
  }
  return leaves;
}

/** 从 zh-CN 词表读菜单文案，供把真实渲染出的菜单项与叶子 path 一一对上 */
function readZhMenuLabels(): Record<string, string> {
  const src = readRepoSource('src/locales/zh-CN.ts', '中文词表');
  const labels: Record<string, string> = {};
  // 只取 layout.menu 段下的键值，形如 dashboard: '仪表盘',
  const at = src.indexOf('menu:');
  if (at < 0) throw new Error('中文词表缺少 layout.menu 段，菜单文案无从派生，判红');
  const tail = src.slice(at);
  const end = tail.indexOf('\n  },');
  const body = tail.slice(0, end < 0 ? tail.length : end);
  for (const mm of body.matchAll(/([A-Za-z0-9_]+):\s*'([^']*)'/g)) labels[mm[1]] = mm[2];
  if (Object.keys(labels).length === 0) throw new Error('中文词表 menu 段解析出零项，判红');
  return labels;
}

/**
 * 镜像 router/index.ts 的 hasRoutePermission 与 actionEquivalent（含粗域展开）。
 * 粗域词表从 router 源文件现读，与守卫同一份，避免两侧各持一套。
 */
function readCoarseDomainResources(source: string): Record<string, string[]> {
  const at = source.indexOf('const COARSE_DOMAIN_RESOURCES');
  if (at < 0) throw new Error('router 源文件缺少 COARSE_DOMAIN_RESOURCES，粗域展开无从派生，判红');
  const tail = source.slice(at);
  const end = tail.indexOf('\n};');
  const body = tail.slice(0, end < 0 ? tail.length : end);
  const out: Record<string, string[]> = {};
  // 域数组的收尾形态有两种：中间各域以「右方括号逗号换行」结束，最后一个域紧贴常量的
  // 「右花括号分号」收尾，切片时那段换行会被切掉，故收尾分隔符必须同时允许行尾与文本尾，
  // 否则最后一个域（本仓现值是 production）会整域丢失，粗域展开就会静默少解释一个域。
  const domainRe = /([A-Za-z0-9_\-]+):\s*\[([\s\S]*?)\],(?:\s*\n|$)/g;
  let m: RegExpExecArray | null;
  while ((m = domainRe.exec(body)) !== null) {
    out[m[1]] = [...m[2].matchAll(/'([^']+)'/g)].map(x => x[1]);
  }
  if (Object.keys(out).length === 0) throw new Error('粗域展开表解析出零个域，判红');
  for (const domain of Object.keys(out)) {
    if (out[domain].length === 0) {
      throw new Error(`粗域展开表的域 ${domain} 解析出零个细资源，收尾分隔符口径失效，判红`);
    }
  }
  return out;
}

function frontendActionEquivalent(userAction: string, requiredAction: string): boolean {
  if (userAction === '*' || userAction === requiredAction) return true;
  if (
    (userAction === 'read' && requiredAction === 'view') ||
    (userAction === 'view' && requiredAction === 'read')
  )
    return true;
  if (
    (userAction === 'update' && requiredAction === 'edit') ||
    (userAction === 'edit' && requiredAction === 'update')
  )
    return true;
  return false;
}

export function frontendRouteAllows(
  required: RoutePermValue,
  userPermissions: readonly string[],
  coarse: Record<string, string[]>
): boolean {
  if (!required) return true;
  const requiredList = Array.isArray(required) ? required : [required];
  return requiredList.some(req => {
    const sep = req.indexOf(':');
    const reqResource = sep > 0 ? req.slice(0, sep) : req;
    const reqAction = sep > 0 ? req.slice(sep + 1) : '';
    const domain = coarse[reqResource];
    return userPermissions.some(up => {
      const uSep = up.indexOf(':');
      const upResource = uSep > 0 ? up.slice(0, uSep) : up;
      const upAction = uSep > 0 ? up.slice(uSep + 1) : '';
      if (upResource === '*' && upAction === '*') return true;
      if (domain ? !domain.includes(upResource) : upResource !== reqResource) return false;
      return frontendActionEquivalent(upAction, reqAction);
    });
  });
}

/**
 * 派生某角色的侧栏可见叶子（守卫判据加 hidden 过滤，与 MainLayout 的 canAccessMenu 同一条链）。
 * hidden 判定从 router 源文本按叶子 path 所在块读 meta.hidden，读不到按 false。
 */
export function deriveVisibleLeaves(
  rolePermissions: readonly string[],
  leaves: Array<{ path: string; labelKey: string }>,
  routePerms: Record<string, RoutePermValue>,
  routerSource: string,
  coarse: Record<string, string[]>
): string[] {
  const visible: string[] = [];
  for (const leaf of leaves) {
    if (!(leaf.path in routePerms)) continue;
    if (isHiddenRoute(routerSource, leaf.path)) continue;
    if (frontendRouteAllows(routePerms[leaf.path], rolePermissions, coarse)) visible.push(leaf.path);
  }
  return visible;
}

/** 该 path 的路由块是否声明 meta.hidden */
function isHiddenRoute(routerSource: string, path: string): boolean {
  const needle = `path: '${path.startsWith('/') ? path.slice(1) : path}'`;
  const at = routerSource.indexOf(needle);
  if (at < 0) return false;
  const next = routerSource.indexOf('path:', at + needle.length);
  const block = routerSource.slice(at, next < 0 ? routerSource.length : next);
  return /hidden:\s*true/.test(block);
}

export { readZhMenuLabels, readCoarseDomainResources, ROUTER_SRC };

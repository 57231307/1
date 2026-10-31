/**
 * 全角色权限矩阵——权限预期模型（派生侧权威轨道 + 防漂移 + 基线统一 schema）
 *
 * 派生（期望集）= 两个真实来源的静态合取，绝不在测试内手写期望表当派生：
 *   1. 角色真实权限码：GET /auth/me → UserInfo.permissions（后端由 role_permission 表构建，
 *      auth_handler.rs:71-91/106-120，正是前端路由守卫消费的同一份输入；任何登录用户可读，
 *      33 个分片角色均以自身身份取码，不借 admin 会话、不撒伪权限码）；
 *   2. 路由 meta.permission：对 src/router/index.ts 源文件实时静态解析（loadRouterPermissions），
 *      路由不存在 ⇒ 该模块不可达（守卫将跳 /404，人人被拒）。
 *      纯别名路由（redirect 指向目标且自身无组件/子树）继承**目标路由**的门控：
 *      vue-router 在守卫执行前完成 redirect 解析，守卫读到的一直是 to=目标路由 的 meta；
 *      实测面 goto /workflow 落点即 /bpm 的门控判定。不跟随 redirect 链会把别名恒派生为
 * "登录即可达"，与守卫实际行为分叉。
 *      链上的 redirect 目标缺失/成环/非字面量一律抛错判红交人工，禁止静默降级。
 *   匹配语义逐一对齐 router/index.ts:1514-1602 的 splitPermissionCode / actionEquivalent /
 *   hasRoutePermission（通配 *:*、resource:*、read↔view、update↔edit）。
 *
 * 防漂移轨（assertRoutePermissionsInSync）：ROUTE_PERMISSIONS 是人工审阅过的"路由门控预期"
 * 快照（不参与派生输入）。每轮 CI 把它与 router 现值逐项比对，不一致即判红——改 router 元信息
 * 必须同步人工更新本表，防止 meta.permission 被无意改动后派生侧"跟着一起错"。
 *
 * 基线轨（每角色一个文件，读写共用 RoleBaseline 这一份 schema）：
 *   - 已评审基线：e2e/traversal/access-map-baseline/<role>.json；
 *   - 首轮无基线 → 派生/实际一致性断言之外仍判红（"无基线不算通过"纪律保留），并把候选
 *     快照写到 e2e/.auth/access-map/<role>.baseline-candidate.json（CI 制品目录已含此路径），
 *     供人工审阅后移入基线目录提交；禁止把当轮自动生成结果直接提交成基线换绿。
 *   - 有基线：现场 drift 必须为 0，且实际可达集合与基线 reachableRoutes 全等——
 *     前者防"守卫行为 vs 权威派生"分叉，后者防权限种子/路由门控回归。
 */

import type { Page } from '@playwright/test';
import { TRAVERSAL_MODULES } from './modules.config';

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';

/** 路由门控权限的取值形态（router meta.permission 支持字符串或任一命中的数组） */
export type RoutePermValue = string | string[] | null;

export interface RoleAccessEntry {
  route: string;
  /** derived=按权限码×路由 meta 推导的可达性; actual=浏览器实测（守卫跳转）结果 */
  derived: 'reachable' | 'denied';
  actual: 'reachable' | 'denied' | 'error';
  match: boolean;
}

export interface RoleAccessMap {
  role: string;
  entries: RoleAccessEntry[];
  /** 菜单收敛差异：菜单项非 <a> 锚点渲染（MainLayout 为 el-menu-item），锚点嗅探已废弃，
   *  保留字段名以兼容既有制品消费者，恒为空数组。 */
  menuDiff: { unexpected: string[]; missing: string[] };
  /** TRAVERSAL_MODULES 已登记但 router 无对应路由的模块（人人 404，配置与实现分叉信号） */
  routerMissingRoutes: string[];
  summary: { total: number; match: number; drift: number };
}

/** 基线统一 schema（读写同一份）：某角色经人工评审的可达路由全集 */
/**
 * 基线来源（可机读字段，不是注释）：
 * - backend-seeded：该岗位权限码来自后端 init 角色矩阵授权，基线可达集能证明后端授权面；
 * - ci-mirrored：该岗位在后端角色定义表与授权分组里都不存在，权限码由 e2e 侧前端手写镜像
 *   清单补建，其可达集只证明"前端镜像权限 × 前端路由守卫"自洽——**后端改授权不会让它变红**，
 *   因此它证明不了后端授权面。读一份基线即可看到这一层，无需翻测试源码。
 */
export type BaselineProvenance = 'backend-seeded' | 'ci-mirrored';

export interface RoleBaseline {
  role: string;
  reachableRoutes: string[];
  /**
   * 来源标注。判据权威是 CI_MIRRORED_ROLES 花名册，本字段是它在文件里的自证：
   * 花名册内岗位必须自证 ci-mirrored（缺标即判红），花名册外岗位不得自称 ci-mirrored。
   */
  provenance?: BaselineProvenance;
}

/**
 * CI 自造镜像岗位花名册（唯一名源）。
 *
 * 入选判据（两条同时成立，均可静态核对）：
 *   1. 角色权限矩阵 job 的分组枚举里有该码（CI 确实跑它）；
 *   2. 后端 init_service_ops 的角色定义表与授权分组都不含该码（后端既没建角也没授过权）。
 * 这些岗位的权限码由 e2e global-setup 的 SEED_ROLE_EXTRA_PERMISSIONS 补建，未在其中列出的
 * 岗位只有应用外壳两枚 SHELL 键，其基线可达集近乎只剩 /dashboard。
 *
 * 消费方（三处同源，不得各写一套）：
 *   - resolveRoleBaseline：基线来源校验与候选快照的 provenance 写入；
 *   - global-setup：补建角色时的同源点名（镜像 or 后端建角缺失）；
 *   - vitest 基线锁：花名册 ↔ 基线文件标注的对账。
 */
export const CI_MIRRORED_ROLES: readonly string[] = [
  'after_sales_manager',
  'ap_accountant',
  'ar_accountant',
  'budget_analyst',
  'color_card_manager',
  'department_manager',
  'dyeing_technician',
  'fixed_assets_accountant',
  'inventory_accountant',
  'report_viewer',
  'salesperson',
  'supplier_manager',
  'tax_accountant',
];

/** 按花名册判定某岗位的基线来源（判据只此一处，不读文件里的自报值）。 */
export function baselineProvenanceOf(role: string): BaselineProvenance {
  return CI_MIRRORED_ROLES.includes(role) ? 'ci-mirrored' : 'backend-seeded';
}

const BASELINE_DIR = 'e2e/traversal/access-map-baseline';
const CANDIDATE_DIR = 'e2e/.auth/access-map';

/**
 * 路由门控"人工审阅快照"：key = 模块 id（对应 TRAVERSAL_MODULES），
 * value = 该路由 router meta.permission 的评审期望值（null = 登录即可达）。
 * 本表不参与派生；assertRoutePermissionsInSync 每轮核它与 router 现值的一致性。
 * 值来源：按 src/router/index.ts 现值人工逐条核对录入。
 */
export const ROUTE_PERMISSIONS: Record<string, RoutePermValue> = {
  dashboard: 'dashboard:read',
  // 原 /system 聚合页拆成六个功能页，门控值沿用拆分前该入口的 users:read，不扩权限面
  users: 'users:read',
  roles: 'users:read',
  company: 'users:read',
  permissions: 'users:read',
  'field-permissions': 'users:read',
  webhooks: 'users:read',
  'system-audit-log': 'audit-logs:read',
  'system-export-approvals': 'audit-logs:read',
  'system-slow-query': 'audit-logs:read',
  finance: 'finance:read',
  // /ap 的门控已从粗域键收成九枚应付族细键（六页签入口判据的并集），
  // 值顺序与 router 源文本内的字面量数组逐项对应；出纳等只持财务域其他细读权的岗位
  // 由此不再被并集放行进入应付壳页。
  ap: [
    'ap-invoices:read',
    'ap-payments:read',
    'ap-payment-requests:read',
    'ap-verifications:read',
    'ap-reconciliations:read',
    'ap-aging:read',
    'ap-daily:read',
    'ap-monthly:read',
    'ap-statistics:read',
  ],
  // /ar 的门控已从粗域键 finance:read 收成九枚细键（六页签入口判据的并集），
  // 值顺序与 router 源文本内的字面量数组逐项对应，并与 constants/permissions.ts 的
  // AR_SHELL_ENTRY_PERMISSIONS 逐字一致（三处一致性由 tests/unit/
  // ar-shell-permission-lock.test.ts 承担）。fund-management:read 属资金账户页签，
  // 取舍理由见该常量定义处注释。只持财务域其他细读权的岗位由此不再被并集放行进入应收壳页。
  ar: [
    'ar-invoices:read',
    'ar-payments:read',
    'ar-verifications:read',
    'ar-reconciliations:read',
    'fund-management:read',
    'ar-aging:read',
    'ar-daily:read',
    'ar-monthly:read',
    'ar-statistics:read',
  ],
  customer: 'customers:read',
  supplier: 'suppliers:read',
  product: 'products:read',
  inventory: 'inventory:read',
  sales: 'sales:read',
  // 以下 5 项门控值按 router 真值钉住：若沿用 purchase:read 族旧值，会把持
  // purchases:read 的角色整片误判 denied。router 若改动这些门控，防漂移门禁会判红并要求人工复核。
  purchase: 'purchases:read',
  quality: ['quality-standards:read', 'quality-inspections:read'],
  production: ['production-orders:read'],
  // BPM 四页门控取后端真实派生键（bpm 是 URL 模块前缀，第四段经 RESOURCE_NAME_MAP 消歧）：
  // /bpm 开页需要待办列表或全局监控面之一，/bpm/approval 需要待办列表或审批执行入口之一。
  bpm: ['bpm-tasks:read', 'bpm-monitor:read'],
  'bpm-definitions': 'bpm-definitions:read',
  'bpm-templates': 'bpm-templates:read',
  'bpm-approval': ['bpm-tasks:read', 'bpm-approvals:create'],
  // 原 /crm 列表已并入 /customer，客户分级页签迁入 /crm-enhanced（同一 customers:read 门控）
  'crm-enhanced': 'customers:read',
  // 客户信用管理页的门控与其列表请求派生键同源，钉住现值防被改回粗键后派生侧跟着一起错
  'customer-credit': 'customer-credits:read',
  'sales-analysis': 'sales:read',
  // /workflow 是纯别名（router 里 `redirect: '/bpm'`），门控事实来源是
  // 目标路由 /bpm 的 meta——登记本条不是为了参与派生（派生走 redirect 链继承，见
  // parseRouterPermissions），而是把"workflow→bpm 的对应关系"钉进人工评审锚：
  // /bpm 或 /workflow 任一侧被改动（如 redirect 换目标、bpm 换权限码）都会触发防漂移判红。
  workflow: ['bpm-tasks:read', 'bpm-monitor:read'],
};

// ==================== 派生侧：权限码 × 路由 meta（均为真实来源） ====================

/**
 * 拉取当前登录角色自己的权限码（GET /auth/me → data.permissions）。
 * 这正是路由守卫（router/index.ts:1644-1649）消费的同一份后端权威数据；
 * 以角色自身会话读取，不冒充 admin、不为低权角色伪造 /roles 访问能力。
 */
export async function fetchSelfPermissions(page: Page): Promise<string[]> {
  const resp = await page.request.get(`${API_BASE}${API_PREFIX}/auth/me`, {
    headers: { 'X-Requested-With': 'XMLHttpRequest' },
  });
  if (!resp.ok()) {
    throw new Error(
      `GET /auth/me 权限码查询失败，HTTP ${resp.status()}——派生侧无输入，矩阵无法执行，判红。`
    );
  }
  const body = (await resp.json()) as { data?: { permissions?: unknown } } | null;
  const perms = body?.data?.permissions;
  // 字段缺失 ≠ 空集合：后端未返回 permissions 数组时抛错，不伪装成"该角色无任何权限"
  if (!Array.isArray(perms)) {
    throw new Error(
      `GET /auth/me 响应缺少 data.permissions 数组：keys=${JSON.stringify(
        Object.keys(body?.data ?? {})
      )}`
    );
  }
  return perms as string[];
}

const ROUTES_START = 'const routes: RouteRecordRaw[]';
const ROUTES_END = 'createRouter(';

/**
 * 从 router 源文本解析 全路由路径 → meta.permission（含 redirect 别名继承）。
 *
 * 口径：router/index.ts 为"单一 children 数组扁平登记"（MainLayout 父级 '/' 下子路径均不带
 * 前导 '/'，如 'system/audit-log'），故 path 值不带前导 '/' 时按 '/' 拼接为完整路径。
 * 每个路由对象的块位于本条 `path:` 与下一条 `path:` 之间；`permission:` 兼容
 * 字符串与数组两种写法；`redirect:` 仅当块内**不含 component/children**（纯别名）时生效，
 * 解析结果为跟随链继承目标路由的门控（见文件头口径）。
 * 解析不出边界（结构改版）、别名目标缺失/成环/非字符串字面量时抛错判红，绝不静默降级为空表
 * 或"登录即可达"。
 */
export function parseRouterPermissions(source: string): Record<string, RoutePermValue> {
  const start = source.indexOf(ROUTES_START);
  const end = source.indexOf(ROUTES_END);
  if (start < 0 || end < 0 || end <= start) {
    throw new Error(
      `无法在 src/router/index.ts 中定位 routes 数组边界（缺少 "${ROUTES_START}" / "${ROUTES_END}" 标记）` +
        `——router 结构已变，解析口径需人工复核后同步更新本文件，禁止静默放行。`
    );
  }
  const body = source.slice(start, end);
  const pathRe = /path:\s*'([^']*)'/g;
  const hits: Array<{ raw: string; from: number; to: number }> = [];
  let m: RegExpExecArray | null;
  while ((m = pathRe.exec(body)) !== null) {
    hits.push({ raw: m[1], from: m.index, to: pathRe.lastIndex });
  }
  if (hits.length === 0) {
    throw new Error('router 源文本中未解析到任何 `path:` 路由项——解析口径失效，判红交人工。');
  }

  const map: Record<string, RoutePermValue> = {};
  /** 纯别名路由：path → redirect 字面量目标（完整路径） */
  const aliases: Record<string, string> = {};
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
          )} vs ${JSON.stringify(perm)}）——派生输入有歧义，判红交人工。`
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
            `——派生输入有歧义，判红交人工。`
        );
      }
      aliases[fullPath] = alias;
    }
  }
  return inheritRedirectPermissions(map, aliases);
}

function extractBlockPermission(block: string): RoutePermValue {
  const arr = block.match(/permission:\s*\[([^\]]*)\]/);
  if (arr) {
    return [...arr[1].matchAll(/'([^']*)'/g)].map(x => x[1]);
  }
  const single = block.match(/permission:\s*'([^']*)'/);
  return single ? single[1] : null;
}

/**
 * 纯别名（path + redirect、无 component/children）的 redirect 目标提取。
 * 返回 undefined = 本块不是纯别名（无 redirect，或带自身组件的重定向由目标 route
 * 之外的渲染逻辑处理，门控以本块 meta 为准——现行 router 不存在该形态，出现时按下方
 * 抛错路径交人工复核）。
 */
function extractBlockRedirectAlias(block: string, fullPath: string): string | undefined {
  if (!block.includes('redirect:')) return undefined;
  // component/children 出现在块内 ⇒ 非纯别名（如 MainLayout 父级 '/' 的块窗口）
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

/**
 * 去掉 redirect 目标上的 query/hash：query 属目标页面的页签状态（如 ?tab=bi），
 * 不参与路由表键匹配；门控始终继承目标路径本身的 meta.permission。
 */
function stripQueryAndHash(target: string): string {
  const sep = target.search(/[?#]/);
  return sep === -1 ? target : target.slice(0, sep);
}

/**
 * redirect 链继承：别名路由的有效门控 = 链尾真实路由的 meta.permission。
 * 目标缺失 / 成环 / 链上自指一律抛错（静默按 null 派生会复现"期望可达、实测被拒"的伪红族）。
 */
function inheritRedirectPermissions(
  map: Record<string, RoutePermValue>,
  aliases: Record<string, string>
): Record<string, RoutePermValue> {
  const effective: Record<string, RoutePermValue> = { ...map };
  for (const [startPath, firstTarget] of Object.entries(aliases)) {
    const seen = new Set<string>([startPath]);
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

/** 读取并解析 router 源文件（Playwright worker 的 cwd = frontend/，与既有制品相对路径口径一致） */
export async function loadRouterPermissions(): Promise<Record<string, RoutePermValue>> {
  const fs = await import('fs');
  const routerPath = 'src/router/index.ts';
  if (!fs.existsSync(routerPath)) {
    throw new Error(
      `路由权限真实来源文件不存在：${routerPath}（相对 Playwright worker 工作目录 frontend/）` +
        `——派生侧无输入，判红。`
    );
  }
  return parseRouterPermissions(fs.readFileSync(routerPath, 'utf-8'));
}

/** 拆分权限码 `"{resource}:{action}"`（与 router/index.ts:1514-1519 同源实现） */
function splitPermissionCode(code: string): { resource: string; action: string } {
  const sepIdx = code.indexOf(':');
  return sepIdx > 0
    ? { resource: code.slice(0, sepIdx), action: code.slice(sepIdx + 1) }
    : { resource: code, action: '' };
}

/** 动作等价（与 router/index.ts:1522-1536 同源）：*:* 之外 read↔view、update↔edit */
function actionEquivalent(userAction: string, requiredAction: string): boolean {
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

/**
 * 粗域键到细资源段集，必须与 router 里 index 文件的 COARSE_DOMAIN_RESOURCES 逐字一致（同源镜像），否则派生侧与实际守卫分叉会推高 drift。仅 4 个无对应后端 resource 段的粗键需展开。
 *
 * 细名唯一名源是后端 utils/path_utils.rs 的 RESOURCE_NAME_MAP 消歧目标与
 * init_service.rs 的 PERMISSION_RESOURCES 在册资源名：财务域随域改名后 ap/ar/gl/
 * fund-transfers 这些旧段名既不在册也不可被 URL 段派生，留在本表会让派生侧把
 * "只有死码授权的角色"误判为可达，与实际守卫分叉。
 */
const COARSE_DOMAIN_RESOURCES: Record<string, string[]> = {
  finance: [
    'vouchers',
    'subjects',
    'fixed-assets',
    'budgets',
    'cost-collections',
    'financial-analysis',
    'fund-management',
    'currencies',
    'exchange-rates',
    'ar-reconciliations',
    'ar-reconciliations-enhanced',
    'accounting-periods',
    'period-adjustments',
    'period-report-snapshots',
    'aging-alert-rules',
    'aging-grades',
    'asset-categories',
    'industry-benchmarks',
    'wages',
    'reports',
    // 应付域细资源：URL 段在 ap 前缀下随域改名（发票/付款/核销/请款/对账/账龄与报表维度）
    'ap-invoices',
    'ap-payments',
    'ap-verifications',
    'ap-payment-requests',
    'ap-reconciliations',
    'ap-aging',
    'ap-daily',
    'ap-monthly',
    'ap-statistics',
    // 应收域细资源：URL 段在 ar 前缀下随域改名（收款/催收同族）
    'ar-invoices',
    'ar-payments',
    'ar-verifications',
    'ar-collections',
    'ar-aging',
    'ar-daily',
    'ar-monthly',
    'ar-statistics',
    // 总账与财务报表域细资源：URL 段在 finance 前缀下随域改名
    'finance-invoices',
    'finance-payments',
    'finance-audit',
    'finance-balance-sheet',
    'finance-cash-flow',
    'finance-general-ledger',
    'finance-income-statement',
    'finance-subsidiary-ledger',
    'finance-trial-balance',
    'finance-drill-down',
    // 根级挂载的财务族直接资源：第四段是同一资源下的查询维度或动作，URL 段即权威资源名
    'bad-debts',
    'collection-tasks',
    'finance-alerts',
    'assist-accounting',
  ],
  sales: [
    'orders',
    'quotations',
    'sales-returns',
    'sales-contracts',
    'sales-prices',
    'sales-analysis',
    'custom-orders',
    'fabric-orders',
    'customers',
    'color-cards',
    'color-prices',
    'export-inspections',
    'pieces',
    'reports',
  ],
  purchases: [
    'purchase-orders',
    'purchase-contracts',
    'purchase-prices',
    'purchase-receipts',
    'purchase-returns',
    'suppliers',
    'supplier-products',
    'supplier-qualifications',
    'supplier-evaluations',
    'supplier-product-colors',
    'sku-mappings',
    'products',
    'product-categories',
    'inventory',
    'reports',
  ],
  production: [
    'production-orders',
    'boms',
    'process-routes',
    'production-recipes',
    'dye-batches',
    'dye-recipes',
    'lab-dip',
    'greige-fabrics',
    'fabric-inspections',
    'flow-cards',
    'mrp',
    'capacity',
    'scheduling',
    'outsourcing-orders',
    'outsourcing-receipts',
    'outsourcing-vouchers',
    'material-shortage',
    'business-modes',
    'chemicals',
    'chemical-lots',
    'chemical-requisitions',
    'chemical-categories',
    'energy-meters',
    'energy-consumptions',
    'equipment',
    'maintenance-records',
    'dye-batch-lifecycle-logs',
    'dye-batch-operations',
    'dye-batch-reworks',
    'dye-batch-state-rules',
    'color-cards',
    'color-prices',
    'pieces',
    'inventory',
    'stock',
    'suppliers',
    'purchase-orders',
    'reports',
  ],
};

/**
 * 权限码匹配——逐分支镜像 router/index.ts:1585-1602 hasRoutePermission：
 * null/'' = 无 meta.permission，登录即可达；空数组 [] 在守卫里为真值且 some 恒 false（拒绝），
 * 镜像必须保留该边界语义，不得"顺手宽容"。
 */
export function hasRoutePermission(
  required: RoutePermValue,
  userPermissions: readonly string[]
): boolean {
  if (!required) return true;
  const requiredList = Array.isArray(required) ? required : [required];
  return requiredList.some(req => {
    const { resource: reqResource, action: reqAction } = splitPermissionCode(req);
    const domain = COARSE_DOMAIN_RESOURCES[reqResource];
    return userPermissions.some(up => {
      const { resource: upResource, action: upAction } = splitPermissionCode(up);
      if (upResource === '*' && upAction === '*') return true;
      if (domain ? !domain.includes(upResource) : upResource !== reqResource) return false;
      return actionEquivalent(upAction, reqAction);
    });
  });
}

/**
 * 推导角色可达路由集合（派生输入全部来自真实来源：/auth/me 权限码 + router meta）。
 */
export function deriveReachableRoutes(
  userPermissions: readonly string[],
  routePerms: Record<string, RoutePermValue>
): Map<string, boolean> {
  const result = new Map<string, boolean>();
  for (const mod of TRAVERSAL_MODULES) {
    if (!(mod.route in routePerms)) {
      // router 无此路由 ⇒ 守卫跳 /404，任何角色都不可达
      result.set(mod.id, false);
      continue;
    }
    result.set(mod.id, hasRoutePermission(routePerms[mod.route], userPermissions));
  }
  return result;
}

// ==================== 防漂移门禁：ROUTE_PERMISSIONS 快照 vs router 现值 ====================

/**
 * 每轮矩阵执行前调用。不一致即抛错判红：
 * 派生用 router 现值（真实来源），本表是人工评审锚——两者分叉说明"路由门控被改动而无人评审"，
 * 处置方向是人工核对权限策略后更新本表（并让基线轨复核），禁止反向迁就。
 */
export function assertRoutePermissionsInSync(routePerms: Record<string, RoutePermValue>): void {
  const problems: string[] = [];
  for (const [modId, declared] of Object.entries(ROUTE_PERMISSIONS)) {
    const mod = TRAVERSAL_MODULES.find(mm => mm.id === modId);
    if (!mod) {
      problems.push(`ROUTE_PERMISSIONS 键 ${modId} 已不在 TRAVERSAL_MODULES 登记（表悬空）`);
      continue;
    }
    if (!(mod.route in routePerms)) {
      problems.push(`${modId} → ${mod.route}：src/router/index.ts 中已无该路由（被删/改名）`);
      continue;
    }
    const truth = routePerms[mod.route];
    if (JSON.stringify(truth) !== JSON.stringify(declared)) {
      problems.push(
        `${modId} ${mod.route}：ROUTE_PERMISSIONS 评审值=${JSON.stringify(
          declared
        )}，router 现值=${JSON.stringify(truth)}`
      );
    }
  }
  if (problems.length > 0) {
    throw new Error(
      `路由权限防漂移门禁失败（ROUTE_PERMISSIONS 与 src/router/index.ts 分叉，共 ${problems.length} 项）：\n - ${problems.join(
        '\n - '
      )}\n处置：确认 router 门控变更符合权限策略后，人工同步 permission-model.ts 的 ROUTE_PERMISSIONS；不许改 router 迁就测试、不许删本门禁。`
    );
  }
}

// ==================== 基线轨：读写共用 RoleBaseline schema ====================

/** 当前实测的可达路由集合（drift===0 断言通过后调用，实际==派生） */
export function currentReachableRoutes(accessMap: RoleAccessMap): string[] {
  return accessMap.entries
    .filter(e => e.actual === 'reachable')
    .map(e => e.route)
    .sort();
}

export type BaselineOutcome =
  | { mode: 'missing'; baselinePath: string; candidatePath: string }
  | { mode: 'compare'; baselinePath: string; baseline: RoleBaseline };

/**
 * 读基线；首轮无基线时写出"候选"（每角色一个文件，33 分片并发零冲突）供人工审阅，
 * 由调用方继续判红——自动生成结果永不被视为通过，也不自动落入基线目录。
 */
export async function resolveRoleBaseline(accessMap: RoleAccessMap): Promise<BaselineOutcome> {
  const fs = await import('fs');
  const baselinePath = `${BASELINE_DIR}/${accessMap.role}.json`;
  if (!fs.existsSync(baselinePath)) {
    fs.mkdirSync(CANDIDATE_DIR, { recursive: true });
    const candidatePath = `${CANDIDATE_DIR}/${accessMap.role}.baseline-candidate.json`;
    const candidate: RoleBaseline = {
      role: accessMap.role,
      reachableRoutes: currentReachableRoutes(accessMap),
      provenance: baselineProvenanceOf(accessMap.role),
    };
    fs.writeFileSync(candidatePath, JSON.stringify(candidate, null, 2) + '\n', 'utf-8');
    return { mode: 'missing', baselinePath, candidatePath };
  }

  const parsed = JSON.parse(fs.readFileSync(baselinePath, 'utf-8')) as Partial<RoleBaseline>;
  // schema 不符（含旧 {roles:{...}}/扁平 Record<role,string[]> 遗留格式）不得当成"该角色无条目"
  // 放行，也不得现场重写；必须人工迁移后复核提交。
  if (parsed.role !== accessMap.role || !Array.isArray(parsed.reachableRoutes)) {
    throw new Error(
      `基线文件 ${baselinePath} schema 不符（期望 {role:"${accessMap.role}", reachableRoutes:string[]}，` +
        `实际 role=${JSON.stringify(parsed.role)}，reachableRoutes=${JSON.stringify(
          parsed.reachableRoutes
        )?.slice(
          0,
          80
        )}）——请人工按 permission-model.ts 的 RoleBaseline 定义修正后重跑，禁止脚本自动重建基线。`
    );
  }
  // 来源标注对账：权威判据是 CI_MIRRORED_ROLES 花名册，文件里的自报值必须与之一致。
  // 镜像岗位漏标 = 一份"看着像后端授权面证据、其实不是"的基线混进防回归轨，必须点名判红；
  // 非镜像岗位自称镜像 = 花名册与后端授权面漂移（后端可能已补建该岗），同样交人工复核。
  // 花名册外岗位允许不带该字段（其来源由花名册判定，无需自证），这是有意的不对称，
  // 目的是让镜像基线自己声明局限，而不是给 22 份后端授权面基线批量加注。
  const expectedProvenance = baselineProvenanceOf(accessMap.role);
  const declaredProvenance = parsed.provenance;
  if (
    declaredProvenance !== undefined &&
    declaredProvenance !== 'ci-mirrored' &&
    declaredProvenance !== 'backend-seeded'
  ) {
    throw new Error(
      `基线文件 ${baselinePath} 的 provenance 取值非法：${JSON.stringify(declaredProvenance)}` +
        `——只接受 'ci-mirrored' 或 'backend-seeded'，交人工按 permission-model.ts 的 BaselineProvenance 修正。`
    );
  }
  if (expectedProvenance === 'ci-mirrored' && declaredProvenance !== 'ci-mirrored') {
    throw new Error(
      `基线文件 ${baselinePath} 缺少来源标注 provenance='ci-mirrored'（该岗位在 CI_MIRRORED_ROLES 花名册内，` +
        `权限码由 e2e 前端镜像清单补建，基线证明不了后端授权面，必须自证以免被当成后端授权证据使用）。`
    );
  }
  if (expectedProvenance === 'backend-seeded' && declaredProvenance === 'ci-mirrored') {
    throw new Error(
      `基线文件 ${baselinePath} 自称 ci-mirrored，但 ${accessMap.role} 不在 CI_MIRRORED_ROLES 花名册内` +
        `——花名册与后端授权面已漂移（该岗可能已由后端建角并授权），需人工复核后从花名册移除或补标。`
    );
  }
  return { mode: 'compare', baselinePath, baseline: parsed as RoleBaseline };
}

/**
 * P5.14 全角色权限矩阵——权限预期模型（派生侧权威轨道 + 防漂移 + 基线统一 schema）
 *
 * 派生（期望集）= 两个真实来源的静态合取，绝不在测试内手写期望表当派生：
 *   1. 角色真实权限码：GET /auth/me → UserInfo.permissions（后端由 role_permission 表构建，
 *      auth_handler.rs:71-91/106-120，正是前端路由守卫消费的同一份输入；任何登录用户可读，
 *      33 个分片角色均以自身身份取码，不借 admin 会话、不撒伪权限码）；
 *   2. 路由 meta.permission：对 src/router/index.ts 源文件实时静态解析（loadRouterPermissions），
 *      路由不存在 ⇒ 该模块不可达（守卫将跳 /404，人人被拒）。
 *   匹配语义逐一对齐 router/index.ts:1474-1560 的 splitPermissionCode / actionEquivalent /
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
export interface RoleBaseline {
  role: string;
  reachableRoutes: string[];
}

const BASELINE_DIR = 'e2e/traversal/access-map-baseline';
const CANDIDATE_DIR = 'e2e/.auth/access-map';

/**
 * 路由门控"人工审阅快照"：key = 模块 id（对应 TRAVERSAL_MODULES），
 * value = 该路由 router meta.permission 的评审期望值（null = 登录即可达）。
 * 本表不参与派生；assertRoutePermissionsInSync 每轮核它与 router 现值的一致性。
 * 值来源：CI #4669 判责后按 src/router/index.ts 现值人工逐条核对录入（2026-10-02）。
 */
export const ROUTE_PERMISSIONS: Record<string, RoutePermValue> = {
  dashboard: 'dashboard:read',
  system: 'users:read',
  'system-audit-log': 'audit-logs:read',
  'system-export-approvals': 'audit-logs:read',
  'system-slow-query': 'audit-logs:read',
  finance: 'finance:read',
  ap: 'finance:read',
  ar: 'finance:read',
  customer: 'customers:read',
  supplier: 'suppliers:read',
  product: 'products:read',
  inventory: 'inventory:read',
  sales: 'sales:read',
  // 以下 5 项旧表值（purchase:read/quality:read/production:read/bpm:read/crm:read）与
  // router 实际门控不符，属陈旧期望；照旧值派生会把持 purchases:read 的角色整片误判 denied。
  // 现按 router 真值钉住：router 若改动这些门控，防漂移门禁会判红并要求人工复核。
  purchase: 'purchases:read',
  quality: 'inventory:read',
  production: 'inventory:read',
  bpm: 'audit-logs:read',
  crm: 'customers:read',
};

// ==================== 派生侧：权限码 × 路由 meta（均为真实来源） ====================

/**
 * 拉取当前登录角色自己的权限码（GET /auth/me → data.permissions）。
 * 这正是路由守卫（router/index.ts:1606-1614）消费的同一份后端权威数据；
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
 * 从 router 源文本解析 全路由路径 → meta.permission。
 *
 * 口径：router/index.ts 为"单一 children 数组扁平登记"（MainLayout 父级 '/' 下子路径均不带
 * 前导 '/'，如 'system/audit-log'），故 path 值不带前导 '/' 时按 '/' 拼接为完整路径。
 * 每个路由对象的 meta 块位于本条 `path:` 与下一条 `path:` 之间；`permission:` 兼容
 * 字符串与数组两种写法；解析不出边界（结构改版）时抛错判红，绝不静默降级为空表。
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
  }
  return map;
}

function extractBlockPermission(block: string): RoutePermValue {
  const arr = block.match(/permission:\s*\[([^\]]*)\]/);
  if (arr) {
    return [...arr[1].matchAll(/'([^']*)'/g)].map(x => x[1]);
  }
  const single = block.match(/permission:\s*'([^']*)'/);
  return single ? single[1] : null;
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

/** 拆分权限码 `"{resource}:{action}"`（与 router/index.ts:1474-1480 同源实现） */
function splitPermissionCode(code: string): { resource: string; action: string } {
  const sepIdx = code.indexOf(':');
  return sepIdx > 0
    ? { resource: code.slice(0, sepIdx), action: code.slice(sepIdx + 1) }
    : { resource: code, action: '' };
}

/** 动作等价（与 router/index.ts:1482-1495 同源）：*:* 之外 read↔view、update↔edit */
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
 * 权限码匹配——逐分支镜像 router/index.ts:1545-1560 hasRoutePermission：
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
    return userPermissions.some(up => {
      const { resource: upResource, action: upAction } = splitPermissionCode(up);
      if (upResource === '*' && upAction === '*') return true;
      if (upResource !== reqResource) return false;
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
  return { mode: 'compare', baselinePath, baseline: parsed as RoleBaseline };
}

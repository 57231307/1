/**
 * P5.14 全角色权限矩阵——权限预期模型（双轨）
 *
 * 轨道 1 结构推导：角色 permissions（GET /roles/{id}）× 路由 meta.permission
 *           → 预期可达路由集合。前端权限判定语义：meta.permission 为空 = 所有
 *           登录用户可达；非空 = 用户 permissions 含该权限（或含通配 `*`/`*:*`）才可达。
 * 轨道 2 黄金基线：首轮全角色遍历生成的 access-map 快照（access-map-baseline.json），
 *           人工审核合理性后固化；后续 CI 对比基线，漂移即 fail（防权限回归）。
 *
 * 推导与基线冲突时：fail 并输出 diff 提示人工复核（动态菜单场景由基线兜底）。
 */

import { TRAVERSAL_MODULES } from './modules.config';

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';

export interface RoutePermission {
  route: string;
  permission: string | null;
}

export interface RoleAccessEntry {
  route: string;
  /** derived=推导可达; actual=实际访问结果 */
  derived: 'reachable' | 'denied';
  actual: 'reachable' | 'denied' | 'error';
  match: boolean;
}

export interface RoleAccessMap {
  role: string;
  entries: RoleAccessEntry[];
  /** 菜单收敛差异：实际渲染侧边栏项 vs 预期可达集合 */
  menuDiff: { unexpected: string[]; missing: string[] };
  summary: { total: number; match: number; drift: number };
}

/**
 * 路由 meta.permission 清单（从 router/index.ts 静态提取）
 * key = 模块 id（对应 TRAVERSAL_MODULES），value = meta.permission 值（null = 登录即可达）
 */
export const ROUTE_PERMISSIONS: Record<string, string | null> = {
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
  purchase: 'purchase:read',
  quality: 'quality:read',
  production: 'production:read',
  bpm: 'bpm:read',
  crm: 'crm:read',
  // 未在 router 中显式标注 permission 的模块默认登录可达（null）
};

/**
 * 通配符权限判定：用户 permissions 含 `*`、`*:*`、或资源前缀通配（如 `users:*`）
 */
export function hasPermission(userPermissions: string[], required: string | null): boolean {
  if (required === null) return true;
  return userPermissions.some(
    (p) =>
      p === '*' ||
      p === '*:*' ||
      p === required ||
      // `users:*` 覆盖 `users:read`
      (p.endsWith(':*') && required.startsWith(p.slice(0, -1))),
  );
}

/**
 * 拉取角色权限列表（GET /roles/{id} → permissions 数组）
 * 角色列表接口若不返回 permissions 明细，回退用 GET /roles 顶层字段
 */
export async function fetchRolePermissions(
  roleId: number,
  authHeaders: Record<string, string>,
): Promise<string[]> {
  const resp = await fetch(`${API_BASE}${API_PREFIX}/roles/${roleId}`, {
    headers: authHeaders,
  }).catch((e) => { console.warn(`[E2E] 操作失败（降级跳过）: ${(e as Error).message}`); return null; });
  if (!resp || !resp.ok) return [];
  const body = (await resp.json().catch((e) => { console.warn(`[E2E] 操作失败（降级跳过）: ${(e as Error).message}`); return null; })) as
    | { data?: { permissions?: string[] } }
    | null;
  return body?.data?.permissions ?? [];
}

/**
 * 推导角色可达路由集合
 */
export function deriveReachableRoutes(
  userPermissions: string[],
): Map<string, boolean> {
  const result = new Map<string, boolean>();
  for (const mod of TRAVERSAL_MODULES) {
    const required = ROUTE_PERMISSIONS[mod.id] ?? null;
    result.set(mod.id, hasPermission(userPermissions, required));
  }
  return result;
}

/**
 * 与黄金基线对比：返回漂移项
 * 基线文件不存在时返回 null（首轮生成基线）
 */
export async function compareWithBaseline(
  accessMap: RoleAccessMap,
): Promise<RoleAccessEntry[] | null> {
  const fs = await import('fs');
  const baselinePath = 'e2e/traversal/access-map-baseline.json';
  if (!fs.existsSync(baselinePath)) return null;

  const baseline = JSON.parse(fs.readFileSync(baselinePath, 'utf-8')) as Record<
    string,
    string[]
  >;
  const baselineRoutes = new Set(baseline[accessMap.role] ?? []);

  return accessMap.entries.filter(
    (e) =>
      baselineRoutes.has(e.route) !== (e.actual === 'reachable'),
  );
}

/**
 * 写入/更新黄金基线（首轮人工审核后固化）
 */
export async function writeBaseline(accessMaps: RoleAccessMap[]): Promise<void> {
  const fs = await import('fs');
  const baselinePath = 'e2e/traversal/access-map-baseline.json';
  const baseline: Record<string, string[]> = {};
  for (const map of accessMaps) {
    baseline[map.role] = map.entries
      .filter((e) => e.actual === 'reachable')
      .map((e) => e.route);
  }
  fs.writeFileSync(baselinePath, JSON.stringify(baseline, null, 2));
}

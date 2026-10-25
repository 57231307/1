/**
 * 侧边栏导航树闭合锁（vitest，无浏览器、无后端依赖）
 *
 * 功能：断言"导航树单一真相"五条不变量——
 *   1. 集合全等：`src/config/navigation.ts` 的叶子 path 集必须与 router 里"有组件、非 hidden、
 *      非 public、非容器"的叶子路由集**双向相等**。缺项即"功能有页面却无入口"，多项即"菜单指向
 *      不存在或被标 hidden 的路由"（后者会让 canAccessMenu 恒 false，页面对所有角色不可达）。
 *   2. 唯一归属：同一 path 在树中出现且仅出现一次（杜绝同一功能挂在两个一级域下的重复入口）。
 *   3. 无第二清单：`MainLayout.vue` 源码内不得再出现逐条 `canAccessMenu('/...')` 字面清单或
 *      `visibleSubMenu` 硬编码 path 表——分组可见性必须由导航树派生，否则两份清单会漂移。
 *   4. 组件唯一：同一页面组件不得同时挂在两个可见路由上，否则同一功能会出现两条入口，
 *      且其中一条的菜单权限与另一条不一致时会出现进得去却看不到的错觉。
 *   5. 遍历覆盖棘轮：每个导航叶子都必须在 e2e 遍历清单 `e2e/traversal/modules.config.ts` 登记，
 *      未覆盖项只能出现在下方 `KNOWN_UNTRAVERSED_NAV_PATHS` 存量表里，且该表只准缩不准增。
 *      已被合并撤除的旧入口不得再登记回遍历清单，避免重定向页被遍历误判为可用入口。
 *
 * 调用方：frontend vitest 单测轨（vitest.config.ts include `tests/**`），CI ci-frontend-test job 执行。
 * 入参：只读仓库内 4 个源文件，不读写后端、不起浏览器。
 * 传给谁：断言结果只进 vitest 报告。
 * 存什么/存哪里：不落库。
 */
import { describe, expect, it } from 'vitest';
import { NAV_TREE } from '@/config/navigation';
import type { NavGroup, NavLeaf, NavNode } from '@/config/navigation';
import {
  ROUTER_SRC,
  collectShellLeaves,
  collectShellRouteEntries,
  readFrontendSource,
} from './route-source';
import type { ShellRouteEntry } from './route-source';

const MAIN_LAYOUT_SRC = readFrontendSource('src/components/Layout/MainLayout.vue');
const TRAVERSAL_SRC = readFrontendSource('e2e/traversal/modules.config.ts');

/**
 * 存量未纳入 e2e 遍历的导航叶子：历史上就没有 traversal 配置，本波不补齐，
 * 但不得再新增。补齐某个页面后必须同步从此表移除（表内残留会被下面的断言判红）。
 */
const KNOWN_UNTRAVERSED_NAV_PATHS = new Set([
  '/bad-debts',
  '/budgets',
  '/bulk-color-approval',
  '/chemicals',
  '/customer-collab',
  '/environmental-compliance',
  '/export-compliance',
  '/fabric-inspections',
  '/flow-cards',
  '/invoice-details',
  '/lab-dip',
  '/labor-contracts',
  '/occupational-health',
  '/outsourcing',
  '/period-adjustments',
  '/production-recipes',
  '/purchase/sku-mapping',
  '/purchase/supplier-product',
  '/quality-8d',
  '/social-insurance',
  '/supplier-enhanced',
  '/system-governance',
  '/system/oa-announcements',
  '/wage',
]);

/** 已被同域合并撤除的旧入口：只能以重定向存在，不得再登记为可遍历模块 */
const WITHDRAWN_LEGACY_PATHS = [
  '/fabric',
  '/crm',
  '/bi/sales-analysis',
  '/ar-reconciliation/enhanced',
  '/periods',
];

function collectTraversalRoutes(src: string): Set<string> {
  const routes = new Set<string>();
  for (const match of src.matchAll(/route:\s*'([^']+)'/g)) routes.add(match[1]);
  return routes;
}

const isLeaf = (node: NavNode): node is NavLeaf => node.kind === 'leaf';
const isGroup = (node: NavNode): node is NavGroup => node.kind === 'group';

function flatten(nodes: NavNode[]): NavLeaf[] {
  return nodes.flatMap(node =>
    isLeaf(node) ? [node] : isGroup(node) ? flatten(node.children) : []
  );
}

describe('导航树与路由表闭合', () => {
  const navLeaves = flatten(NAV_TREE);
  const navPaths = navLeaves.map(leaf => leaf.path);
  const shellEntries: ShellRouteEntry[] = collectShellRouteEntries(ROUTER_SRC);
  const routable = collectShellLeaves(ROUTER_SRC);

  it('解析器本身有效（外壳叶子数量落在合理区间，防解析塌缩成空集）', () => {
    expect(routable.size).toBeGreaterThan(100);
    expect(routable.has('/dashboard')).toBe(true);
    expect(routable.has('/login')).toBe(false);
  });

  it('导航叶子集与可路由叶子集双向相等', () => {
    const navSet = new Set(navPaths);
    expect([...routable].filter(p => !navSet.has(p)).sort()).toEqual([]);
    expect([...navSet].filter(p => !routable.has(p)).sort()).toEqual([]);
  });

  it('同一 path 不得出现两个菜单入口', () => {
    const duplicated = navPaths.filter((p, i) => navPaths.indexOf(p) !== i);
    expect(duplicated.sort()).toEqual([]);
  });

  it('同一页面组件不得挂到两个可见路由', () => {
    const byComponent = new Map<string, string[]>();
    for (const entry of shellEntries) {
      const paths = byComponent.get(entry.component) ?? [];
      paths.push(entry.path);
      byComponent.set(entry.component, paths);
    }
    const duplicated = [...byComponent.entries()]
      .filter(([, paths]) => paths.length > 1)
      .map(([component, paths]) => `${component} 同时出现在 ${paths.sort().join(' 与 ')}`);
    expect(duplicated).toEqual([]);
  });

  it('每个节点都持完整字面 i18n 键（供 check-i18n 静态判定）', () => {
    const offenders = navLeaves
      .filter(leaf => !/^layout\.menu\.[A-Za-z0-9]+$/.test(leaf.labelKey))
      .map(leaf => `${leaf.path} -> ${leaf.labelKey}`);
    expect(offenders).toEqual([]);
    const groups: NavGroup[] = NAV_TREE.filter(isGroup).flatMap(function walk(
      node: NavGroup
    ): NavGroup[] {
      return [node, ...node.children.filter(isGroup).flatMap(walk)];
    });
    expect(groups.length).toBeGreaterThan(0);
    const badGroups = groups
      .filter(g => !/^layout\.(group|section)\.[A-Za-z0-9]+$/.test(g.labelKey))
      .map(g => `${g.key} -> ${g.labelKey}`);
    expect(badGroups).toEqual([]);
  });

  it('MainLayout 不得再维护第二份 path 清单', () => {
    expect(MAIN_LAYOUT_SRC).not.toMatch(/canAccessMenu\(\s*'/);
    expect(MAIN_LAYOUT_SRC).not.toMatch(/visibleSubMenu/);
    expect(MAIN_LAYOUT_SRC).toMatch(/<NavTree/);
  });
});

describe('导航入口与 e2e 遍历清单覆盖棘轮', () => {
  const navPaths = flatten(NAV_TREE).map(leaf => leaf.path);
  const traversed = collectTraversalRoutes(TRAVERSAL_SRC);

  it('遍历清单解析器本身有效（防解析塌缩成空集）', () => {
    expect(traversed.size).toBeGreaterThan(80);
    expect(traversed.has('/customer')).toBe(true);
  });

  it('导航叶子必须登记进遍历清单，或明确留在存量未覆盖表里', () => {
    const uncovered = navPaths
      .filter(p => !traversed.has(p) && !KNOWN_UNTRAVERSED_NAV_PATHS.has(p))
      .sort();
    expect(uncovered).toEqual([]);
  });

  it('存量未覆盖表只准缩不准增（已覆盖或已下线的 path 必须移出）', () => {
    const stale = [...KNOWN_UNTRAVERSED_NAV_PATHS]
      .filter(p => traversed.has(p) || !navPaths.includes(p))
      .sort();
    expect(stale).toEqual([]);
  });

  it('同域合并后的唯一入口必须被遍历覆盖', () => {
    const mergedEntries = [
      '/customer',
      '/crm-enhanced',
      '/sales-analysis',
      '/ar-reconciliation',
      '/greige-fabrics',
      '/dye-batch',
      '/dye-recipe',
    ];
    expect(mergedEntries.filter(p => !traversed.has(p)).sort()).toEqual([]);
  });

  it('已撤除的旧入口不得重新登记为可遍历模块', () => {
    expect(WITHDRAWN_LEGACY_PATHS.filter(p => traversed.has(p)).sort()).toEqual([]);
  });
});

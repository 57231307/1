/**
 * 侧边栏导航树闭合锁（vitest，无浏览器、无后端依赖）
 *
 * 功能：断言"导航树单一真相"四条不变量——
 *   1. 集合全等：`src/config/navigation.ts` 的叶子 path 集必须与 router 里"有组件、非 hidden、
 *      非 public、非容器"的叶子路由集**双向相等**。缺项即"功能有页面却无入口"，多项即"菜单指向
 *      不存在或被标 hidden 的路由"（后者会让 canAccessMenu 恒 false，页面对所有角色不可达）。
 *   2. 唯一归属：同一 path 在树中出现且仅出现一次（杜绝同一功能挂在两个一级域下的重复入口）。
 *   3. 无第二清单：`MainLayout.vue` 源码内不得再出现逐条 `canAccessMenu('/...')` 字面清单或
 *      `visibleSubMenu` 硬编码 path 表——分组可见性必须由导航树派生，否则两份清单会漂移。
 *   4. 组件唯一：同一页面组件不得同时挂在两个可见路由上，否则同一功能会出现两条入口，
 *      且其中一条的菜单权限与另一条不一致时会出现进得去却看不到的错觉。
 *
 * 调用方：frontend vitest 单测轨（vitest.config.ts include `tests/**`），CI ci-frontend-test job 执行。
 * 入参：只读仓库内 3 个源文件，不读写后端、不起浏览器。
 * 传给谁：断言结果只进 vitest 报告。
 * 存什么/存哪里：不落库。
 */
import { describe, expect, it } from 'vitest';
import { NAV_TREE } from '@/config/navigation';
import type { NavGroup, NavLeaf, NavNode } from '@/config/navigation';
import { ROUTER_SRC, collectShellRouteEntries, readFrontendSource } from './route-source';

const MAIN_LAYOUT_SRC = readFrontendSource('src/components/Layout/MainLayout.vue');

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
  const shellEntries = collectShellRouteEntries(ROUTER_SRC);
  const routable = new Set(shellEntries.map(entry => entry.path));

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

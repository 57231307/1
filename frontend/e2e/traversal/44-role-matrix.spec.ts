import { test, expect } from '../diagnose-fixture';
import {
  loginAsRole,
  trackPageHealth,
  takePageHealth,
  assertPageHealthy,
  BROWSER_NETWORK_NOISE,
  getRoleCredential,
} from '../flow/helpers';
import { TRAVERSAL_MODULES } from './modules.config';
import {
  assertRoutePermissionsInSync,
  currentReachableRoutes,
  deriveReachableRoutes,
  fetchSelfPermissions,
  loadRouterPermissions,
  resolveRoleBaseline,
  type RoleAccessMap,
  type RoleAccessEntry,
} from './permission-model';

/**
 * P5.14 全角色权限全量矩阵
 *
 * 派生侧（期望集）= 角色真实权限码（GET /auth/me，守卫的同一输入源）
 *                 × 路由 meta.permission（src/router/index.ts 实时解析，真实来源）。
 *                 纯别名路由（redirect 且无自身组件，如 /workflow → /bpm）在解析侧继承
 *                 目标路由的门控——vue-router 在守卫前完成 redirect，实测面读到的本来就是
 *                 目标的 meta；不继承则别名恒派生"可达"、与实测分叉（CI #4671 的 31 条
 *                 /workflow 伪红族），继承口径与防漂移登记见 permission-model.ts。
 * 旧实现用侧边栏 `<a href>` 锚点嗅探当派生，而菜单实际渲染为 <el-menu-item role="menuitem">
 * （MainLayout.vue:24 起），根本没有锚点——期望集恒空、98/99 项恒"派生 denied"，
 * 属测量伪影（CI #4669 R-角色矩阵判责报告），锚点嗅探已整体移除。
 * 权限码一律来自后端真实数据，禁止在测试里手写期望表当派生、禁止给角色种子伪造权限码。
 *
 * 每角色测试流：
 * 1. 真实 UI 登录 → Dashboard 可达
 * 2. 防漂移门禁：ROUTE_PERMISSIONS 人工评审快照 vs router 现值逐项核对（分叉即判红）
 * 3. 全模块三分支断言（实测面 = 前端路由守卫，端点级鉴权由 37/39/41 + matrix-probe 负责）：
 *    - 派生可达 → 页面正常渲染（白屏/pageerror/5xx 健康断言）
 *    - 派生不可达 → 直接输 URL 被拦截（跳 /login、/403、/404）
 *    - 实际行为 ∈ 预期分支（"该拒却可达"或"该达却被拒"均 fail）
 * 4. 界面显示健康：无未翻译 key / NaN / undefined 抽样
 * 5. access-map + 首轮基线候选写入 artifacts（e2e/.auth/access-map，CI 已上传该目录）
 * 6. 基线对比（统一 RoleBaseline schema，每角色一个文件）：
 *    - 现场 drift 必须为 0（派生 vs 守卫实测一致性）；
 *    - 有基线：实际可达集合必须与基线全等（防权限种子/路由门控回归）；
 *    - 首轮无基线：写候选、加 annotation 后**判红**——"无基线不算通过"，
 *      人工审阅候选并移入 e2e/traversal/access-map-baseline/ 提交后方可转绿。
 *
 * 角色分片：CI role-permission-matrix job 按角色分组并行
 */

const role = process.env.E2E_MATRIX_ROLE || 'admin';

const API_BASE = process.env.API_BASE || 'http://localhost:8082';

test.describe(`P5.14 角色权限矩阵: ${role}`, () => {
  test('登录 + 全模块三分支断言 + access-map 生成', async ({ page }) => {
    const cred = getRoleCredential(role);
    // 角色无凭证 = global-setup ensureRoleUsers 未建该账号，属环境缺陷：
    // 矩阵必须判红，不能再 test.skip 把"根本没测"伪装成"通过"。
    if (!cred) {
      throw new Error(
        `角色 ${role} 无 E2E 凭证（getRoleCredential 返回空）——global-setup 未补建该角色账号，矩阵无法执行，判红。`
      );
    }

    const collector = trackPageHealth(page);
    await loginAsRole(page, role);

    await page.waitForURL(url => !url.pathname.includes('/login'), { timeout: 15000 });

    // ---- 派生侧权威输入（两个真实来源，均非测试内手写）----
    // 1) 该角色当前真实持有的权限码（后端由 role_permission 表构建，守卫消费的同一份数据）
    const userPermissions = await fetchSelfPermissions(page);
    // 2) 路由 meta.permission：对 router 源文件实时解析
    const routePerms = await loadRouterPermissions();
    // 防漂移门禁：人工评审快照表必须与 router 现值一致，分叉即判红交人工
    assertRoutePermissionsInSync(routePerms);

    const derived = deriveReachableRoutes(userPermissions, routePerms);

    // 配置与实现分叉信号：模块已登记但 router 无该路由（全角色 404）。
    // 可达性上与"人人被拒"一致、不混入权限漂移；作为独立缺陷清单外显（P1，交编排判责），
    // 待用户拍板"补路由 or 删登记"后可升级为判红项。
    const routerMissingRoutes = TRAVERSAL_MODULES.filter(mod => !(mod.route in routePerms)).map(
      mod => `${mod.id} ${mod.route}`
    );
    if (routerMissingRoutes.length > 0) {
      console.error(
        `[44-role-matrix] TRAVERSAL_MODULES 已登记但 router 不存在的路由（全角色不可达）：${routerMissingRoutes.join(
          '; '
        )}`
      );
      test.info().annotations.push({
        type: 'router-missing-route',
        description: `模块登记与 router 实现分叉：${routerMissingRoutes.join(
          '; '
        )}——待人工拍板补路由或删除登记（CI #4669 判责 P1）。`,
      });
    }

    const entries: RoleAccessEntry[] = [];
    const fs = await import('fs');

    for (const mod of TRAVERSAL_MODULES) {
      const expectedReachable = derived.get(mod.id) ?? true;

      if (expectedReachable) {
        // 应可达 → 访问 + 健康断言
        await page.goto(mod.route);
        await page.waitForLoadState('networkidle', { timeout: 10000 });

        let actual: 'reachable' | 'denied' = 'reachable';
        // 同一 page 内连续遍历多个模块：取增量并带模块标识，
        // 否则第 N 个模块的失败信息里混着前 N-1 个模块的错误，无法判责
        await assertPageHealthy(page, takePageHealth(collector), {
          consoleNoisePatterns: BROWSER_NETWORK_NOISE,
          label: `[${mod.id} ${mod.route}]`,
        });

        const currentPath = page.url().replace(process.env.BASE_URL || 'http://localhost:3000', '');
        if (currentPath.includes('/login') || currentPath.includes('/403')) {
          actual = 'denied';
        }

        entries.push({
          route: mod.route,
          derived: 'reachable',
          actual,
          match: actual === 'reachable',
        });
      } else {
        // 应被拒 → 直接输 URL 应被守卫拦截（跳 /login、/403 或 /404）
        await page.goto(mod.route);
        await page.waitForLoadState('networkidle', { timeout: 10000 });

        const currentPath = page.url().replace(process.env.BASE_URL || 'http://localhost:3000', '');
        const blocked =
          currentPath.includes('/login') ||
          currentPath.includes('/403') ||
          currentPath.includes('/404');

        // 旧此分支还要求"菜单无此项"（基于 <a href> 锚点嗅探）；锚点在真实 DOM 中恒不存在，
        // 该条件恒真、从未断到任何东西，随锚点嗅探一并移除。菜单可见性收敛与守卫同源
        // （MainLayout.canAccessMenu 同样走 hasRoutePermission），由守卫实测面覆盖。
        entries.push({
          route: mod.route,
          derived: 'denied',
          actual: blocked ? 'denied' : 'reachable',
          match: blocked,
        });
      }
    }

    // 界面显示健康：未翻译 key / NaN / undefined 渲染抽样
    const pageText = await page.evaluate(() => document.body.innerText);
    // 无匹配时 match 返回 null，?? [] 表示"未检出未翻译 key"（健康态），非字段缺失伪装，保留。
    const untranslatedKeys = pageText.match(/\b[a-z]+\.[a-z]+(\.[a-z]+)+\b/g) ?? [];
    const renderedNaN = /\bNaN\b|\bundefined\b/.test(pageText);

    const accessMap: RoleAccessMap = {
      role,
      entries,
      menuDiff: { unexpected: [], missing: [] },
      routerMissingRoutes,
      summary: {
        total: entries.length,
        match: entries.filter(e => e.match).length,
        drift: entries.filter(e => !e.match).length,
      },
    };

    // access-map 报告写入 artifacts（CI 已上传 e2e/.auth/access-map/）
    fs.mkdirSync('e2e/.auth/access-map', { recursive: true });
    fs.writeFileSync(
      `e2e/.auth/access-map/${role}.json`,
      JSON.stringify(
        {
          ...accessMap,
          // 本轮报告出处（provenance）：权限码与实测分别取自该 API_BASE 与 BASE_URL 会话
          apiBase: API_BASE,
          // 只记录权限码数量，不在制品里整表外显权限码清单
          permissionCount: userPermissions.length,
          untranslatedKeys: untranslatedKeys.slice(0, 10),
          renderedNaN,
        },
        null,
        2
      )
    );

    // ---- 矩阵断言（收紧假绿）----
    // 一致性主断言：派生（权限码×路由 meta）与守卫实测必须逐项一致，任何分支都执行。
    expect(
      accessMap.summary.drift,
      `角色 ${role} 派生/实测存在权限漂移（${accessMap.summary.drift}/${
        accessMap.summary.total
      } 项不一致）: ${entries
        .filter(e => !e.match)
        .map(e => `${e.route}(期望${e.derived}/实际${e.actual})`)
        .join('; ')}`
    ).toBe(0);

    // 基线轨（统一 RoleBaseline schema，每角色一个文件，33 分片并发互不覆写）：
    const baseline = await resolveRoleBaseline(accessMap);
    if (baseline.mode === 'missing') {
      test.info().annotations.push({
        type: 'baseline-missing',
        description: `角色 ${role} 首轮：候选基线已写到 ${
          baseline.candidatePath
        }（现场派生/实测漂移=${accessMap.summary.drift}，一致性主断言已通过），待人工审阅后移入 ${
          baseline.baselinePath
        } 提交。`,
      });
      throw new Error(
        `角色 ${role} 无权限矩阵基线（"无基线"不算通过）。本轮派生/实测漂移 ${
          accessMap.summary.drift
        } 项；候选基线已写入 ${baseline.candidatePath}（${
          currentReachableRoutes(accessMap).length
        } 条可达路由）。请人工核对候选与权限策略一致后，将其提交为 ${
          baseline.baselinePath
        } 再复跑；禁止把当轮自动生成结果未经评审直接入库换绿。`
      );
    }

    const currentReachable = currentReachableRoutes(accessMap);
    const baselineRoutes = [...baseline.baseline.reachableRoutes].sort();
    const baseSet = new Set(baselineRoutes);
    const curSet = new Set(currentReachable);
    const newlyReachable = currentReachable.filter(r => !baseSet.has(r));
    const newlyDenied = baselineRoutes.filter(r => !curSet.has(r));
    expect(
      newlyReachable,
      `角色 ${role} 相对已评审基线（${baseline.baselinePath}）新增可达 ${
        newlyReachable.length
      } 项：${newlyReachable.join(', ')}——权限种子/路由门控疑似回归，判红`
    ).toEqual([]);
    expect(
      newlyDenied,
      `角色 ${role} 相对已评审基线失去可达 ${newlyDenied.length} 项：${newlyDenied.join(
        ', '
      )}——权限种子/路由门控疑似回归，判红`
    ).toEqual([]);

    // 未翻译 key 与 NaN 渲染不阻塞但记录在报告中
  });
});

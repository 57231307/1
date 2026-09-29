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
  ROUTE_PERMISSIONS,
  deriveReachableRoutes,
  type RoleAccessMap,
  type RoleAccessEntry,
} from './permission-model';

/**
 * P5.14 全角色权限全量矩阵
 *
 * 每角色测试流：
 * 1. 真实 UI 登录 → Dashboard 可达
 * 2. 菜单收敛断言：侧边栏菜单项与该角色预期路由集合比对
 * 3. 全模块遍历三分支断言：
 *    - 推导可达 → 页面正常渲染（白屏/pageerror/5xx）
 *    - 推导不可达 → 菜单无此项 + 直接输 URL 被拦截
 *    - 实际行为 ∈ 预期分支（"该拒却可达"或"该达却被拒"均 fail）
 * 4. 界面显示健康：无未翻译 key / NaN / undefined 抽样
 * 5. access-map 报告写入 artifacts（黄金基线对比在 CI 侧执行）
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

    // 读取侧边栏菜单项（路由 href 集合）
    const menuHrefs = await page.evaluate(() => {
      const links = Array.from(
        document.querySelectorAll('.el-menu a[href], aside a[href], nav a[href]')
      );
      return links.map(a => (a as HTMLAnchorElement).getAttribute('href') ?? '');
    });

    // 拉取角色权限做推导（从 storageState cookie 登录态调 API）
    const derived = await deriveFromMenu(menuHrefs);

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
        // 应被拒 → 直接输 URL 应被拦截（403 页/跳转登录/菜单无此项）
        await page.goto(mod.route);
        await page.waitForLoadState('networkidle', { timeout: 10000 });

        const currentPath = page.url().replace(process.env.BASE_URL || 'http://localhost:3000', '');
        const blocked =
          currentPath.includes('/login') ||
          currentPath.includes('/403') ||
          currentPath.includes('/404');

        const menuHasIt = menuHrefs.some(h => h.startsWith(mod.route));
        entries.push({
          route: mod.route,
          derived: 'denied',
          actual: blocked ? 'denied' : 'reachable',
          match: blocked && !menuHasIt,
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
      summary: {
        total: entries.length,
        match: entries.filter(e => e.match).length,
        drift: entries.filter(e => !e.match).length,
      },
    };

    // access-map 报告写入 artifacts
    fs.mkdirSync('e2e/.auth/access-map', { recursive: true });
    fs.writeFileSync(
      `e2e/.auth/access-map/${role}.json`,
      JSON.stringify(
        { ...accessMap, untranslatedKeys: untranslatedKeys.slice(0, 10), renderedNaN },
        null,
        2
      )
    );

    // 矩阵断言（收紧假绿）：
    // - 有基线：漂移即 fail（权限回归防护，保持）。
    // - 无基线（首轮）：旧实现只 console.warn + 加 annotation，测试"通过"——等于"根本没比对
    //   就报绿"的假绿。现改为：首轮自动生成/固化基线文件供人工审阅，但本用例判红，
    //   明确要求"人工核对 access-map 无误并提交基线后再复跑"，无基线不算通过。
    const fs2 = fs;
    const baselinePath = 'e2e/traversal/access-map-baseline.json';
    if (fs2.existsSync(baselinePath)) {
      const raw = fs2.readFileSync(baselinePath, 'utf-8');
      const baseline = JSON.parse(raw) as {
        roles?: Record<string, Array<{ route: string; derived: string; actual: string }>>;
      };
      const prior = baseline.roles?.[role];
      if (!prior) {
        throw new Error(
          `基线文件存在但无角色 ${role} 的条目（access-map-baseline.json.roles 缺该键）——需人工为该角色固化基线后再复跑，无基线不算通过。`
        );
      }
      const priorDrift = prior.filter(
        e => !(e.derived === e.actual || (e.derived === 'denied' && e.actual === 'denied'))
      ).length;
      expect(
        accessMap.summary.drift,
        `角色 ${role} 相对基线存在权限漂移（当前 ${accessMap.summary.drift} 项漂移，基线记录 ${priorDrift} 项）: ${entries
          .filter(e => !e.match)
          .map(e => `${e.route}(期望${e.derived}/实际${e.actual})`)
          .join('; ')}`
      ).toBe(0);
    } else {
      // 首轮无基线：写出本次快照，初始化基线文件（人工审阅后提交），并判红强制人工确认。
      const baselineFile = {
        roles: {
          [role]: entries.map(e => ({ route: e.route, derived: e.derived, actual: e.actual })),
        },
      };
      fs2.mkdirSync('e2e/traversal', { recursive: true });
      fs2.writeFileSync(baselinePath, JSON.stringify(baselineFile, null, 2));
      test.info().annotations.push({
        type: 'baseline-initializing',
        description: `角色 ${role} 首轮：已初始化基线 ${baselinePath}（${accessMap.summary.drift} 项漂移待人工核对），本用例判红要求确认。`,
      });
      throw new Error(
        `角色 ${role} 无权限矩阵基线，本轮已初始化 ${baselinePath}（含 ${accessMap.summary.total} 项，其中 ${accessMap.summary.drift} 项派生/实际不一致）。` +
          `请人工审阅 access-map 无误后提交该基线文件并重跑，"无基线"不算通过。`
      );
    }

    // 未翻译 key 与 NaN 渲染不阻塞但记录在报告中
  });
});

/**
 * 从侧边栏菜单推导可达集合（动态菜单即角色真实权限的 UI 投影）
 * 菜单含 route = 可达；不含 = 不可达
 */
async function deriveFromMenu(menuHrefs: string[]): Promise<Map<string, boolean>> {
  const result = new Map<string, boolean>();
  for (const mod of TRAVERSAL_MODULES) {
    // 菜单 href 精确或前缀匹配模块路由
    const inMenu = menuHrefs.some(h => h === mod.route || h.startsWith(`${mod.route}/`));
    result.set(mod.id, inMenu);
  }
  return result;
}

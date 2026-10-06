/**
 * 角色×权限矩阵基线轨——契约锁与判红语义断言（vitest，无浏览器、无后端依赖）
 *
 * 功能：
 *   1. 契约锁：已评审基线目录 e2e/traversal/access-map-baseline/ 的内容**恰好**为
 *      人工已审阅入库的两份甲类基线（admin.json、e2e_noperm.json），用文件名集合相等
 *      （双向）断言，任何未评审角色基线被抢跑塞入、或已评审基线被移除，本锁判红。
 *   2. 基线文件 schema 与内容不变量：与 permission-model.ts 的 RoleBaseline 定义一致
 *      （role 与文件名同源；reachableRoutes 为字符串数组；admin 全集已排序去重；
 *      e2e_noperm 边界角色可达集为空是设计意图）。
 *   3. 基线轨两种情形在实现上分清（对 resolveRoleBaseline 直接断言）：
 *      - 有基线 → mode='compare'，调用方走"现场 drift===0 + 可达集合与基线全等"防回归轨；
 *      - 无基线 → mode='missing'，调用方必须判红（44-role-matrix.spec.ts 的哨兵 throw），
 *        且候选快照如实记录该角色现场实测可达集（差异如实报告，不静默跳过、
 *        不因"没有基线"判绿）。
 *   4. 负向断言：schema 不符的基线文件必须抛错拒收，不得被当作"无条目"放行或自动重建。
 *
 * 调用方：frontend vitest 单测轨（tests/unit 下按 vitest.config.ts 的 include  glob 收），
 * CI 的 ci-frontend-test job 执行。
 * 入参：无；只读取仓库内基线目录与 e2e 模块，不读写后端、不起浏览器。
 * 传给谁：断言结果只进 vitest 报告；基线判红语义原文见
 *          e2e/traversal/permission-model.ts resolveRoleBaseline 与
 *          e2e/traversal/44-role-matrix.spec.ts 基线轨段。
 * 存什么/存哪里：不落库；沙箱断言在系统临时目录内完成，仓库工作树零污染。
 */
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from 'fs';
import { tmpdir } from 'os';
import { join, resolve } from 'path';
import { resolveRoleBaseline, type RoleAccessMap } from '../../e2e/traversal/permission-model';

/**
 * 基线目录绝对路径（与 permission-model.ts 的 BASELINE_DIR 相对口径同源：
 * vitest/Playwright 均以 frontend/ 为工作目录运行，cwd 相对解析一致）。
 */
const BASELINE_DIR = resolve('e2e/traversal/access-map-baseline');
/** 矩阵 spec 源文件（判红传导文本锁用） */
const SPEC_PATH = resolve('e2e/traversal/44-role-matrix.spec.ts');

/**
 * 已评审入库基线全集（人工审阅后的唯一权威清单）。
 * 扩充条件：某角色候选基线经人工逐条核对权限策略后另行入库，本锁必须同步改为
 * 显式评审结果——禁止为转绿把生成器候选直接塞进这里。
 */
const APPROVED_BASELINES = ['admin.json', 'e2e_noperm.json'];

function syntheticMap(role: string, reachableRoutes: string[]): RoleAccessMap {
  const reachSet = new Set(reachableRoutes);
  const routes = ['/dashboard', '/purchase', '/inventory', ...reachableRoutes];
  const entries = [...new Set(routes)].map(route => ({
    route,
    derived: reachSet.has(route) ? ('reachable' as const) : ('denied' as const),
    actual: reachSet.has(route) ? ('reachable' as const) : ('denied' as const),
    match: true,
  }));
  return {
    role,
    entries,
    menuDiff: { unexpected: [], missing: [] },
    routerMissingRoutes: [],
    summary: {
      total: entries.length,
      match: entries.length,
      drift: entries.filter(e => !e.match).length,
    },
  };
}

describe('角色权限矩阵基线目录：契约锁', () => {
  it('基线目录存在且文件名集合恰好等于已评审入库全集（双向全等，数量相等不充分）', () => {
    expect(existsSync(BASELINE_DIR), '基线目录不存在——甲类基线丢失，判红').toBe(true);
    const actual = [...readdirSync(BASELINE_DIR)].sort();
    const expected = [...APPROVED_BASELINES].sort();
    const unexpected = actual.filter(f => !expected.includes(f));
    const lost = expected.filter(f => !actual.includes(f));
    // 集合相等 = "无未评审文件抢跑入库" 且 "已评审基线未被移除" 同时成立
    expect(
      unexpected,
      `基线目录出现未评审文件（候选基线必须人工逐条审阅后另行入库）：${unexpected.join(', ')}`
    ).toEqual([]);
    expect(
      lost,
      `已评审基线缺失：${lost.join(', ')}——移除基线等同移除防回归锚，需同等评审流程`
    ).toEqual([]);
    expect(actual).toEqual(expected);
  });

  it('甲类基线 schema 与内容不变量（RoleBaseline 定义对齐）', () => {
    for (const file of APPROVED_BASELINES) {
      const role = file.replace(/\.json$/, '');
      const parsed = JSON.parse(readFileSync(join(BASELINE_DIR, file), 'utf-8')) as {
        role: unknown;
        reachableRoutes: unknown;
      };
      expect(parsed.role, `${file}: role 字段必须与文件名同源`).toBe(role);
      expect(
        Array.isArray(parsed.reachableRoutes),
        `${file}: reachableRoutes 必须是数组（schema 不符即失格）`
      ).toBe(true);
      const routes = parsed.reachableRoutes as string[];
      routes.forEach(r => {
        expect(typeof r, `${file}: 条目必须是字符串`).toBe('string');
        expect(r.startsWith('/'), `${file}: 条目必须是完整路由路径`).toBe(true);
      });
      expect(new Set(routes).size, `${file}: 可达路由不允许重复条目`).toBe(routes.length);
      expect([...routes].sort(), `${file}: 可达路由应处于排序形态（生成口径 sort 后入库）`).toEqual(
        routes
      );
    }

    const admin = JSON.parse(readFileSync(join(BASELINE_DIR, 'admin.json'), 'utf-8') as string) as {
      reachableRoutes: string[];
    };
    // admin 注入 *:*（后端 auth_handler 口径），可达集=全模块全集：非空、含核心域入口
    expect(admin.reachableRoutes.length, 'admin 基线可达集为空与超管权限策略矛盾').toBeGreaterThan(
      0
    );
    expect(admin.reachableRoutes).toContain('/dashboard');

    // 空权限边界角色的设计意图：可达集恒为空数组
    const noperm = JSON.parse(
      readFileSync(join(BASELINE_DIR, 'e2e_noperm.json'), 'utf-8') as string
    ) as { reachableRoutes: string[] };
    expect(noperm.reachableRoutes).toEqual([]);
  });
});

describe('基线轨语义：有基线=比对防回归，无基线=如实报告并判红（不静默、不判绿）', () => {
  let sandbox: string;
  let cwdBackup: string;

  beforeAll(() => {
    // 沙箱内复刻"仓库当前基线目录原样"，resolveRoleBaseline 的相对路径口径按 cwd 解析；
    // 候选写入落在沙箱的 e2e/.auth/access-map，仓库工作树与真实候选目录零污染
    cwdBackup = process.cwd();
    sandbox = mkdtempSync(join(tmpdir(), 'role-matrix-baseline-'));
    mkdirSync(resolve(sandbox, 'e2e/traversal/access-map-baseline'), { recursive: true });
    for (const file of APPROVED_BASELINES) {
      cpSync(
        join(BASELINE_DIR, file),
        resolve(sandbox, `e2e/traversal/access-map-baseline/${file}`)
      );
    }
    process.chdir(sandbox);
  });

  afterAll(() => {
    process.chdir(cwdBackup);
    rmSync(sandbox, { recursive: true, force: true });
  });

  it('已入库角色（admin）→ mode=compare，进入"drift===0 + 可达集合全等"防回归轨', async () => {
    const outcome = await resolveRoleBaseline(
      syntheticMap('admin', ['/dashboard', '/purchase', '/inventory'])
    );
    expect(outcome.mode).toBe('compare');
    if (outcome.mode === 'compare') {
      expect(outcome.baseline.role).toBe('admin');
      expect(outcome.baseline.reachableRoutes.length).toBeGreaterThan(0);
    }
  });

  it('负向锁：未入库岗位（purchaser）现场有实测可达 → mode=missing，候选如实落盘供判红，绝不返回 compare', async () => {
    const observedReachable = ['/dashboard', '/purchase'];
    const outcome = await resolveRoleBaseline(syntheticMap('purchaser', observedReachable));
    // 调用方（44-role-matrix.spec.ts）对 mode!=='compare' 的分支是 throw 哨兵判红；
    // "missing" 就是判红信号，永不是放行信号
    expect(outcome.mode).toBe('missing');
    if (outcome.mode === 'missing') {
      // 基线路径必须指向已评审基线目录（相对 Playwright/vitest 工作目录 frontend/ 的口径）
      expect(outcome.baselinePath).toBe('e2e/traversal/access-map-baseline/purchaser.json');
      // 候选快照如实记录现场实测可达集（差异报告的真实内容，不掺派生假设）
      const candidate = JSON.parse(readFileSync(outcome.candidatePath, 'utf-8')) as {
        role: unknown;
        reachableRoutes: string[];
      };
      expect(candidate.role).toBe('purchaser');
      expect(candidate.reachableRoutes).toEqual(observedReachable);
      // 未入库角色不得"因为没有基线"而被跳过：missing 分支必须产出可读候选文件本体
      expect(existsSync(outcome.candidatePath), 'missing 分支必须写出候选快照供人工审阅').toBe(
        true
      );
    }
  });

  it('负向锁：schema 不符的基线文件抛错拒收，不被当作"无条目"放行，也不自动重建', async () => {
    const badRole = 'warehouse_manager';
    writeFileSync(
      resolve(sandbox, `e2e/traversal/access-map-baseline/${badRole}.json`),
      JSON.stringify({ role: 'someone_else', reachableRoutes: ['/dashboard'] }, null, 2) + '\n',
      'utf-8'
    );
    await expect(resolveRoleBaseline(syntheticMap(badRole, ['/dashboard']))).rejects.toThrow(
      /schema 不符/
    );
    // 抛错路径不得留下自动重建产物
    expect(
      existsSync(resolve(sandbox, `e2e/.auth/access-map/${badRole}.baseline-candidate.json`)),
      'schema 不符时必须拒收交人工，禁止顺手写候选自动重建'
    ).toBe(false);
  });
});

describe('判红传导：spec 基线轨无静默跳过通道（源码文本锁）', () => {
  it('无基线分支为 throw 哨兵判红，且全文件不存在 test.skip/test.fixme 逃生门', () => {
    const source = readFileSync(SPEC_PATH, 'utf-8');
    expect(source.includes('无权限矩阵基线'), '基线缺失哨兵文案必须存在（无基线不算通过）').toBe(
      true
    );
    expect(/test\.(skip|fixme|only)\(/.test(source), 'spec 出现 skip/fixme/only 即打开通道').toBe(
      false
    );
    // missing 分支的处置必须是 throw 而非 return/注释化
    const missingBlock = source.slice(source.indexOf("baseline.mode === 'missing'"));
    expect(missingBlock.slice(0, 1200).includes('throw new Error')).toBe(true);
    // 现场一致性主断言必须仍被硬判（drift 非 0 即红），不得被基线轨的存在稀释
    expect(source.includes('accessMap.summary.drift')).toBe(true);
  });
});

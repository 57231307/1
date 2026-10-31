/**
 * 角色授权底数与后端放行面矩阵
 *
 * 功能：
 *   1. 把「本项目真实拥有多少角色」做成可复算的底数，并对后端角色表实测结果双向全等：
 *      角色种子文件在册的每一个角色码都必须能在库里的 GET /roles 读到，
 *      库里多出的每一个角色码都必须能被 CI 镜像花名册或 global-setup 的边界与黑名单清单交代，
 *      两头对不上都点名判红，不写死一个数字当分母。
 *   2. 逐角色核进行级数据范围与授权行：库里 roles.data_scope 必须等于种子里该角色的取值，
 *      库里该角色的角色级授权行（allowed 为真且不带记录 id）集合必须与授权矩阵定义分组
 *      加上应用外壳码双向全等，缺项与多项都算红。
 *   3. 逐角色跑后端放行面矩阵：对该角色被授予的每一个资源所对应的集合列表端点，
 *      期望来自该角色实际被授予的权限键清单按后端唯一判据（资源段等值加星号通配、
 *      动作段等值加星号通配，无粗域展开、无 read 与 view 等价）派生；
 *      实测发真请求取状态码与机器码；期望放行集合与实测放行集合做双向全等。
 *
 * 与 201 的分工：本文件只判后端放行面（RBAC 与 handler 角色门），
 * 前端菜单与路由守卫可见面由 201 单独断言。二者必须分列，
 * 因为本项目里粗域展开只存在于前端守卫，前端菜单可见与后端放行本就不同集合。
 *
 * 调用方：Playwright E2E 套件，CI 的 e2e flow 分片按文件哈希收集执行。
 * 入参：API_BASE 与 BASE_URL 取环境变量；角色凭证取 env 优先、回退 role-credentials.json；
 *      期望集全部由 ./role-matrix-support 从仓库源文件现读派生，本文件不含任何手写权限清单。
 * 传给谁：会话与身份钉交给 fixtures/relay 与 fixtures/multi-context 的 real 档，
 *      请求与拒绝归因交给 flow/helpers 的 apiCallExpectFail 与 expectDenied，不另立第二套。
 * 存什么：只读断言为主；为越权负例准备的行在 203，本文件不落业务数据。
 * 存哪里：矩阵实测结果按角色写 e2e/.auth/role-visibility-matrix/<role>.json 供人工审阅，
 *      该目录不是基线、不参与判绿，判绿只看上面的双向全等断言。
 *
 * 判红即缺陷的口径：若某角色的实测放行集合与按授权定义派生的期望集合不等，
 * 那说明后端授权链或门控与该角色的应然面不符，用例保持判红并在消息里点名角色、端点、
 * 资源段、该角色实持权限键与命中的那一条键，不改期望凑绿。
 */
import { test, expect } from '../diagnose-fixture';
import { apiCallExpectFail, apiCallRaw, getRoleCredential } from './helpers';
import { readRelayIdentity, createRelay } from '../fixtures/relay';
import { createIsolatedSession, type IsolatedSession } from '../fixtures/multi-context';
import { CI_MIRRORED_ROLES } from '../traversal/permission-model';
import { readFileSync, mkdirSync, writeFileSync } from 'fs';
import {
  buildEndpointProbes,
  expectedBackendClass,
  readMatrixGrants,
  readRegisteredResources,
  readSeededRoles,
  type EndpointProbe,
} from './role-matrix-support';

interface RoleGrantRow {
  resource_type: string;
  resource_id: number | null;
  action: string;
  allowed: boolean;
}

interface RoleDetail {
  id: number;
  code: string;
  data_scope: string;
  permissions?: RoleGrantRow[] | null;
}

/** 库里 GET /roles 的行形状，只取判据需要的三字段 */
interface RoleListRow {
  id: number;
  code: string;
}

const MATRIX_ARTIFACT_DIR = 'e2e/.auth/role-visibility-matrix';

function pairKey(resource: string, action: string): string {
  return `${resource}:${action}`;
}

/**
 * 从 global-setup 源文本读边界与黑名单角色码。
 * 用途：库里出现的非种子角色必须能被这三份 CI 侧清单交代，读不到就判红，
 * 而不是把「多出来的角色」当成可忽略噪声。
 */
function readCiSideRoleCodes(): string[] {
  const src = readFileSync('e2e/global-setup.ts', 'utf-8').replace(/\r\n/g, '\n');
  const codes: string[] = [];
  for (const constName of ['BOUNDARY_ROLES', 'BLACKLIST_TEST_ROLES', 'SEED_ROLES']) {
    const at = src.indexOf(`const ${constName}`);
    if (at < 0) {
      throw new Error(`global-setup 源文本缺少 ${constName} 定义，CI 侧角色清单无从核对，判红`);
    }
    const tail = src.slice(at);
    const end = tail.indexOf('\n];');
    const body = tail.slice(0, end < 0 ? tail.length : end);
    for (const m of body.matchAll(/code:\s*'([a-z0-9_]+)'/g)) codes.push(m[1]);
    if (constName === 'SEED_ROLES') {
      for (const m of body.matchAll(/^\s*'([a-z0-9_]+)',/gm)) codes.push(m[1]);
    }
  }
  return [...new Set(codes)];
}

const seededRoles = readSeededRoles();
const matrixGrants = readMatrixGrants();
const registeredResources = readRegisteredResources();
const grantedResources = new Set<string>();
for (const pairs of matrixGrants.values()) {
  for (const p of pairs) grantedResources.add(p.resource);
}
const probes = buildEndpointProbes(grantedResources);

test.describe('角色授权底数与后端放行面矩阵', () => {
  test('底数一 角色种子在册角色与后端角色表实测双向全等', async ({ browser }) => {
    const session = await createIsolatedSession(browser, 'admin', { authMode: 'real' });
    try {
      const rows = await apiCallRaw<{ roles?: RoleListRow[] }>(session.page, 'GET', '/roles');
      expect(
        Array.isArray(rows.roles),
        `GET /roles 出参缺 roles 数组（后端契约 role_handler 的 RoleListResponse），实际键=${Object.keys(
          rows
        ).join(',')}`
      ).toBe(true);
      const dbCodes = new Set((rows.roles ?? []).map(r => r.code).filter(Boolean));
      const seedCodes = new Set(seededRoles.map(r => r.code));

      // 正向：种子在册的每一个角色都必须真的能在库里读到，缺一个就说明建角链断了
      const missingInDb = [...seedCodes].filter(c => !dbCodes.has(c));
      expect(
        missingInDb,
        `角色种子在册但库里读不到的角色码共 ${missingInDb.length} 个：${missingInDb.join(
          ', '
        )}——建角链断则授权矩阵无落点，该角色既测不了也不该被当成不存在`
      ).toEqual([]);

      // 反向：库里多出的角色码必须能被 CI 侧清单交代，交代不了的点名
      const ciSide = new Set([...readCiSideRoleCodes(), ...CI_MIRRORED_ROLES]);
      const unaccounted = [...dbCodes].filter(c => !seedCodes.has(c) && !ciSide.has(c));
      expect(
        unaccounted,
        `库里出现既不在后端角色种子、也不在 CI 镜像花名册与边界与黑名单清单的角色码共 ` +
          `${unaccounted.length} 个：${unaccounted.join(', ')}——角色底数出现无主来源，判红交人工`
      ).toEqual([]);

      // 分母自证：底数非零且与授权矩阵的分组枚举互相交代
      expect(
        seededRoles.length,
        `角色种子解析出 ${seededRoles.length} 个角色，分母为零说明提取口径失效`
      ).toBeGreaterThan(0);
      const adminSeed = seededRoles.find(r => r.code === 'admin');
      expect(adminSeed, '角色种子里读不到 admin，建角链判据无从核对').toBeTruthy();

      const matrixCodes = new Set(matrixGrants.keys());
      const seedNoMatrix = [...seedCodes].filter(c => c !== 'admin' && !matrixCodes.has(c));
      expect(
        seedNoMatrix,
        `角色种子在册但授权矩阵无分组、因而除超管旁路外零授权面的角色共 ${
          seedNoMatrix.length
        } 个：${seedNoMatrix.join(
          ', '
        )}——被指派这些角色的用户登录即停在无权限页，属真实缺陷，保持判红`
      ).toEqual([]);
      const matrixNoSeed = [...matrixCodes].filter(c => !seedCodes.has(c));
      expect(
        matrixNoSeed,
        `授权矩阵有分组但角色种子不建角、授权永远落不了地的角色码共 ${
          matrixNoSeed.length
        } 个：${matrixNoSeed.join(', ')}`
      ).toEqual([]);
    } finally {
      await session.close();
    }
  });

  test('底数二 逐角色 data_scope 与角色级授权行与矩阵定义双向全等', async ({ browser }) => {
    const adminSession = await createIsolatedSession(browser, 'admin', { authMode: 'real' });
    const mismatches: string[] = [];
    const adminWildcard: string[] = [];
    try {
      const rows = await apiCallRaw<{ roles?: RoleListRow[] }>(adminSession.page, 'GET', '/roles');
      const dbByCode = new Map((rows.roles ?? []).map(r => [r.code, r]));

      for (const seed of seededRoles) {
        const row = dbByCode.get(seed.code);
        if (!row) {
          mismatches.push(`${seed.code} 库里读不到（底数一已判红，此处跳过逐行核对）`);
          continue;
        }
        const detail = await apiCallRaw<RoleDetail>(adminSession.page, 'GET', `/roles/${row.id}`);

        // 行级数据范围逐角色核到种子里明文写的那一个取值
        expect(
          detail.data_scope,
          `角色 ${seed.code} 的库里 data_scope 应等于种子值 ${seed.dataScope}，实际=${
            detail.data_scope
          }（词表 all 与 dept 与 self，见 backend src utils data_scope.rs）`
        ).toBe(seed.dataScope);

        const granted = (detail.permissions ?? []).filter(
          p => p.allowed === true && p.resource_id === null
        );
        if (seed.code === 'admin') {
          // admin 不写在授权矩阵分组里，超管旁路认的是角色码等于 admin，/auth/me 下发通配一枚
          adminWildcard.push(...granted.map(p => pairKey(p.resource_type, p.action)));
          continue;
        }
        const defined = matrixGrants.get(seed.code);
        if (!defined) {
          mismatches.push(`${seed.code} 授权矩阵无分组，但库里角色级授权行有 ${granted.length} 条`);
          continue;
        }
        const definedSet = new Set(defined.map(p => pairKey(p.resource, p.action)));
        const actualSet = new Set(granted.map(p => pairKey(p.resource_type, p.action)));
        const missing = [...definedSet].filter(k => !actualSet.has(k));
        const extra = [...actualSet].filter(k => !definedSet.has(k));
        if (missing.length > 0 || extra.length > 0) {
          mismatches.push(
            `${seed.code} 缺授权 ${missing.length} 项（${missing.slice(0, 8).join(', ')}）` +
              `与多项 ${extra.length} 项（${extra.slice(0, 8).join(', ')}）`
          );
        }
        // 资源码必须在册：授予了却不在册的键是死码，谁都拿不到
        const unregistered = [...definedSet]
          .map(k => k.split(':')[0])
          .filter(r => !registeredResources.has(r));
        if (unregistered.length > 0) {
          mismatches.push(
            `${seed.code} 的授权定义引用了不在册资源码 ${[...new Set(unregistered)].join(', ')}`
          );
        }
      }
    } finally {
      await adminSession.close();
    }

    expect(
      mismatches,
      `逐角色的 data_scope 与角色级授权行对账不平，共 ${mismatches.length} 项：\n  ${mismatches.join(
        '\n  '
      )}`
    ).toEqual([]);

    // admin 的通配面自证：库里给 admin 写了角色级授权行即为绕开矩阵的旁路，必须点名
    expect(
      adminWildcard,
      `admin 角色不应靠 role_permission 行获得权限（超管旁路按角色码判定），实际却有 ${
        adminWildcard.length
      } 行：${adminWildcard.slice(0, 8).join(', ')}`
    ).toEqual([]);
  });

  test('底数三 端点探针表非空且已知在册资源能被派生出来（提取器检测力自证）', async () => {
    expect(
      probes.length,
      `端点探针表为空，说明资源派生或快照读取失效，后端放行面无从断言，判红`
    ).toBeGreaterThan(0);
    // 已知存在的样本：这几个资源在后端种子与快照里都真实在册，提取器必须抓到
    for (const known of ['products', 'customers', 'orders', 'purchase-orders', 'users']) {
      expect(
        probes.some(p => p.resource === known),
        `端点探针表缺已知在册资源 ${known}，提取器对该族路径失察（快照里确有该资源的集合列表端点），判红`
      ).toBe(true);
    }
    // 反向自证：探针表里的每条路径都必须在权威端点快照里以 GET 在册
    const snapshotGetPaths = new Set(
      readFileSync('scripts/route-snapshot.txt', 'utf-8')
        .split('\n')
        .filter(l => l.replace(/\r/g, '').startsWith('GET\t'))
        .map(l => l.replace(/\r/g, '').split('\t')[1])
    );
    const ghosts = probes.filter(p => !snapshotGetPaths.has(`/api/v1/erp${p.path}`));
    expect(
      ghosts,
      `探针表存在快照里不存在的 GET 路径共 ${ghosts.length} 条：${ghosts
        .map(g => `${g.resource}=${g.path}`)
        .join(', ')}`
    ).toEqual([]);
  });
});

/**
 * 逐角色的后端放行面矩阵，一角色一条 test 让失败可直接归到岗位。
 * 期望放行集与实测放行集双向全等：应放却 403 与不应放却 2xx 都是红。
 */
for (const seed of seededRoles) {
  test.describe(`后端放行面 角色 ${seed.code}`, () => {
    test(`${seed.code} 的端点放行集合与授权定义双向全等`, async ({ browser }) => {
      const cred = getRoleCredential(seed.code);
      if (!cred) {
        throw new Error(
          `角色 ${seed.code} 无 E2E 凭证（env 与 role-credentials.json 两处都无该角色码），` +
            `该角色的可见面根本没测到，判红而非 skip`
        );
      }
      let session: IsolatedSession | undefined;
      try {
        session = await createIsolatedSession(browser, seed.code, {
          authMode: 'real',
          credentialRole: seed.code,
        });
        const identity = await readRelayIdentity(session.page, `角色 ${seed.code} 的矩阵会话`);
        expect(
          identity.username,
          `角色 ${seed.code} 登录回来的账号应是凭证里的 ${cred.username}，实际=${identity.username}`
        ).toBe(cred.username);

        const keys = identity.permissions;
        const roleCode = seed.code;
        const expectedAllow: string[] = [];
        const expectedDeny: string[] = [];
        const observedAllow: string[] = [];
        const anomalies: string[] = [];

        for (const probe of probes as EndpointProbe[]) {
          const exp = expectedBackendClass(probe, keys, roleCode);
          if (exp.allow) expectedAllow.push(probe.resource);
          else expectedDeny.push(probe.resource);

          const result = await apiCallExpectFail(session!.page, 'GET', probe.path);
          if (result.status >= 200 && result.status < 400) {
            observedAllow.push(probe.resource);
          } else if (result.status === 403) {
            // 拒绝必须归到权限门机器码，CSRF 家族的 403 属会话前置问题，点名而非算通过
            expect(
              result.code,
              `角色 ${roleCode} 的 ${probe.method} ${probe.path} 得到 403 但机器码=${
                result.code ?? '缺失'
              }，不属权限门 FORBIDDEN，属会话或中间件前置异常，判红`
            ).toBe('FORBIDDEN');
          } else {
            anomalies.push(
              `${probe.resource} ${probe.path} 期望 ${
                exp.allow ? '放行' : '权限门拒绝'
              }（${exp.basis}），实际 HTTP ${result.status} code=${result.code ?? '缺失'}`
            );
          }
        }

        const deniedButAllowed = observedAllow.filter(r => !expectedAllow.includes(r));
        const allowedButDenied = expectedAllow.filter(r => !observedAllow.includes(r));

        expect(
          anomalies,
          `角色 ${roleCode} 有 ${anomalies.length} 条端点既非放行也非权限门 403：\n  ${anomalies
            .slice(0, 12)
            .join('\n  ')}`
        ).toEqual([]);
        expect(
          deniedButAllowed,
          `角色 ${roleCode} 越权放行 ${deniedButAllowed.length} 个资源（无授权却 2xx）：${deniedButAllowed.join(
            ', '
          )}——该角色的授权面里没有对应读键，后端却放行了，属越权缺陷，保持判红`
        ).toEqual([]);
        expect(
          allowedButDenied,
          `角色 ${roleCode} 应放却拒 ${allowedButDenied.length} 个资源：${allowedButDenied.join(
            ', '
          )}——按该角色实持权限键派生应放行，实际未放行，属授权链或门控缺陷，保持判红`
        ).toEqual([]);

        mkdirSync(MATRIX_ARTIFACT_DIR, { recursive: true });
        writeFileSync(
          `${MATRIX_ARTIFACT_DIR}/${seed.code}-backend.json`,
          JSON.stringify(
            {
              role: seed.code,
              dataScope: seed.dataScope,
              permissionKeyCount: keys.length,
              probeCount: probes.length,
              expectedAllow,
              expectedDeny,
              observedAllow,
            },
            null,
            2
          ) + '\n',
          'utf-8'
        );
      } finally {
        await session?.close();
      }
    });
  });
}

/**
 * 接力地基可达性自证：本文件的存在前提是多角色真登录可用。
 * 用 relay 跑两棒（admin 与任一业务角色）各回读一次身份，证明地基未坏。
 * 若地基坏了，上面 42 条矩阵会成片红且根因埋在夹具，这条单独红能把根因先钉住。
 */
test('接力地基在矩阵用车道内可用（两棒身份互异且各自回读到）', async ({ browser }) => {
  const relay = await createRelay(browser);
  try {
    await relay.runSteps([
      {
        role: 'admin',
        label: '矩阵地基第一棒',
        run: async ({ session }) => {
          const me = await apiCallRaw<{ id?: number }>(session.page, 'GET', '/auth/me');
          return { artifacts: [{ key: 'adminUserId', value: Number(me.id) }] };
        },
      },
      {
        role: seededRoles.find(r => r.code === 'auditor') ? 'auditor' : 'manager',
        label: '矩阵地基第二棒',
        run: async ({ session, require }) => {
          const me = await apiCallRaw<{ id?: number }>(session.page, 'GET', '/auth/me');
          const adminUserId = Number(require('adminUserId'));
          expect(
            Number(me.id),
            `第二棒用户 id 必须与第一棒不同，实际两棒都是 id=${
              me.id
            }——同账号自问自答不构成多角色协同，地基失效`
          ).not.toBe(adminUserId);
          return { artifacts: [{ key: 'secondUserId', value: Number(me.id) }] };
        },
      },
    ]);
    expect(
      relay.sessions.length,
      `接力句柄应记下两棒会话，实际 ${relay.sessions.length} 棒，地基未真正开棒`
    ).toBe(2);
  } finally {
    await relay.close();
  }
});

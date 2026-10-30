/**
 * 多角色接力夹具：在同一条用例里按顺序开启多个真登录的角色会话，把上一棒产出的业务标识
 * 显式交给下一棒，并在每一棒收尾时按既有清理口径登记数据回收。
 *
 * 功能：
 * - 按角色码取凭证建真登录 context（凭证来源与登录腿与 loginAsRole 同源，见 multi-context 的 real 档）
 * - 每棒先钉"当前登录身份等于预期账号"，再跑业务动作，跑完复核身份没有漂移
 * - 每棒必须显式声明产出的业务标识，夹具逐项校验其存在与取值合法，缺失即判红
 * - 统一收尾：逐会话 flush 延迟清理队列后关闭 context，可安全重复调用
 *
 * 它拦的两种假绿：
 * 1. 同账号自问自答：相邻棒次若落到同一个用户 id 或同一个账号名即抛错点名，
 *    接力不会因凭证文件里两个角色码指向同一账号而悄悄退化成单人剧。
 * 2. 前序失败后续照绿：下一棒读上一棒产出必须走 require，键不存在即抛错点名已知键；
 *    上一棒没有声明任何产出、或产出值不是有效业务标识，同样当场抛错而不是留给下游。
 *
 * 调用方：e2e/flow 下需要多角色协同接力的 spec；后续业务流程接力用例（审批、发货、收款等）
 * 也经由本夹具开启会话，不各自拼登录。
 * 入参：Playwright 的 browser（由 spec 的 test 作用域 fixture 提供）、角色码、业务动作回调。
 * 传给谁：取凭证与登录交给 fixtures/multi-context 与 flow/helpers；断言消息只进 Playwright 报告。
 * 存什么：只在内存里保存各棒的会话对象、已登记的业务标识与各棒的清理队列。
 * 存哪里：全部不落盘；会话态 cookie 只存在于各自 BrowserContext 内，随 close 一起消亡。
 */
import { expect, type Browser, type Page } from '@playwright/test';
import {
  apiCallExpectFail,
  apiCallRaw,
  BASE_URL,
  deferCleanup,
  expectDenied,
  flushDeferredCleanups,
  getRoleCredential,
  type DeferredCleanup,
} from '../flow/helpers';
import { safeGoto } from '../flow/ui-helpers';
import { createIsolatedSession, type IsolatedSession } from './multi-context';

/** 页面守卫弹跳与表格渲染的等待上限，与既有角色页访问用例的取值同口径 */
const UI_GUARD_WAIT_MS = 15_000;
const UI_ROW_WAIT_MS = 30_000;
/** 头部用户标识渲染等待上限：会话已登录，外壳首屏出现即应可读 */
const UI_IDENTITY_WAIT_MS = 30_000;

/** 一棒会话的完整形态：真登录的隔离会话加身份与清理队列 */
export interface RelaySession extends IsolatedSession {
  /** 该棒登录账号名（由 /auth/me 回读坐实，不是凭证文件里的期望值） */
  username: string;
  /** 该棒登录用户 id，跨棒唯一性判据 */
  userId: number;
  /** 该棒的授权码面（可见性判据的授权侧来源） */
  permissions: string[];
  /** 该棒登记的延迟清理动作，由 close 用该棒自己的会话发起 */
  cleanupQueue: DeferredCleanup[];
}

/** 一棒产出的业务标识：key 是下一棒读取用的名字，value 是单据 id 或单号 */
export interface RelayArtifact {
  key: string;
  value: number | string;
}

/** 一棒登记的数据回收动作，语义与 helpers 的 deferCleanup 入参一致 */
export interface RelayCleanupEntry {
  method: 'DELETE' | 'PUT' | 'POST';
  path: string;
  label?: string;
}

/** 一棒的返回：产出的业务标识必填且非空，清理动作可选 */
export interface RelayStepOutput {
  artifacts: RelayArtifact[];
  cleanup?: RelayCleanupEntry[];
}

/** 交给业务动作回调的上下文 */
export interface RelayStepContext {
  /** 本棒会话（真登录 context 与其 page） */
  session: RelaySession;
  /** 截至上一棒累积的业务标识快照，只读用途；读取请走 require */
  prior: Record<string, number | string>;
  /** 取上一棒（或更早棒次）产出的业务标识，键不存在即抛错点名已知键，绝不返回 undefined */
  require(key: string): number | string;
  /** 登记本棒造出的数据，收尾时由 close 用本棒会话发起清理 */
  registerCleanup(entry: RelayCleanupEntry): void;
}

/** 接力中的一棒 */
export interface RelayStep {
  /** 角色码，用于从 env 与 role-credentials.json 取凭证 */
  role: string;
  /** 本棒在失败信息里的名字，缺省用角色码 */
  label?: string;
  run: (ctx: RelayStepContext) => Promise<RelayStepOutput>;
}

/** 接力运行句柄 */
export interface RelayHandle {
  /** 已开启的棒次会话，按执行顺序 */
  sessions: RelaySession[];
  /** 已登记的全部业务标识 */
  artifacts: Record<string, number | string>;
  /** 顺序执行各棒：任一棒抛错即中断并向上传递，已开启的会话仍由 close 收走 */
  runSteps(steps: RelayStep[]): Promise<void>;
  /** 统一收尾：逆序 flush 各棒清理队列并关闭全部 context，重复调用只生效一次 */
  close(): Promise<void>;
}

/** 会话身份的读法：从后端 auth/me 取当前登录者，不从页面文案或凭证文件反推 */
export interface SessionIdentity {
  userId: number;
  username: string;
  permissions: string[];
}

function describeKeys(obj: Record<string, unknown>): string {
  return Object.keys(obj).join(',');
}

/**
 * 读取一个会话的当前登录身份。
 * 调用方：createRelay 的每一棒开号与收尾复核、spec 的身份互异断言。
 * 入参：已真登录的 page。传给谁：flow/helpers 的 apiCallRaw，打后端 auth/me。
 * 存什么/存哪里：不持久化，直接返回给调用方做判据。
 * 判据口径：id 必须是正数、username 必须是非空字符串、permissions 必须是数组；
 * 任一项缺失或形状不符即抛错点名实际键，绝不以默认值顶替（顶替会把契约漂移读成身份正常）。
 */
export async function readRelayIdentity(page: Page, subject: string): Promise<SessionIdentity> {
  const me = await apiCallRaw<Record<string, unknown> | null>(page, 'GET', '/auth/me');
  if (!me || typeof me !== 'object' || Array.isArray(me)) {
    throw new Error(
      `读身份失败：${subject} 的 auth/me 出参不是对象，实际=${JSON.stringify(me)?.slice(0, 200)}`
    );
  }
  const userId = Number(me.id);
  if (!Number.isFinite(userId) || userId <= 0) {
    throw new Error(
      `读身份失败：${subject} 的 auth/me 未返回有效 id，实际 id=${JSON.stringify(me.id)}，` +
        `实际键=${describeKeys(me)}`
    );
  }
  if (typeof me.username !== 'string' || me.username === '') {
    throw new Error(
      `读身份失败：${subject} 的 auth/me 未返回 username 字符串，实际=` +
        `${JSON.stringify(me.username)}，实际键=${describeKeys(me)}`
    );
  }
  if (!Array.isArray(me.permissions)) {
    throw new Error(
      `读身份失败：${subject} 的 auth/me 出参缺 permissions 数组（可见性差异的授权面判据来源），` +
        `实际键=${describeKeys(me)}`
    );
  }
  return { userId, username: me.username, permissions: (me.permissions as unknown[]).map(String) };
}

/**
 * 校验一棒声明的业务标识取值合法。
 * 调用方：createRelay 的产出登记环节。
 * 入参：产出条目与本棒名字。传给谁：仅用于抛错文本，不落库。
 * 存什么/存哪里：不持久化。
 * 合法取值：正数 id 或非空字符串单号；其余形态（0、负数、NaN、空串、布尔、对象）一律抛错，
 * 因为把 undefined 或 0 记进产出让下一棒"读得到"，正是前序失败后续照绿的通道。
 */
function validateArtifact(entry: RelayArtifact, stepName: string): void {
  if (!entry || typeof entry !== 'object') {
    throw new Error(`${stepName} 返回的产出条目不是对象，实际=${JSON.stringify(entry)}`);
  }
  if (typeof entry.key !== 'string' || entry.key.trim() === '') {
    throw new Error(`${stepName} 的产出键必须是命名字符串，实际=${JSON.stringify(entry.key)}`);
  }
  if (typeof entry.value === 'number') {
    if (!Number.isFinite(entry.value) || entry.value <= 0) {
      throw new Error(
        `${stepName} 的产出 ${entry.key} 为数值时必须是正数（单据 id），实际=${entry.value}`
      );
    }
    return;
  }
  if (typeof entry.value === 'string') {
    if (entry.value.trim() === '') {
      throw new Error(`${stepName} 的产出 ${entry.key} 为空字符串，单号类标识不允许为空`);
    }
    return;
  }
  throw new Error(
    `${stepName} 的产出 ${entry.key} 取值类型只能是数值 id 或字符串单号，` +
      `实际类型=${typeof entry.value}，值=${JSON.stringify(entry.value)}`
  );
}

/**
 * 创建一个接力运行句柄。
 * 调用方：需要多角色协同的 spec，通常在 test 体内先建句柄、try 里 runSteps、finally 里 close。
 * 入参：Playwright 的 browser。传给谁：逐棒交给 createIsolatedSession 的 real 档建真登录会话。
 * 存什么：棒次会话列表、已登记业务标识表、身份去重表。
 * 存哪里：仅本句柄闭包内的内存态，随 spec 结束回收，不落任何文件。
 */
export async function createRelay(browser: Browser): Promise<RelayHandle> {
  const sessions: RelaySession[] = [];
  const artifacts: Record<string, number | string> = {};
  const identityOwners = new Map<number, string>();
  let closed = false;

  function requireArtifact(key: string, stepName: string): number | string {
    if (!Object.prototype.hasOwnProperty.call(artifacts, key)) {
      const known = Object.keys(artifacts);
      throw new Error(
        `${stepName} 读取上一棒产出 ${key} 失败：该键从未被登记。` +
          `已登记的键=${known.length > 0 ? known.join(', ') : '无'}。` +
          '前序棒次没有产出该标识时，本棒不允许继续，否则就是前一步失败、后一步照绿'
      );
    }
    return artifacts[key];
  }

  async function runSteps(steps: RelayStep[]): Promise<void> {
    for (let index = 0; index < steps.length; index++) {
      const step = steps[index];
      const stepName = `接力第 ${index + 1} 棒（角色 ${step.role}，${step.label ?? step.role}）`;
      const cred = getRoleCredential(step.role);
      if (!cred) {
        throw new Error(
          `${stepName} 取凭证失败：角色码 ${step.role} 在 env 与 e2e/.auth/role-credentials.json ` +
            '两处都不存在，接力前提不成立（禁止改用其它账号顶替）'
        );
      }

      const raw = await createIsolatedSession(browser, step.role, {
        authMode: 'real',
        credentialRole: step.role,
      });
      // 身份先钉后跑：登录回来的必须就是凭证里那个账号
      const before = await readRelayIdentity(raw.page, stepName);
      if (before.username !== cred.username) {
        await raw.close();
        throw new Error(
          `${stepName} 身份与预期账号不符：登录返回 ${before.username}，` +
            `角色码 ${step.role} 的凭证账号是 ${cred.username}`
        );
      }
      const repeated = identityOwners.get(before.userId);
      if (repeated !== undefined) {
        await raw.close();
        throw new Error(
          `${stepName} 与前序棒次落到同一个用户 id=${before.userId}（账号 ${repeated} 与 ` +
            `${cred.username}）：接力必须是不同身份，同账号自问自答不构成协作`
        );
      }
      identityOwners.set(before.userId, cred.username);

      const session: RelaySession = {
        ...raw,
        username: before.username,
        userId: before.userId,
        permissions: before.permissions,
        cleanupQueue: [],
      };
      sessions.push(session);

      const output = await step.run({
        session,
        prior: { ...artifacts },
        require: key => requireArtifact(key, stepName),
        registerCleanup: entry =>
          deferCleanup(session.cleanupQueue, entry.method, entry.path, entry.label),
      });

      // 跑完复核：CSRF 恢复分支会在同一 context 内重登成本 worker 账号，
      // 身份若在棒次执行中被换掉，下一棒读到的"他人产出"就成了自问自答，必须在这里钉住。
      const after = await readRelayIdentity(session.page, stepName);
      if (after.userId !== session.userId || after.username !== session.username) {
        throw new Error(
          `${stepName} 执行后身份发生漂移：开棒时 ${session.username}（id=${session.userId}），` +
            `收棒时 ${after.username}（id=${after.userId}），本棒产出不可信，判红不续棒`
        );
      }

      if (!output || !Array.isArray(output.artifacts) || output.artifacts.length === 0) {
        throw new Error(
          `${stepName} 没有声明任何业务标识产出：接力每棒必须交出可被下一棒读取的 ` +
            'id 或单号，空产出说明本棒的业务动作没有真正发生'
        );
      }
      for (const entry of output.artifacts) {
        validateArtifact(entry, stepName);
        if (Object.prototype.hasOwnProperty.call(artifacts, entry.key)) {
          throw new Error(
            `${stepName} 重复登记产出键 ${entry.key}（已登记值=${JSON.stringify(
              artifacts[entry.key]
            )}，本棒值=${JSON.stringify(entry.value)}）：` +
              '后棒覆盖前棒标识会让下一棒读到谁的产出不可判，判红而非静默覆盖'
          );
        }
        artifacts[entry.key] = entry.value;
      }
      for (const entry of output.cleanup ?? []) {
        deferCleanup(session.cleanupQueue, entry.method, entry.path, entry.label);
      }
    }
  }

  async function close(): Promise<void> {
    if (closed) return;
    closed = true;
    // 逆序收尾：后开的 context 先关，且清理用产出者自己的会话权限发，
    // 与既有 flushDeferredCleanups 的"子记录先于父记录、失败仅告警"口径一致。
    for (const session of sessions.slice().reverse()) {
      await flushDeferredCleanups(session.page, session.cleanupQueue);
      await session.close();
    }
  }

  return { sessions, artifacts, runSteps, close };
}

/**
 * 断言某个会话的 UI 外壳已认到预期账号。
 * 调用方：接力 spec 里需要"页面上看得见这个身份"的用例。
 * 入参：已真登录的 page、预期账号名、失败信息前缀。传给谁：只读页面头部用户标识文本。
 * 存什么/存哪里：不持久化。
 * 判据来源：MainLayout 头部用户区渲染的是登录态用户信息里的 username，
 * 取值与后端 auth/me 同源；文本必须逐字等于预期账号，不做包含匹配（包含匹配会被前后缀掩盖）。
 */
export async function assertUiIdentity(
  page: Page,
  expectedUsername: string,
  context: string
): Promise<void> {
  const chip = page.locator('.header-right .user-info');
  await expect(
    chip,
    `${context}：页面头部用户标识未渲染，说明外壳没有认到登录态（页面可能停在登录页）`
  ).toBeVisible({ timeout: UI_IDENTITY_WAIT_MS });
  const shown = (await chip.innerText()).trim();
  expect(
    shown,
    `${context}：页面头部账号名应逐字等于预期账号 ${expectedUsername}，实际渲染为 ${shown}`
  ).toBe(expectedUsername);
}

/**
 * 断言侧栏菜单里存在该名字的叶子项。
 * 调用方：需要钉"角色 A 看得见某入口"的接力 spec。
 * 入参：已停在应用外壳内的 page、菜单项文案、失败信息前缀。
 * 传给谁：只在 DOM 上计数，不发请求、不落库。
 * 存什么/存哪里：不持久化。
 * 判据口径：取 DOM 存在性计数（与既有入口回归 spec 同选法）。分组折叠时子项仍留在 DOM 内，
 * 因此判定不受展开态影响；无权限的项由可见性判据直接不渲染，计数为 0，两种角色天然可分。
 */
export async function expectMenuItemPresent(
  page: Page,
  label: string,
  context: string
): Promise<void> {
  const menu = page.locator('.menu');
  await expect(menu, `${context}：侧栏菜单容器未渲染，外壳未加载成功`).toBeVisible({
    timeout: UI_IDENTITY_WAIT_MS,
  });
  const item = menu.locator('.el-menu-item', { hasText: label });
  const count = await item.count();
  expect(
    count,
    `${context}：侧栏应存在菜单项 ${label}，实际命中 ${count} 个（有权却缺项说明可见性判据把该岗挡了）`
  ).toBe(1);
}

/**
 * 断言侧栏菜单里不存在该名字的叶子项。
 * 调用方：需要钉"越权角色看不见某入口"的接力 spec。
 * 入参：已停在应用外壳内的 page、菜单项文案、失败信息前缀。传给谁：只在 DOM 上计数。
 * 存什么/存哪里：不持久化。
 * 与 expectMenuItemPresent 成对使用：同一菜单项在一侧计数为 1、另一侧为 0，
 * 才构成"不同角色的菜单可见性差异真实存在"，只断其中一侧不算差异。
 */
export async function expectMenuItemAbsent(
  page: Page,
  label: string,
  context: string
): Promise<void> {
  const menu = page.locator('.menu');
  await expect(menu, `${context}：侧栏菜单容器未渲染，外壳未加载成功`).toBeVisible({
    timeout: UI_IDENTITY_WAIT_MS,
  });
  const count = await menu.locator('.el-menu-item', { hasText: label }).count();
  expect(
    count,
    `${context}：侧栏不应存在菜单项 ${label}，实际命中 ${count} 个（越权角色看见了无权入口）`
  ).toBe(0);
}

/**
 * 正向：进入功能页并断言表格真渲出含该唯一标识的行。
 * 调用方：接力 spec 里"下一棒看得见上一棒造的对象"这一步。
 * 入参：会话 page、功能页路径、行内可见的唯一标识文案、失败信息前缀。
 * 传给谁：导航走 flow/ui-helpers 的 safeGoto（含 Vite 首编译重试），表格走行文本定位。
 * 存什么/存哪里：不持久化，只读 DOM。
 * 判据：页面未被路由守卫弹到无权限页，且含该文案的数据行在 DOM 里可见。
 * 只断"页面能打开"不算看见；只断"接口返回列表有这行"也不算看见，本函数钉的是 UI 渲出来。
 */
export async function expectPageRendersRow(
  page: Page,
  routePath: string,
  uniqueText: string,
  context: string
): Promise<void> {
  await safeGoto(page, routePath);
  const url = page.url();
  expect(
    url,
    `${context}：进入功能页 ${routePath} 被路由守卫弹到无权限页，与"该角色有权看见"的预期相反`
  ).not.toMatch(/\/403(\?|#|$)/);
  const row = page.locator('.el-table__body .el-table__row', { hasText: uniqueText });
  try {
    await row.first().waitFor({ state: 'visible', timeout: UI_ROW_WAIT_MS });
  } catch (error) {
    throw new Error(
      `${context}：功能页 ${routePath} 未渲染出含 ${uniqueText} 的数据行：${(error as Error).message}`
    );
  }
}

/**
 * 反向：断言功能页被路由守卫挡到无权限页且不泄露任何业务数据行。
 * 调用方：接力 spec 里"越权角色看不到该对象"这一步。
 * 入参：会话 page、功能页路径、失败信息前缀。传给谁：直接 goto 目标地址并等待真实导航结果。
 * 存什么/存哪里：不持久化，只读 DOM 与 URL。
 * 判据取真实导航落点（waitForURL），并附钉被挡后页面不含业务表格行。
 * goto 不吞异常：导航自身失败就是真失败，落点判定不成立时应由它点名，不降级成告警。
 */
export async function expectPageGuardDenied(
  page: Page,
  routePath: string,
  context: string
): Promise<void> {
  await page.goto(`${BASE_URL}${routePath}`, { waitUntil: 'domcontentloaded' });
  try {
    await page.waitForURL(/\/403(\?|#|$)/, { timeout: UI_GUARD_WAIT_MS });
  } catch (error) {
    throw new Error(
      `${context}：越权页 ${routePath} 未被路由守卫挡到无权限页，停留在 ${page.url()}：` +
        (error as Error).message
    );
  }
  const leakedRows = await page.locator('.el-table__body .el-table__row').count();
  expect(
    leakedRows,
    `${context}：被挡的越权页 ${routePath} 不应渲染业务数据行，实际 ${leakedRows} 行`
  ).toBe(0);
}

/**
 * 越权端点必须被权限门拒绝。
 * 调用方：接力 spec 里第三种角色对同一端点的负向步骤。
 * 入参：会话 page、方法、端点路径、失败信息前缀。
 * 传给谁：请求与归因全部走 flow/helpers 既有实现（apiCallExpectFail 加 expectDenied）。
 * 存什么/存哪里：只发请求，不持久化。
 * 本函数只是既有判据的薄封装，不另立第二套拒绝标准：HTTP 403 加权限机器码 FORBIDDEN，
 * CSRF 家族的拒绝码会被 expectDenied 点名成会话前置问题而非权限判定。
 */
export async function expectEndpointDenied(
  page: Page,
  method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE',
  path: string,
  context: string
): Promise<void> {
  const result = await apiCallExpectFail(page, method, path);
  expectDenied(result, context);
}

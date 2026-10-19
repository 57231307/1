/**
 * E2E worker 会话池：账号名、会话态文件路径与会话池清单的唯一事实来源。
 *
 * 为什么需要它（根因面）：后端 CSRF 凭据按 user_id 单命名空间登记、一次性消费，且登录时
 * 清除该用户既有凭据；Playwright 的 use.storageState 是配置级静态值（配置文件只在主进程
 * 求值一次，worker 进程拿到的是反序列化后的同一份对象），因此"一个分片一个账号 + 一份
 * 静态会话态文件 + 多个并发 worker"必然让各 worker 持有同一枚凭据字符串并互相打死：
 * 第一个消费者之外的 worker 带的都是已消费值，写请求确定性 403 CSRF_TOKEN_INVALID。
 * 本模块把「一个并行 worker 槽位 = 一个独立账号 = 一份独立会话态文件」这条约束固化成
 * 单一派生点，账号名与路径都由并行槽序号决定，任何第二处拼账号的写法都会与这里漂移。
 *
 * 调用方：
 *   - e2e/global-setup.ts：按解析后的 workers 数建号、逐 worker 登录并写出会话态与会话池清单；
 *   - e2e/diagnose-fixture.ts：worker 作用域 fixture 按 testInfo.parallelIndex 取本 worker 的会话；
 *   - e2e/flow/helpers.ts、e2e/fixtures/auth.ts、e2e/fixtures/multi-context.ts、e2e/smoke/_helpers.ts：
 *     经 env 读取本 worker 的账号与密码（显式登录/重登路径必须与本 worker 会话态同源）。
 * 入参形态：shardIndex 取自 E2E_SHARD_INDEX（CI 分片序号，本地为空串）、parallelIndex 是
 * Playwright 并行槽序号（0 起、worker 重启后复用，因此可安全当作账号槽位；workerIndex 会
 * 随重启递增，不可用）。
 * 存什么：会话池清单存 shardIndex / workerCount / seedUsername / sessions[]（每条含账号名、
 * 密码、会话态文件路径）；会话态文件存该 worker 登录后拿到的 access_token 与 csrf_token。
 * 存哪里：一律落在被 gitignore 覆盖的 e2e/.auth/ 下，随 CI job 的独立数据库一同消亡。
 *
 * 失败口径：会话池清单缺失、条目缺失、内容损坏一律显式抛错并点名缺什么，绝不回落到
 * 共享账号或共享会话态——回落会把并发互踢的真实缺陷重新藏成偶发。
 */
import { existsSync, readFileSync } from 'fs';

/** 会话池清单路径：globalSetup 写、worker 侧读，两侧唯一的对账对象 */
export const WORKER_SESSIONS_MANIFEST_PATH = 'e2e/.auth/worker-sessions.json';

/** 本 worker 会话在 env 里的三个键：由 worker 作用域 fixture 写入，同进程的 helpers 读取 */
export const WORKER_USERNAME_ENV = 'E2E_WORKER_USERNAME';
export const WORKER_PASSWORD_ENV = 'E2E_WORKER_PASSWORD';
export const WORKER_STORAGE_STATE_ENV = 'E2E_WORKER_STORAGE_STATE';

/** 一个并行槽位的完整会话：账号、建号时写入的密码、该账号专属会话态文件 */
export interface WorkerSession {
  parallelIndex: number;
  username: string;
  password: string;
  storageStatePath: string;
}

/** 会话池清单结构：workerCount 是并行槽上界，seedUsername 只作建号/种子主体不参与会话 */
export interface WorkerSessionsManifest {
  shardIndex: string;
  workerCount: number;
  seedUsername: string;
  sessions: WorkerSession[];
}

/** 分片账号基名：worker 账号与种子账号都在这一段命名空间下，集中一处避免拼写分叉 */
const ADMIN_ACCOUNT_BASE = 'e2e_admin';
const AUTH_DIR = 'e2e/.auth';

function requireNonNegativeInt(value: number, label: string): void {
  if (!Number.isInteger(value) || value < 0) {
    throw new Error(`${label} 必须是非负整数，实际：${JSON.stringify(value)}`);
  }
}

/**
 * 派生某个并行槽的 admin 账号名。
 * 入参：shardIndex（CI 分片序号，本地传空串）、parallelIndex（并行槽序号）。
 * 传给谁：globalSetup 建号与登录、会话池清单写出、门禁断言账号互不相同。
 * 存什么：只存字符串，不落盘；账号名同时含分片段与 worker 段，跨分片与跨 worker 都不会撞名，
 * 也不会等于分片种子账号（e2e_admin_s{N}）或基础管理员账号（e2e_admin）。
 */
export function workerAdminUsername(shardIndex: string, parallelIndex: number): string {
  requireNonNegativeInt(parallelIndex, 'parallelIndex');
  const shardPart = shardIndex === '' ? '' : `_s${shardIndex}`;
  return `${ADMIN_ACCOUNT_BASE}${shardPart}_w${parallelIndex}`;
}

/**
 * 派生某个并行槽专属的会话态文件路径。
 * 入参：同 workerAdminUsername。
 * 传给谁：globalSetup 写文件、内置 storageState fixture 的覆盖值、用例内二次建 context。
 * 存什么：该 worker 自己登录后由后端下发的 cookie（含 csrf_token），一个文件只属于一个 worker。
 * 存哪里：e2e/.auth/ 下（gitignore 覆盖）；用正斜杠拼接，Node 在 Windows 与 Linux 两侧一致。
 */
export function workerStorageStatePath(shardIndex: string, parallelIndex: number): string {
  requireNonNegativeInt(parallelIndex, 'parallelIndex');
  const shardPart = shardIndex === '' ? '' : `-s${shardIndex}`;
  return `${AUTH_DIR}/storage-state${shardPart}-w${parallelIndex}.json`;
}

/**
 * 构造整池会话（纯派生，不触网）。
 * 入参：shardIndex、workerCount（解析后的 Playwright workers 数）、password（建号即登录用的密码）。
 * 传给谁：globalSetup 拿去逐条建号登录并写盘；门禁断言账号与路径两两不同。
 * 存什么：每条含并行槽序号、账号名、密码、会话态路径；序号必须与下标一致（清单按槽位对账）。
 * 存哪里：不持久化，返回给调用方写出清单。
 */
export function buildWorkerSessions(
  shardIndex: string,
  workerCount: number,
  password: string
): WorkerSession[] {
  if (!Number.isInteger(workerCount) || workerCount < 1) {
    throw new Error(
      `workerCount 必须是 ≥1 的整数（会话池规模 = 解析后的 Playwright workers 数），实际：${workerCount}`
    );
  }
  if (typeof password !== 'string' || password === '') {
    throw new Error('buildWorkerSessions 必须拿到建号密码：空密码会让会话池写出不可用账号');
  }
  const sessions: WorkerSession[] = [];
  for (let parallelIndex = 0; parallelIndex < workerCount; parallelIndex++) {
    sessions.push({
      parallelIndex,
      username: workerAdminUsername(shardIndex, parallelIndex),
      password,
      storageStatePath: workerStorageStatePath(shardIndex, parallelIndex),
    });
  }
  return sessions;
}

function describeManifest(manifest: unknown): string {
  try {
    return JSON.stringify(manifest).slice(0, 240);
  } catch {
    return '<不可序列化>';
  }
}

/**
 * 从清单文本取某个并行槽的会话。
 * 入参：manifestText（清单文件原文）、parallelIndex（要取的槽位）、expectedWorkerCount
 *      （调用方已知的 workers 数，仅用于把"缺槽位"报成可定位的差异，缺省按清单自报规模）。
 * 传给谁：readWorkerSession（worker 侧）与门禁（构造缺槽位/损坏内容判红）。
 * 存什么：不持久化。
 * 存哪里：不持久化；任何结构异常都显式抛错，绝不返回共享账号或空会话。
 */
export function parseWorkerSessions(
  manifestText: string,
  parallelIndex: number,
  expectedWorkerCount?: number
): WorkerSession {
  requireNonNegativeInt(parallelIndex, 'parallelIndex');
  let parsed: unknown;
  try {
    parsed = JSON.parse(manifestText);
  } catch (e) {
    throw new Error(
      `会话池清单不是合法 JSON（并行槽 ${parallelIndex} 无从取会话）：${(e as Error).message}`
    );
  }
  const manifest = parsed as Partial<WorkerSessionsManifest> | null;
  if (!manifest || typeof manifest !== 'object') {
    throw new Error(`会话池清单结构非法（应为对象），实际片段=${describeManifest(parsed)}`);
  }
  if (!Array.isArray(manifest.sessions)) {
    throw new Error(
      `会话池清单缺 sessions 数组（workers 规模无法对账），实际片段=${describeManifest(manifest)}`
    );
  }
  const declared = manifest.workerCount;
  if (!Number.isInteger(declared) || (declared as number) < 1) {
    throw new Error(
      `会话池清单的 workerCount 非法（应为 ≥1 整数，实际=${JSON.stringify(declared)}）——` +
        `会话池规模必须等于解析后的 Playwright workers 数`
    );
  }
  if (manifest.sessions.length !== declared) {
    throw new Error(
      `会话池清单条目数与自报规模不符：sessions=${manifest.sessions.length} workers=${declared}` +
        `（建号或写出中断，禁止带缺槽运行）`
    );
  }
  const knownWorkers = Number.isInteger(expectedWorkerCount)
    ? (expectedWorkerCount as number)
    : (declared as number);
  if (parallelIndex >= knownWorkers) {
    throw new Error(
      `会话池清单里没有并行槽 ${parallelIndex} 的账号：清单登记 ${manifest.sessions.length} 条，` +
        `本次运行 workers=${knownWorkers}；会话池规模必须覆盖 0..workers-1 的全部槽位，` +
        `缺槽位时拒绝运行（禁止回落共享账号）`
    );
  }
  const session = manifest.sessions[parallelIndex];
  if (!session || typeof session !== 'object') {
    throw new Error(
      `会话池清单并行槽 ${parallelIndex} 的条目不存在（workers=${knownWorkers}），` +
        `实际片段=${describeManifest(manifest.sessions)}`
    );
  }
  const candidate = session as Partial<WorkerSession>;
  for (const field of ['username', 'password', 'storageStatePath'] as const) {
    if (typeof candidate[field] !== 'string' || candidate[field] === '') {
      throw new Error(
        `会话池清单并行槽 ${parallelIndex} 的 ${field} 缺失或为空（实际条目=${describeManifest(
          candidate
        )}）——宁可判红，不写出不可用会话`
      );
    }
  }
  if (!Number.isInteger(candidate.parallelIndex) || candidate.parallelIndex !== parallelIndex) {
    throw new Error(
      `会话池清单并行槽 ${parallelIndex} 的条目 self-index 不是 ${parallelIndex}` +
        `（实际=${JSON.stringify(candidate.parallelIndex)}）——清单按槽位下标对账，错位即判红`
    );
  }
  return {
    parallelIndex,
    username: candidate.username as string,
    password: candidate.password as string,
    storageStatePath: candidate.storageStatePath as string,
  };
}

/**
 * 读本 worker 的会话（worker 侧唯一入口）。
 * 入参：parallelIndex（testInfo.parallelIndex）。
 * 传给谁：e2e/diagnose-fixture.ts 的 worker 作用域 fixture。
 * 存什么/存哪里：只读 e2e/.auth/ 下的会话池清单，不写任何东西；文件缺失即点名该路径判红
 * （说明 globalSetup 未运行或未建够会话池，属 setup 缺陷，不能当成"用例自己的账号"继续跑）。
 */
export function readWorkerSession(parallelIndex: number): WorkerSession {
  if (!existsSync(WORKER_SESSIONS_MANIFEST_PATH)) {
    throw new Error(
      `worker 会话池清单不存在：${WORKER_SESSIONS_MANIFEST_PATH}` +
        `（并行槽 ${parallelIndex} 无从取账号）。globalSetup 未运行或未写出会话池——` +
        `每 worker 独立账号是本套件的前提，禁止回落到共享账号`
    );
  }
  const text = readFileSync(WORKER_SESSIONS_MANIFEST_PATH, 'utf-8');
  return parseWorkerSessions(text, parallelIndex);
}

/**
 * 把本 worker 的会话写进同进程 env。
 * 入参：session（readWorkerSession 的返回值）。
 * 传给谁：worker 作用域 fixture 在每个 worker 启动时调用一次。
 * 存什么：账号名、密码、会话态路径三个键。
 * 存哪里：仅本 worker 进程的 process.env（Playwright 每个 worker 是独立 Node 进程，
 * 因此这里写的值不会外溢到别的 worker；主进程 env 在各 worker 启动前已定型且不含这三个键）。
 */
export function applyWorkerSessionEnv(session: WorkerSession): void {
  process.env[WORKER_USERNAME_ENV] = session.username;
  process.env[WORKER_PASSWORD_ENV] = session.password;
  process.env[WORKER_STORAGE_STATE_ENV] = session.storageStatePath;
}

function requireWorkerEnv(name: string): string {
  const value = process.env[name];
  if (!value) {
    throw new Error(
      `${name} 未注入：当前进程还没绑定 worker 会话。` +
        `请确认该用例从 e2e/diagnose-fixture 取 test（worker 作用域 fixture 负责注入），` +
        `且会话池清单 ${WORKER_SESSIONS_MANIFEST_PATH} 已覆盖本次运行的 workers 数`
    );
  }
  return value;
}

/** 本 worker 的登录账号名；调用方：helpers 的显式登录/重登与用例内二次建会话 */
export function workerUsernameFromEnv(): string {
  return requireWorkerEnv(WORKER_USERNAME_ENV);
}

/** 本 worker 账号的密码（与建号时同源，不再从 env 猜测，避免凭证与账号漂移） */
export function workerPasswordFromEnv(): string {
  return requireWorkerEnv(WORKER_PASSWORD_ENV);
}

/** 本 worker 的会话态文件路径；调用方：需要另建 context 复用本 worker 登录态的用例 */
export function workerStorageStatePathFromEnv(): string {
  return requireWorkerEnv(WORKER_STORAGE_STATE_ENV);
}

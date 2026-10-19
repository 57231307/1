// E2E 统一 fixture：worker 级独立登录态 + 诊断信号采集。
//
// 一、worker 级独立登录态（本模块是会话态进入 spec 的唯一通道）
// 配置里的 use.storageState 是"全 run 求值一次"的静态值，多个并发 worker 会读到同一份
// 会话态文件、带同一枚 CSRF 凭据字符串；后端凭据按 user_id 单命名空间且一次性消费，
// 于是 worker 之间互相打死对方的凭据，表现为同分片同用例确定性 403 CSRF_TOKEN_INVALID。
// 这里按 Playwright 并行槽序号（testInfo.parallelIndex，worker 重启后仍复用同一槽位）
// 取该 worker 专属的账号与会话态文件，覆盖内置 storageState fixture，使每个 worker 只
// 消费自己那份凭据。账号名与会话态路径的派生规则集中在 e2e/worker-auth.ts（单点），
// 本模块只负责"取会话 + 注入同进程 env + 覆盖 fixture"。
// 用法：spec 文件从本模块取 test（不得再从 '@playwright/test' 直接取，否则该文件
// 拿不到 worker 会话而静默共享账号，由 scripts/check-e2e-worker-account.mjs 判红）。
//
// 二、诊断模式（用户指令：尽可能多获取测试日志便于错误分析，减少 CI 重跑次数）
// 对每个测试页面自动采集以下信号并以 `[diag]` 前缀显式输出到 stdout
// （CI 控制台与 playwright 输出文件均可见）：
//   1. console error/warning（计数 + 失败/慢用例时输出前 30 条明细）
//   2. pageerror（未捕获 JS 异常）
//   3. requestfailed（网络层失败：DNS/连接重置/中止等）
//   4. 4xx/5xx/3xx 响应（方法、URL、状态码、响应体前 200 字符）
// 同时保留 trace/screenshot/video 的 retain-on-failure（playwright.config.ts）。
import { test as base, expect } from '@playwright/test';
import {
  applyWorkerSessionEnv,
  readWorkerSession,
  type WorkerSession,
} from './worker-auth';

type DiagSignals = {
  consoleErrors: string[];
  consoleWarns: string[];
  pageErrors: string[];
  requestFailures: string[];
  httpErrors: string[];
};

type WorkerSessionArgs = {
  /** 本 worker 的会话（账号、密码、专属会话态文件路径） */
  workerSession: WorkerSession;
};

const BODY_TRUNCATE = 200;
const CONSOLE_DETAIL_MAX = 30;

// worker 作用域 + auto：每个 worker 起一次（不是每用例一次），因此账号与会话态
// 的生命周期跟 worker 严格对齐；auto 保证未显式声明依赖的 spec 也会被绑定会话，
// beforeAll（'all-hooks-only' 求解）同样会先跑它，用例里用 browser 另建 context 时
// 已能经 env 取到本 worker 的会话态路径。
// 第一个类型参数用 Record<never, never> 表达"不新增 test 作用域 fixture"：Playwright 的
// extend 约束虽写作 T extends {}，但 @typescript-eslint/no-empty-object-type 会把空对象字面量
// 类型判为 error；Record<never, never> 与之等值、可赋给该约束，且类型在运行时被擦除、不触该规则。
// 首参必须是解构模式：Playwright 在注册 fixture 时强制校验首参形如 {…}，写成普通标识符会让收集
// 阶段直接报 "First argument must use the object destructuring pattern" 并崩掉整条 --list 链。
// 本 fixture 不消费任何 test 作用域 fixture，故留空对象解构 {}；no-empty-pattern 会判它为空模式，
// 这是 Playwright API 的硬性要求而非疏漏，按行豁免该规则而不是改成非解构形参。
const withWorkerSession = base.extend<Record<never, never>, WorkerSessionArgs>({
  workerSession: [
    // eslint-disable-next-line no-empty-pattern
    async ({}, use, testInfo) => {
      const session = readWorkerSession(testInfo.parallelIndex);
      applyWorkerSessionEnv(session);
      console.log(
        `[workerAuth] 并行槽 ${testInfo.parallelIndex} 绑定账号 ${session.username} ` +
          `会话态 ${session.storageStatePath}（worker=${testInfo.workerIndex}）`
      );
      await use(session);
      console.log(
        `[workerAuth] 并行槽 ${testInfo.parallelIndex} 会话结束（账号 ${session.username}）`
      );
    },
    { scope: 'worker', auto: true },
  ],
});

// 覆盖内置 storageState option fixture：context 建立时读本 worker 的会话态文件。
// 刻意不返回内联对象而返回路径字符串——Playwright 在每次建 context 时现读文件，
// 文件内容由 globalSetup 一次写出且只属于这一个 worker。
const withWorkerStorageState = withWorkerSession.extend({
  storageState: async ({ workerSession }, use) => {
    await use(workerSession.storageStatePath);
  },
});

export const test = withWorkerStorageState.extend<{ diag: DiagSignals }>({
  page: async ({ page }, use, testInfo) => {
    const diag: DiagSignals = {
      consoleErrors: [],
      consoleWarns: [],
      pageErrors: [],
      requestFailures: [],
      httpErrors: [],
    };
    page.on('console', msg => {
      const type = msg.type();
      if (type === 'error') {
        diag.consoleErrors.push(`${Date.now()} ${msg.text().slice(0, 500)}`);
      } else if (type === 'warning') {
        diag.consoleWarns.push(`${Date.now()} ${msg.text().slice(0, 500)}`);
      }
    });
    page.on('pageerror', err => {
      const line = `${Date.now()} ${err.message.slice(0, 500)}`;
      diag.pageErrors.push(line);
      console.log(`[diag][pageerror] ${line} @ ${testInfo.title}`);
    });
    page.on('requestfailed', req => {
      const line = `${req.method()} ${req.url()} -> ${(req.failure() as { errorText?: string } | null)?.errorText ?? 'unknown'}`;
      diag.requestFailures.push(line);
      console.log(`[diag][requestfailed] ${line} @ ${testInfo.title}`);
    });
    page.on('response', res => {
      const status = res.status();
      if (status >= 300) {
        const loc = testInfo.title;
        const isError = status >= 400;
        res
          .text()
          .then(body => {
            const line = `${res.request().method()} ${res.url()} -> ${status} | ${body.slice(0, BODY_TRUNCATE).replace(/\s+/g, ' ')}`;
            diag.httpErrors.push(line);
            // 4xx/5xx 无条件即时输出（用户指令：尽可能多拿日志减少 CI 次数）；3xx 仅汇总防刷屏
            if (isError) console.log(`[diag][http${status}] ${line} @ ${loc}`);
          })
          .catch(() => {
            const line = `${res.request().method()} ${res.url()} -> ${status} | <响应体读取失败>`;
            diag.httpErrors.push(line);
            if (isError) console.log(`[diag][http${status}] ${line} @ ${loc}`);
          });
      }
    });
    await use(page);
    // 统一诊断汇总：所有用例都输出计数；失败或耗时>90s 的额外输出明细样本
    const failed = testInfo.status === 'failed' || testInfo.status === 'timedOut';
    const slow = testInfo.duration > 90_000;
    console.log(
      `[diag][summary] ${testInfo.title} | status=${testInfo.status} dur=${Math.round(testInfo.duration / 1000)}s ` +
        `consoleErr=${diag.consoleErrors.length} consoleWarn=${diag.consoleWarns.length} ` +
        `pageErr=${diag.pageErrors.length} reqFail=${diag.requestFailures.length} httpErr=${diag.httpErrors.length}`
    );
    if (failed || slow) {
      for (const e of diag.consoleErrors.slice(0, CONSOLE_DETAIL_MAX))
        console.log(`[diag][console.error] ${e} @ ${testInfo.title}`);
      for (const e of diag.pageErrors.slice(0, CONSOLE_DETAIL_MAX))
        console.log(`[diag][pageerror] ${e} @ ${testInfo.title}`);
      for (const e of diag.requestFailures.slice(0, CONSOLE_DETAIL_MAX))
        console.log(`[diag][requestfailed] ${e} @ ${testInfo.title}`);
      for (const e of diag.httpErrors.slice(0, CONSOLE_DETAIL_MAX))
        console.log(`[diag][http] ${e} @ ${testInfo.title}`);
    }
  },
});

export { expect };

// 重新导出 @playwright/test 的常用符号，供 spec 文件统一从 diagnose-fixture 导入
export { devices, type Page, type Locator, type BrowserContext } from '@playwright/test';

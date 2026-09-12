// E2E 诊断模式统一 fixture。
// 背景：CI 单轮约 3 小时，为在一次运行内最大化错误归因信息，本模块对每个测试页面
// 自动采集以下信号并以 `[diag]` 前缀显式输出到 stdout（CI 控制台与 playwright 输出文件均可见）：
//   1. console error/warning（计数 + 失败/慢用例时输出前 30 条明细）
//   2. pageerror（未捕获 JS 异常）
//   3. requestfailed（网络层失败：DNS/连接重置/中止等）
//   4. 4xx/5xx/3xx 响应（方法、URL、状态码、响应体前 200 字符）
// 同时保留 trace/screenshot/video 的 retain-on-failure（playwright.config.ts）。
// 用法：spec 文件把 `import { test } from '@playwright/test'` 替换为
// `import { test, expect } from '…/diagnose-fixture'`（新 test 继承原 test.extend，无行为差异）。
import { test as base, expect } from '@playwright/test';

type DiagSignals = {
  consoleErrors: string[];
  consoleWarns: string[];
  pageErrors: string[];
  requestFailures: string[];
  httpErrors: string[];
};

const BODY_TRUNCATE = 200;
const CONSOLE_DETAIL_MAX = 30;

export const test = base.extend<{ diag: DiagSignals }>({
  page: async ({ page }, use, testInfo) => {
    const diag: DiagSignals = {
      consoleErrors: [],
      consoleWarns: [],
      pageErrors: [],
      requestFailures: [],
      httpErrors: [],
    };
    page.on('console', (msg) => {
      const type = msg.type();
      if (type === 'error') {
        diag.consoleErrors.push(`${Date.now()} ${msg.text().slice(0, 500)}`);
      } else if (type === 'warning') {
        diag.consoleWarns.push(`${Date.now()} ${msg.text().slice(0, 500)}`);
      }
    });
    page.on('pageerror', (err) => {
      const line = `${Date.now()} ${err.message.slice(0, 500)}`;
      diag.pageErrors.push(line);
      console.log(`[diag][pageerror] ${line} @ ${testInfo.title}`);
    });
    page.on('requestfailed', (req) => {
      const line = `${req.method()} ${req.url()} -> ${(req.failure() as { errorText?: string } | null)?.errorText ?? 'unknown'}`;
      diag.requestFailures.push(line);
      console.log(`[diag][requestfailed] ${line} @ ${testInfo.title}`);
    });
    page.on('response', (res) => {
      const status = res.status();
      if (status >= 300) {
        const loc = testInfo.title;
        const isError = status >= 400;
        res
          .text()
          .then((body) => {
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
        `pageErr=${diag.pageErrors.length} reqFail=${diag.requestFailures.length} httpErr=${diag.httpErrors.length}`,
    );
    if (failed || slow) {
      for (const e of diag.consoleErrors.slice(0, CONSOLE_DETAIL_MAX)) console.log(`[diag][console.error] ${e} @ ${testInfo.title}`);
      for (const e of diag.pageErrors.slice(0, CONSOLE_DETAIL_MAX)) console.log(`[diag][pageerror] ${e} @ ${testInfo.title}`);
      for (const e of diag.requestFailures.slice(0, CONSOLE_DETAIL_MAX)) console.log(`[diag][requestfailed] ${e} @ ${testInfo.title}`);
      for (const e of diag.httpErrors.slice(0, CONSOLE_DETAIL_MAX)) console.log(`[diag][http] ${e} @ ${testInfo.title}`);
    }
  },
});

export { expect };

// 重新导出 @playwright/test 的常用符号，供 spec 文件统一从 diagnose-fixture 导入
export { devices, type Page, type Locator, type BrowserContext } from '@playwright/test';

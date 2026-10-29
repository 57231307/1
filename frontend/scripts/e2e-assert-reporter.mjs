// e2e-assert-reporter.mjs — 分片用例「步骤/断言计数」上报器（E2E 真实性门禁判据一/二/三的数据源）。
// 功能：把每条用例的最终状态、各次尝试的 expect 断言数与步骤数落成机器可读 JSON，
// 供 e2e-authenticity-guard 断言"每片 executed>0 且每用例至少一枚断言"。
// 为什么不用内置 json reporter：实测（@playwright/test 1.63，最小夹具 standalone 跑）
// 其用例 result 对象不携带 steps 字段，断言计数无从取数；expect 断言只在 reporter
// 事件的 onStepEnd（category='expect'）里可见。
// 调用方：.github/workflows/ci-cd.yml ci-e2e 跑测步骤 --reporter=line,scripts/e2e-assert-reporter.mjs。
// 入参：无 CLI 参；输出路径取 env E2E_ASSERT_STATS，缺省 reports/e2e-assert-stats.json
// （相对 playwright 运行目录 frontend/，随 e2e-report-shard 产物上传）。
// 传给谁：actions/upload-artifact → e2e-authenticity-guard 的 check-e2e-authenticity.mjs。
// 存什么：schema 版本号、stats 四档计数、tests[] 逐条 {file,line,title,project,status,
// attempts,skippedByAnnotation,assertionsPerAttempt,stepsTotal}；不落库、不发请求。
// 存哪里：单文件 JSON，覆盖写；文件缺失时 guard 按"未验证"判红，不做静默放行。
import fs from 'node:fs';
import path from 'node:path';

const OUT = process.env.E2E_ASSERT_STATS || path.join('reports', 'e2e-assert-stats.json');

export default class E2eAssertReporter {
  constructor() {
    // test 对象 → 计数记录（Map 保持插入序，产物里用例顺序即执行登记顺序）
    this.rows = new Map();
    this.stats = { expected: 0, unexpected: 0, flaky: 0, skipped: 0 };
  }

  onTestBegin(test) {
    const r = this.#row(test);
    r.attempts += 1;
    // 每次尝试对应一个断言计数槽位；未跑起来的尝试保持 0，guard 侧据此区分空跑
    r.assertionsPerAttempt.push(0);
  }

  onStepEnd(test, _result, step) {
    const r = this.#row(test);
    r.stepsTotal += 1;
    if (step.category === 'expect') {
      const last = r.assertionsPerAttempt.length - 1;
      if (last >= 0) r.assertionsPerAttempt[last] += 1;
    }
  }

  onTestEnd(test, result) {
    const r = this.#row(test);
    r.status = result.status;
    const last = r.assertionsPerAttempt.length - 1;
    if (
      last >= 0 &&
      r.assertionsPerAttempt[last] === 0 &&
      (result.status === 'skipped' || result.status === 'interrupted')
    ) {
      // skipped/interrupted 的尝试不产生断言步骤，不计入"零断言"面
      r.assertionsPerAttempt[last] = null;
    }
  }

  onEnd(result) {
    for (const r of this.rows.values()) {
      const st = r.status;
      if (st === 'skipped' || st === 'cancelled') this.stats.skipped += 1;
      else if (r.attempts > 1 && st === 'passed') this.stats.flaky += 1;
      else if (st === 'passed') this.stats.expected += 1;
      else this.stats.unexpected += 1;
    }
    const doc = {
      schema: 'e2e-assert-stats/1',
      runStatus: result.status,
      stats: this.stats,
      tests: [...this.rows.values()],
    };
    fs.mkdirSync(path.dirname(OUT) || '.', { recursive: true });
    fs.writeFileSync(OUT, JSON.stringify(doc));
  }

  #row(test) {
    const loc = test.location || {};
    const key = `${loc.file || ''}|${loc.line || 0}|${test.title}|${test.project?.name || ''}`;
    let r = this.rows.get(key);
    if (!r) {
      r = {
        file: loc.file || '',
        line: loc.line || 0,
        title: test.title,
        project: test.project?.name || '',
        status: 'unknown',
        attempts: 0,
        assertionsPerAttempt: [],
        stepsTotal: 0,
      };
      this.rows.set(key, r);
    }
    return r;
  }
}

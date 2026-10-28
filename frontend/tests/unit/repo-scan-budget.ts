/*
 * 扫描型静态锁的时间预算与规模自证。
 *
 * 调用方：tests/unit 下会递归读全仓源码的静态锁用例（bpm-definition-config-payload-lock、
 *         created-by-upload-locks、translated-value-select、ap-access-drift、role-matrix-baseline），
 *         各用例把自身 walker 得到的文件清单交给本模块记账，并把 SCAN_TIMEOUT_MS 作为该用例的超时。
 * 入参：label 用例自报名（用于日志点名），files 本次扫到的文件绝对路径清单，
 *       minFiles 该清单的规模下限，ms 该次扫描耗时。
 * 传给谁：断言失败信息进 vitest 报告；计时行进 CI 日志，供事后判断是"扫描缩水"还是"预算不够"。
 * 存什么：不存状态，纯校验与回报。
 * 存哪里：frontend/tests/unit/repo-scan-budget.ts，随单测一起被 vitest 收集。
 *
 * 为什么要这么一层。vitest.config.ts 未设 testTimeout，默认 5000ms，而全仓递归读文件的
 * 静态锁在冷跑下就要数秒（2026-10-28 实测 tests/unit/bpm-definition-config-payload-lock.test.ts
 * 的「反向锁：全仓 getUserList 不得以 page_size>100 凑全量」单条 9474ms，独自跑不并发），
 * 另两条同型用例（created-by-upload-locks 的 solid-waste-disposals 正向锁、
 * translated-value-select 的残留点位对账）在多条门禁抢同一块盘时从百毫秒级被放大过 5 秒。
 * 默认预算对这类用例就是错的，而"超时判红"与"断言判红"混在一条 exit code 里，
 * 看 CI 的人分不清是没修还是没跑完。
 *
 * 这里只做三件事，不缓存扫描结果：
 * 一 给这类用例一个有实测依据的统一预算；
 * 二 扫描规模低于下限时响亮报错，防"没读到文件"被当成"没发现违规"的假绿
 *    （同一族的假零教训：ref 前缀写错测出 0 命中）；
 * 三 把耗时与文件数打成一行的可观测记录，并在逼近预算时直接判红，
 *    让"预算该重估"这件事在还有余量时就被说出来。
 * 不缓存是因为这类锁里有用例要先注入违例夹具再扫（变异自证），缓存会把自证糊成绿。
 */

/** 扫描型静态锁的单用例时间预算。依据：冷跑最慢单条约 9.5 秒，取约 12 倍余量。 */
export const SCAN_TIMEOUT_MS = 120_000;

/** 逼近预算即判红的比例线，越过它说明预算或扫描方式需要重估，而不是继续加时。 */
const BUDGET_PRESSURE_RATIO = 0.8;

/**
 * 校验一次全仓扫描的规模，防"没读到"伪装成"没违规"。
 *
 * 调用方 各扫描型用例在拿到文件清单后立刻调用。
 * 入参 label 用例自报名，files walker 产出的路径清单，minFiles 该清单的规模下限。
 * 传给谁 抛错进 vitest 断言失败通道。
 * 存什么 不存。
 * 存哪里 不适用。
 */
export function guardScanSize(label: string, files: readonly string[], minFiles: number): void {
  if (files.length < minFiles) {
    throw new Error(
      `扫描规模异常 ${label} 只取到 ${files.length} 个文件，下限 ${minFiles}。` +
        `这是"没读到"而不是"没违规"，判红并要求核对 walker 的根路径与文件后缀。`
    );
  }
}

/**
 * 回报一次扫描的耗时与规模，并在逼近预算时判红。
 *
 * 调用方 各扫描型用例在扫完并判定后调用，ms 由 performance.now 差或 Date.now 差给出。
 * 入参 label 用例自报名，ms 本次扫描毫秒数，files 参与扫描的文件数。
 * 传给谁 console.info 进 CI 日志；判红线进 vitest 断言失败通道。
 * 存什么 不存。
 * 存哪里 不适用。
 */
export function reportScanElapsed(label: string, ms: number, files: number): void {
  console.info(`[scan-budget] ${label} files=${files} elapsedMs=${Math.round(ms)} budgetMs=${SCAN_TIMEOUT_MS}`);
  if (ms > SCAN_TIMEOUT_MS * BUDGET_PRESSURE_RATIO) {
    throw new Error(
      `扫描逼近预算 ${label} 耗时 ${Math.round(ms)}ms，已超 ${SCAN_TIMEOUT_MS}ms 的八成，` +
        `预算或扫描方式需要重估（先看是不是每次用例都整仓重扫），不要靠继续加时掩盖。`
    );
  }
}

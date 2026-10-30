/**
 * 夹具功能，反向自证样本，含 .only( 泄漏形态，证明 skip 与 only 对账判据
 * 遇 only 必判红而不是静默通过。
 * 调用方，本地反向自证命令与门禁判定脚本的夹具演练。
 * 入参，无，本文件只被判定脚本当作源码扫描样本读取，不参与任何跑测。
 * 传给谁，scripts/ci/check-e2e-shard-red-forensics.mjs 的 skip-only 模式。
 * 存什么，一段含 test.only( 泄漏形态的最小代码样本，外加两处只写在注释里
 * 的 test.skip( 干扰形态，自证注释剔除后不误计（本文件 skip 计数的期望值
 * 是 0，only 计数的期望值是 1）。
 * 存哪里，scripts/ci/fixtures/e2e-shard-forensics/only-leak-src/e2e/ 目录，
 * 不进入 frontend/e2e 扫描面，不会被真实跑测收集。
 */
export const fixtureName = 'only-leak';

/* 块注释里转述的 test.skip() 必须不被计入，否则棘轮基线会被注释文本污染 */

// 行注释里转述的 test.skip( 同样必须不被计入。
export function leakShape(test: { only: (title: string, fn: () => void) => void }) {
  test.only('泄漏的 only 用例', () => {
    // 故意保留 only 形态，判定脚本必须在这里报红。
  });
}

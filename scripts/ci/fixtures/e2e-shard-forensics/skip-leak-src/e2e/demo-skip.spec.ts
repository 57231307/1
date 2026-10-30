/**
 * 夹具功能，反向自证样本，skip 类计数高于基线，证明只降不升棘轮会
 * 点名文件与超出数。
 * 调用方，本地反向自证命令与门禁判定脚本的夹具演练。
 * 入参，无，本文件只被判定脚本当作源码扫描样本读取，不参与任何跑测。
 * 传给谁，scripts/ci/check-e2e-shard-red-forensics.mjs 的 skip-only 模式。
 * 存什么，两处 test.skip( 与一处 describe.skip( 的活跃代码形态样本。
 * 存哪里，scripts/ci/fixtures/e2e-shard-forensics/skip-leak-src/e2e/ 目录，
 * 不进入 frontend/e2e 扫描面，不会被真实跑测收集。
 */
export const fixtureName = 'skip-leak';

interface TestLike {
  skip: (title: string, fn?: () => void) => void;
  describe: { skip: (title: string, fn: () => void) => void };
}

export function leakShape(test: TestLike) {
  test.skip('基线外新增的跳过用例甲');
  test.skip('基线外新增的跳过用例乙');
  test.describe.skip('基线外新增的跳过套件', () => {
    // 套件整体跳过形态，判定脚本必须计进 skip 总数。
  });
}

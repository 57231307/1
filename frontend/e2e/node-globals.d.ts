/**
 * e2e 程序专用的 Node 全局类型建模（最小面、故意不引 @types/node）。
 *
 * 背景：Playwright 的 spec/helper/globalSetup 运行在 Node，但本仓 #27 裁定不在本轮引入
 * `@types/node`——它会连带把此前不判定的表达式全部纳入检查，需单独 PR 逐个判责。
 * 于是 `process`/`Buffer`/`node:fs` 长期报 TS2580/TS2307，被 `check-e2e-types.mjs`
 * 按 `文件|错误码` 记账成棘轮基线。
 *
 * 本文件的做法：只声明 e2e 代码里**实际用到**的成员（`.env` / `.cwd()` / `.exit()`），
 * 不伪造 Node 的完整 API 面。用 `node.<member>` 的新代码若不在此声明会直接判红，
 * 由人补声明——即"扩展面必须显式评审"，而不是静默放行。
 *
 * 放置位置的约束也有意义：本文件在 e2e 目录内，只被 tsconfig.e2e.json（include 覆盖 e2e
 * 目录下全部 ts 文件）纳入；tsconfig.json 只含 src 目录下的 ts/tsx/vue 与根 env.d.ts，
 * 因此应用侧代码误用 process.env（浏览器运行期根本不存在）仍会照判 TS2580，
 * 不会因这里的声明而被掩盖。
 */

/** 环境变量：未设置时为 undefined，调用点必须显式给出默认值或处理 undefined */
interface NodeProcessEnv {
  [key: string]: string | undefined;
}

declare const process: {
  readonly env: NodeProcessEnv;
  /** 只读工作目录（globalSetup 写凭证文件、报错回显 cwd 用） */
  cwd(): string;
  /** 结束进程（ensure-role-users 脚本的退出路径） */
  exit(code?: number): never;
};

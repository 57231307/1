/**
 * e2e 程序专用的 Node 全局类型建模（最小面、故意不引 @types/node）。
 *
 * 背景：Playwright 的 spec/helper/globalSetup 运行在 Node，但本仓 裁定不在本轮引入
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

/**
 * execSync 最小面声明（本文件既定流程：新代码用 Node API 须在此显式补声明）。
 * 使用方与用到的成员逐一对应，不伪造完整 child_process API 面：
 * - global-setup.ts::ensureGlobalBusinessSeed 本位币种子（币种无创建端点，只能 psql 直连写库）：
 *   command 字符串 + options { input, stdio(数组形), env }，返回体 .toString()；
 * - setup-wizard/00-setup-wizard.spec.ts 数据级校验与后端重启脚本：
 *   options { cwd, stdio: 'inherit', env }，返回体 .toString()。
 * 失败路径：Node 抛出的 error 带 stderr/ message，调用点按此窄形状读取。
 */
interface NodeExecSyncOptions {
  input?: string;
  cwd?: string;
  env?: Record<string, string | undefined>;
  stdio?: 'inherit' | Array<'pipe' | 'ignore' | 'inherit'>;
  encoding?: string;
  maxBuffer?: number;
}

interface NodeExecSyncResult {
  toString(encoding?: string): string;
}

declare module 'child_process' {
  export function execSync(command: string, options?: NodeExecSyncOptions): NodeExecSyncResult;
}

/**
 * fs 最小面声明（与上面 child_process 同一既定流程：e2e 用 Node API 必须在此显式补声明，
 * 而不是让 `import 'fs'` 长期挂 TS2307 使该文件整段脱离类型检查）。
 * 成员-调用点逐一对应，未用到的 API 一律不声明：
 * - flow/32-import-export.spec.ts、purchase/sku-mapping.spec.ts：writeFileSync 生成导入用临时 csv；
 * - global-setup.ts、traversal/44-role-matrix.spec.ts：mkdirSync(recursive) + writeFileSync 落凭证/access-map；
 * - flow/helpers.ts、traversal/permission-model.ts、setup-wizard/00-setup-wizard.spec.ts：
 *   existsSync 判在册 + readFileSync(path, 'utf-8') 读回；
 * - flow/32-import-export.spec.ts：statSync(...).size 断下载文件非空。
 * 失败路径：这些 API 抛的是 Node 的 Error，调用点按 try/catch 或前置 existsSync 处理，本声明不改变运行期行为。
 */
declare module 'fs' {
  export function existsSync(path: string): boolean;
  export function mkdirSync(path: string, options?: { recursive?: boolean }): void;
  export function readFileSync(path: string, options: string | { encoding: string }): string;
  export function readFileSync(path: string): Uint8Array;
  export function statSync(path: string): { size: number };
  export function writeFileSync(path: string, data: string, options?: string): void;
}

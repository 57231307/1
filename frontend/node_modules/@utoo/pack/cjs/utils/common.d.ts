export { debounce, isWellKnownError, ModuleBuildError, processIssues, rustifyEnv, } from "@utoo/pack-shared";
export declare function blockStdout(): void;
/**
 * Pack 根目录（pack 包所在目录）。
 * - CJS：使用 __dirname。
 * - ESM：在构建后由脚本注入 __dirname（基于 import.meta.url），此处仅用 __dirname 以保持 CJS 产物不含 import.meta，避免 Node 将 .js 判为 ESM。
 */
export declare function getPackPath(): string;

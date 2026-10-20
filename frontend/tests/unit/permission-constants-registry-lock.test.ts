/**
 * 前端权限常量表与后端权限注册表/动作词表的一致性静态锁。
 * 功能：把 frontend/src/constants/permissions.ts 的 PERMISSIONS 枚取值与
 * backend/src/services/init_service.rs 的 PERMISSION_RESOURCES 注册表、
 * backend/src/middleware/permission.rs 的可派生动作全集
 * （方法派生 read/create/update/delete ∪ PATH_ACTION_KEYWORDS ∪ QUERY_ACTION_KEYWORDS）
 * 三者互锁：枚取值必须是「资源名:动作」单冒号形状、资源名 kebab-case 且在注册表在册、
 * 动作在该集合内；粗域键（后端永不派生、仅由 router 别名表解释的域级键）以显式白名单
 * 登记，白名单外的任何非派生键一律判红。
 * 背景判据：运行时匹配是等值比较、无行即拒，写成点号/下划线/注册表外名的表达式恒假或恒真，
 * 这类漂移只能靠静态锁拦在提交前。
 * 调用方：vitest（tests/unit，CI vitest job）。
 * 入参：仅 fs 读取仓库内源码文本并 import 常量表，不执行后端代码、不发网络请求。
 * 传给谁：纯断言，无下游；存什么/存哪里：不落盘、不存储。
 */
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { PERMISSIONS } from '@/constants/permissions';

const TEST_DIR = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND_ROOT = path.resolve(TEST_DIR, '..', '..');
const REPO_ROOT = path.resolve(FRONTEND_ROOT, '..');

/** 去 BOM 读源码文本；Rust 文件在 Windows 工具链下可能带 BOM，不去则首行正则失配 */
function readRepoSource(absPath: string): string {
  return readFileSync(absPath, 'utf8').replace(/^\uFEFF/, '');
}

const INIT_SERVICE_RS = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/services/init_service.rs')
);
const PERMISSION_MIDDLEWARE_RS = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/middleware/permission.rs')
);

/** 截取 Rust 常量数组字面量体内的字符串项（从声明处到该数组首个右括号） */
function extractRustStringArray(source: string, constName: string): string[] {
  const start = source.indexOf(constName);
  if (start === -1) {
    throw new Error(`后端源码中找不到常量 ${constName}（后端改名必须同步本锁）`);
  }
  const end = source.indexOf('];', start);
  if (end === -1) {
    throw new Error(`常量 ${constName} 的数组字面量未闭合`);
  }
  return [...source.slice(start, end).matchAll(/"([a-z0-9_-]+)"/g)].map(m => m[1]);
}

const registry = new Set(extractRustStringArray(INIT_SERVICE_RS, 'PERMISSION_RESOURCES'));

const pathActions = extractRustStringArray(PERMISSION_MIDDLEWARE_RS, 'PATH_ACTION_KEYWORDS');
const queryActions = extractRustStringArray(PERMISSION_MIDDLEWARE_RS, 'QUERY_ACTION_KEYWORDS');
// 方法派生动作：从 ACTION_* 常量取值提取，禁止在本锁里另写一份动作字面量
const methodActions = [
  ...PERMISSION_MIDDLEWARE_RS.matchAll(
    /pub\(crate\) const ACTION_(READ|CREATE|UPDATE|DELETE): &str = "([a-z]+)"/g
  ),
].map(m => m[2]);
expect(methodActions).toHaveLength(4);
if (methodActions.length !== 4) {
  throw new Error('后端方法派生动作常量增删未同步本锁');
}
const assignableActions = new Set([...methodActions, ...pathActions, ...queryActions]);

/**
 * 粗域键白名单：这些键后端不可派生、不在注册表，由 router 守卫的粗域别名表解释，
 * 属设计如此的历史遗留；新增粗键必须走统一裁定，不允许随手入列。
 */
const COARSE_DOMAIN_KEYS: readonly string[] = ['finance:read'];

describe('PERMISSIONS 常量表与后端派生契约静态锁', () => {
  const entries = Object.entries(PERMISSIONS);

  it('常量表非空且全部为单冒号「资源:动作」形状（禁点号、禁无冒号、禁多冒号）', () => {
    expect(entries.length).toBeGreaterThan(0);
    for (const [name, value] of entries) {
      expect(value, `${name}=${value}`).toMatch(/^[a-z0-9-]+:[a-z*]+$/);
    }
  });

  it('每枚动作段都落在后端可派生动作全集（方法派生 ∪ 路径末段关键字 ∪ 查询白名单）', () => {
    for (const [name, value] of entries) {
      if (COARSE_DOMAIN_KEYS.includes(value)) continue;
      const action = value.split(':')[1];
      expect(assignableActions.has(action), `${name}=${value} 动作 ${action} 后端不可派生`).toBe(
        true
      );
    }
  });

  it('每枚资源段都在后端 PERMISSION_RESOURCES 注册表在册（粗域白名单除外）', () => {
    for (const [name, value] of entries) {
      if (COARSE_DOMAIN_KEYS.includes(value)) continue;
      const resource = value.split(':')[0];
      expect(registry.has(resource), `${name}=${value} 资源 ${resource} 不在册`).toBe(true);
    }
  });

  it('资源段命名与后端权威名同词形：kebab-case（禁下划线/大写）', () => {
    for (const [name, value] of entries) {
      if (COARSE_DOMAIN_KEYS.includes(value)) continue;
      const resource = value.split(':')[0];
      expect(resource, `${name}=${value} 资源段非 kebab-case`).toMatch(
        /^[a-z][a-z0-9]*(-[a-z0-9]+)*$/
      );
    }
  });

  it('锁自身的检测力：注册表外资源、非法动作、点号写法都必须被拒', () => {
    // 用三组故意漂移的样本走同一判据，证明上述断言不是恒真摆设
    const bogusResource = 'nonexistent_resource:read';
    const bogusAction = 'users:transfer';
    const dottedKey = 'finance.ap_invoices.read';
    expect(registry.has(bogusResource.split(':')[0])).toBe(false);
    expect(assignableActions.has(bogusAction.split(':')[1])).toBe(false);
    expect(/^[a-z0-9-]+:[a-z*]+$/.test(dottedKey)).toBe(false);
  });
});

/**
 * 应付域门控静态锁的取源与解析工具（供 tests/unit 下的门控锁与可达集复算用例共用）。
 *
 * 为什么单独成文件：两份用例都要读同一批权威源文本（后端权限资源注册表、后端角色授权分组、
 * router 的粗域展开表、应付壳页的页签登记表），解析口径只允许有一份；
 * 各写一套正则就会出现"两份用例对同一个源给出不同结论"的假绿。
 *
 * 只读文本、不执行后端代码、不发网络请求。
 * 行尾归一：Windows 检出为 CRLF、CI 检出为 LF，含 `\n` 的字面量夹具对行尾敏感，
 * 不归一会让本地跑出的解析结果与 CI 不一致（历史 CRLF 假红族）。
 */
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
export const FRONTEND_ROOT = path.resolve(HERE, '../..');
export const REPO_ROOT = path.resolve(FRONTEND_ROOT, '..');

// 以下路径一律相对**仓库根**（不是 frontend/），避免出现两种基准混用。

/** 后端权限资源注册表（资源名唯一权威清单） */
export const BACKEND_REGISTRY_RS = 'backend/src/services/init_service.rs';
/** 后端角色授权矩阵（哪个角色持有哪些资源×动作） */
export const BACKEND_MATRIX_RS = 'backend/src/services/init_service_ops/permission.rs';
/** 前端路由守卫（含粗域展开表 COARSE_DOMAIN_RESOURCES） */
export const ROUTER_TS = 'frontend/src/router/index.ts';
/** 应付壳页容器（页签登记表 AP_TABS 的唯一形态来源） */
export const AP_SHELL_VUE = 'frontend/src/views/ap/index.vue';
/** e2e 权限模型（router 粗域表的同源镜像 + ROUTE_PERMISSIONS 人工评审锚） */
export const PERMISSION_MODEL_TS = 'frontend/e2e/traversal/permission-model.ts';

export function readRepoSource(relPath: string): string {
  return readFileSync(path.join(REPO_ROOT, relPath), 'utf8').replace(/\r\n/g, '\n');
}

/** 取源文本中 `[startMarker, endMarker)` 之间的一段；任一标记缺失即抛错（防解析静默失去覆盖）。 */
export function sliceBetween(
  src: string,
  startMarker: string,
  endMarker: string,
  label: string
): string {
  const start = src.indexOf(startMarker);
  if (start === -1) {
    throw new Error(
      `${label}：未找到起始标记 "${startMarker}"——源结构已漂移，锁需同步修订，禁止静默放行`
    );
  }
  const end = src.indexOf(endMarker, start);
  if (end === -1) {
    throw new Error(`${label}：未找到终止标记 "${endMarker}"（自起始标记之后）`);
  }
  return src.slice(start, end);
}

/**
 * 解析 PERMISSION_RESOURCES 数组里的资源名。
 * 口径：只认"独占一行、整行就是 `"名字",`" 的行——数组体内夹带的说明行（// 开头、含反引号或
 * 中文）一律不会被误当成资源名，注册表名与注释文本彼此污染不了。
 */
export function parseBackendResourceRegistry(src: string): string[] {
  const body = sliceBetween(
    src,
    'pub const PERMISSION_RESOURCES: &[&str] = &[',
    '\n];',
    'PERMISSION_RESOURCES'
  );
  return [...body.matchAll(/^ {4}"([a-z0-9][a-z0-9-]*)",$/gm)].map(m => m[1]);
}

/**
 * 解析后端角色授权分组：`("角色码", &[("资源","动作"), ...])` → 角色码 → `${资源}:${动作}` 列表。
 * 矩阵里没出现的角色码（如靠 *:* 通配放行的 admin）返回 undefined，由调用方按"未在册"处置。
 */
export function parseBackendRoleGrantGroups(src: string): Record<string, string[]> {
  const groups: Record<string, string[]> = {};
  for (const m of src.matchAll(/"([a-z_0-9]+)",\s*&\[([\s\S]*?)\],\s*\)/g)) {
    const pairs = [...m[2].matchAll(/\("([a-z0-9_-]+)",\s*"([a-z*_-]+)"\)/g)].map(
      p => `${p[1]}:${p[2]}`
    );
    if (pairs.length === 0) continue;
    groups[m[1]] = pairs;
  }
  return groups;
}

/**
 * 解析 COARSE_DOMAIN_RESOURCES 对象体：域名 → 细资源段列表。
 * 供两处共用：router 源文本（守卫实际消费的表）与 permission-model 源文本（人工镜像表），
 * 两者必须逐字一致，由锁比对。
 * 数组体内同样只认独占一行、整行为 `'名字',` 的行，说明行不入集。
 */
export function parseCoarseDomainResources(src: string): Record<string, string[]> {
  const body = sliceBetween(
    src,
    'COARSE_DOMAIN_RESOURCES: Record<string, string[]> = {',
    '\n};',
    'COARSE_DOMAIN_RESOURCES'
  );
  const out: Record<string, string[]> = {};
  for (const m of body.matchAll(/^ {2}([a-z]+): \[([\s\S]*?)\],$/gm)) {
    out[m[1]] = [...m[2].matchAll(/^ {4}'([a-z0-9][a-z0-9-]*)',$/gm)].map(x => x[1]);
  }
  return out;
}

/** 解析 ROUTE_PERMISSIONS 里某个模块的声明值（字符串或字符串数组），缺条目返回 undefined。 */
export function parseRoutePermissionEntry(
  src: string,
  modId: string
): string | string[] | undefined {
  const body = sliceBetween(
    src,
    'export const ROUTE_PERMISSIONS: Record<string, RoutePermValue> = {',
    '\n};',
    'ROUTE_PERMISSIONS'
  );
  const single = body.match(new RegExp(`^ {2}${modId}: '([^']*)',$`, 'm'));
  if (single) return single[1];
  const arr = body.match(new RegExp(`^ {2}${modId}: \\[([\\s\\S]*?)\\],$`, 'm'));
  if (arr) return [...arr[1].matchAll(/'([^']*)'/g)].map(x => x[1]);
  return undefined;
}

/**
 * 解析应付壳页 AP_TABS 登记表：页签名 → 入口权限码列表。
 * 入口值在源码里写成常量标识符（PERMISSIONS.X / AP_REPORT_TAB_PERMISSIONS），
 * 这里只做词法抽取，把标识符换成真实常量值的工作交给调用方（锁用例持有 constants 模块），
 * 以免解析器与常量定义之间出现第二份取值。
 */
export function parseApTabRegistrations(src: string): Array<{ name: string; identifier: string }> {
  const body = sliceBetween(src, 'const AP_TABS = [', '\n] as const;', 'AP_TABS');
  return [...body.matchAll(/\{ name: '([a-zA-Z]+)', entry: ([A-Za-z_.]+) \}/g)].map(m => ({
    name: m[1],
    identifier: m[2],
  }));
}

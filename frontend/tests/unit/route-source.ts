/**
 * 路由表源码解析共享件（外壳叶子路由的唯一解析器）
 *
 * 功能：把 `src/router/index.ts` 里挂在 MainLayout 外壳下的叶子路由解析成结构化条目
 *   （path、component、title、permission），并派生出"可路由叶子 path 集"。
 * 为什么要有这个文件：解析规则只能有一份。导航闭合锁与入口重叠登记册都要读同一批路由，
 *   各自写一遍正则与括号配对就会漂移——本仓已多次因"同一事实两处表达"漏同步而静默少跑。
 * 调用方：frontend/tests/unit/navigation-closure.test.ts（闭合断言）、
 *   frontend/tests/unit/route-overlap-registry.test.ts（重叠登记册棘轮）。
 * 入参：仓库内源码文本；所有读取统一把 CRLF 归一为 LF，本地检出与 CI 检出看到同一份输入。
 * 传给谁：返回条目数组或 path 集合，纯函数，无副作用。
 * 存什么/存哪里：不落盘、不存储。
 *
 * 解析口径（与抽取前的 navigation-closure 保持一致，未放宽任何一条）：
 *   - 只认 `component: MainLayout` 那条路由的 children 数组（应用外壳内）；
 *   - 逐个顶层对象做花括号配对，嵌套 children 并进父对象文本，因此取到的是父级 path；
 *   - 无 `component:` 的（纯 redirect 容器）不参与叶子集；
 *   - `hidden: true` 与 `public: true` 排除在叶子集外。
 */
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
export const FRONTEND_ROOT = path.resolve(HERE, '../..');

/** 读仓库内前端源码并把行尾归一为 LF */
export function readFrontendSource(relativePath: string): string {
  return readFileSync(path.join(FRONTEND_ROOT, relativePath), 'utf8').replace(/\r\n/g, '\n');
}

export const ROUTER_SRC = readFrontendSource('src/router/index.ts');

/** 外壳叶子路由条目 */
export interface ShellRouteEntry {
  path: string;
  component: string;
  title: string;
  permission: string;
}

/** 从对象文本里取字符串字面量字段，取不到给空串 */
function literal(obj: string, key: string): string {
  const matched = obj.match(new RegExp(`${key}:\\s*['"]([^'"]*)['"]`));
  return matched ? matched[1] : '';
}

/** 解析 MainLayout 外壳下的叶子路由条目 */
export function collectShellRouteEntries(src: string): ShellRouteEntry[] {
  const anchor = src.indexOf('component: MainLayout,');
  if (anchor < 0) throw new Error('router 未找到 MainLayout 容器路由');
  const open = src.indexOf('[', src.indexOf('children:', anchor));
  if (open < 0) throw new Error('MainLayout 路由缺少 children 数组');
  let depth = 0;
  let close = -1;
  for (let i = open; i < src.length; i += 1) {
    if (src[i] === '[') depth += 1;
    else if (src[i] === ']') {
      depth -= 1;
      if (depth === 0) {
        close = i;
        break;
      }
    }
  }
  if (close < 0) throw new Error('MainLayout children 数组未闭合');

  let objDepth = 0;
  let buf = '';
  const objects: string[] = [];
  for (const ch of src.slice(open + 1, close)) {
    if (ch === '{') objDepth += 1;
    if (objDepth > 0) buf += ch;
    if (ch === '}') {
      objDepth -= 1;
      if (objDepth === 0) {
        objects.push(buf);
        buf = '';
      }
    }
  }

  const leaves: ShellRouteEntry[] = [];
  for (const obj of objects) {
    const pathMatch = obj.match(/path:\s*['"]([^'"]+)['"]/);
    if (!pathMatch || !obj.includes('component:')) continue;
    if (/hidden:\s*true/.test(obj) || /public:\s*true/.test(obj)) continue;
    const raw = pathMatch[1];
    const full = raw.startsWith('/') ? raw : `/${raw}`;
    const componentMatch = obj.match(
      /component:\s*(?:\(\)\s*=>\s*import\(\s*['"]([^'"]+)['"]\s*\)|([\w$.]+))/
    );
    leaves.push({
      path: full,
      component: componentMatch?.[1] ?? componentMatch?.[2] ?? full,
      title: literal(obj, 'title'),
      permission: literal(obj, 'permission'),
    });
  }
  return leaves;
}

/** 可路由叶子 path 集（闭合锁用它与导航树双向比对） */
export function collectShellLeaves(src: string): Set<string> {
  return new Set(collectShellRouteEntries(src).map(entry => entry.path));
}

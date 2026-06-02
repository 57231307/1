/**
 * 懒加载助手 - 用于优化页面并发请求
 *
 * 使用场景：页面有多个 Tab 或多个数据块时，按需加载而非同时并发
 *
 * @example
 * // 基本用法
 * const hasLoaded = createLazyLoader()
 * onMounted(() => loadIfNot('key', fetchData))
 *
 * @example
 * // Tab 场景
 * const activeTab = ref('tab1')
 * const hasLoaded = createLazyLoader()
 * <el-tabs @tab-change="(tab) => loadTab(tab, hasLoaded)">
 *
 * const loadTab = (tabName: string, loader: Record<string, () => void>) => {
 *   loadIfNot(tabName, loader[tabName], hasLoaded)
 * }
 */

export interface LazyLoadState {
  [key: string]: boolean
}

/**
 * 创建懒加载状态对象
 */
export function createLazyLoader(): LazyLoadState {
  return {}
}

/**
 * 如果未加载过则执行加载函数
 * @param key 加载标识
 * @param loadFn 加载函数
 * @param state 懒加载状态对象
 */
export function loadIfNot(
  key: string,
  loadFn: () => void | Promise<void>,
  state: LazyLoadState
): void {
  if (!state[key]) {
    state[key] = true
    loadFn()
  }
}

/**
 * 重置某个 key 的加载状态（用于刷新）
 */
export function resetLoad(key: string, state: LazyLoadState): void {
  state[key] = false
}

/**
 * 标记为已加载
 */
export function markLoaded(key: string, state: LazyLoadState): void {
  state[key] = true
}

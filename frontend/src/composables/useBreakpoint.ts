/**
 * V15 P1-20-2 响应式断点 composable
 *
 * 业务背景：审计报告 batch-20 维度 24.1 缺陷 2 指出 MainLayout 未做侧边栏抽屉化。
 * 项目未引入 @vueuse/core，使用原生 window.matchMedia 实现响应式断点检测，
 * 支持在 SSR/测试环境降级（matchMedia 不存在时默认 PC 视图）。
 *
 * 断点定义（与 Element Plus 一致）：
 * - xs: < 768px（手机竖屏）
 * - sm: >= 768px（手机横屏/小平板）
 * - md: >= 992px（平板）
 * - lg: >= 1200px（桌面）
 * - xl: >= 1920px（大桌面）
 *
 * 业务规则：
 * - md 以下（< 992px）视为移动端，侧边栏改用 el-drawer 抽屉化
 * - md 及以上保持原 el-aside 固定侧边栏
 */
import { ref, onMounted, onBeforeUnmount, type Ref } from 'vue';

/** 断点像素值 */
export const BREAKPOINTS = {
  xs: 0,
  sm: 768,
  md: 992,
  lg: 1200,
  xl: 1920,
} as const;

/** 响应式断点状态 */
export interface BreakpointState {
  /** 当前是否为移动端（width < md） */
  isMobile: Ref<boolean>;
  /** 当前是否为平板（md <= width < lg） */
  isTablet: Ref<boolean>;
  /** 当前是否为桌面（width >= lg） */
  isDesktop: Ref<boolean>;
  /** 当前视口宽度（px） */
  width: Ref<number>;
}

/**
 * 创建 matchMedia 监听器（内部 helper）
 * 返回 cleanup 函数用于卸载时移除监听
 */
function createMediaQueryListener(query: string, onChange: (matches: boolean) => void): () => void {
  if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') {
    return () => {};
  }
  const mql = window.matchMedia(query);
  onChange(mql.matches);
  const handler = (e: MediaQueryListEvent) => onChange(e.matches);
  if (mql.addEventListener) {
    mql.addEventListener('change', handler);
    return () => mql.removeEventListener('change', handler);
  }
  // 兼容旧浏览器（Safari < 14）
  mql.addListener(handler);
  return () => mql.removeListener(handler);
}

/**
 * 响应式断点 composable
 *
 * 用法：
 * ```ts
 * const { isMobile } = useBreakpoint()
 * if (isMobile.value) { ... }
 * ```
 *
 * 实现细节：
 * - 使用 matchMedia 监听断点变化（性能优于 resize 事件）
 * - 初始值在 onMounted 时设置（避免 SSR 不一致）
 * - onBeforeUnmount 自动清理监听器
 */
export function useBreakpoint(): BreakpointState {
  const isMobile = ref(false);
  const isTablet = ref(false);
  const isDesktop = ref(false);
  const width = ref(typeof window !== 'undefined' ? window.innerWidth : 1280);

  let cleanups: Array<() => void> = [];

  onMounted(() => {
    if (typeof window === 'undefined') return;
    width.value = window.innerWidth;

    // 移动端：width < md (992px)
    cleanups.push(
      createMediaQueryListener(`(max-width: ${BREAKPOINTS.md - 1}px)`, matches => {
        isMobile.value = matches;
      })
    );

    // 平板：md <= width < lg
    cleanups.push(
      createMediaQueryListener(
        `(min-width: ${BREAKPOINTS.md}px) and (max-width: ${BREAKPOINTS.lg - 1}px)`,
        matches => {
          isTablet.value = matches;
        }
      )
    );

    // 桌面：width >= lg
    cleanups.push(
      createMediaQueryListener(`(min-width: ${BREAKPOINTS.lg}px)`, matches => {
        isDesktop.value = matches;
      })
    );

    // 视口宽度（resize 时更新，供组件按需响应）
    const resizeHandler = () => {
      width.value = window.innerWidth;
    };
    window.addEventListener('resize', resizeHandler);
    cleanups.push(() => window.removeEventListener('resize', resizeHandler));
  });

  onBeforeUnmount(() => {
    cleanups.forEach(cleanup => cleanup());
    cleanups = [];
  });

  return { isMobile, isTablet, isDesktop, width };
}

export default useBreakpoint;

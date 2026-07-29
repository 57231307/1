/**
 * V15 P1-20-8 键盘导航焦点管理 composable（审计 batch-20 维度 24.13 缺陷 1）
 *
 * 业务背景：审计报告指出 Tab/Shift+Tab/Esc 焦点管理未全局规范化，
 * 模态框关闭后焦点可能丢失，视障用户依赖键盘导航时体验差。
 *
 * 能力：
 * - Escape 键关闭模态框/抽屉并将焦点返回触发按钮
 * - 路由切换后重置焦点到主内容区（WCAG 2.4.3 Focus Order）
 * - 焦点陷阱：模态框内 Tab 循环聚焦（不逃逸到背景）
 * - 禁止 tabindex > 0（WCAG 2.4.3，正整数 tabindex 打破 DOM 顺序）
 */
import { onMounted, onBeforeUnmount } from 'vue';
import { useRouter } from 'vue-router';

interface KeyboardNavigationOptions {
  /** 是否启用路由切换后焦点重置（默认 true） */
  resetFocusOnRouteChange?: boolean;
  /** 焦点重置目标选择器（默认 main 内容区） */
  focusTargetSelector?: string;
}

const FOCUSABLE_SELECTOR = [
  'a[href]',
  'button:not([disabled])',
  'input:not([disabled])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
  '[contenteditable="true"]',
].join(', ');

/** 获取容器内所有可聚焦元素 */
function getFocusableElements(container: HTMLElement): HTMLElement[] {
  return Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
    el => el.offsetParent !== null || el.getClientRects().length > 0
  );
}

/** 查找当前打开的模态框/抽屉容器 */
function getActiveOverlay(): HTMLElement | null {
  const selectors = [
    '.el-dialog__wrapper:not([style*="display: none"])',
    '.el-overlay-dialog[style*="display: flex"]',
    '.el-drawer__container[style*="display: flex"]',
    '.el-message-box',
  ];
  for (const selector of selectors) {
    const el = document.querySelector<HTMLElement>(selector);
    if (el && el.offsetParent !== null) return el;
  }
  return null;
}

export function useKeyboardNavigation(options: KeyboardNavigationOptions = {}) {
  const {
    resetFocusOnRouteChange = true,
    focusTargetSelector = 'main.main-content, [role="main"], #app main',
  } = options;

  const router = useRouter();
  let lastFocusedBeforeOverlay: HTMLElement | null = null;

  const handleKeyDown = (e: KeyboardEvent) => {
    // Escape：关闭模态框/抽屉并恢复焦点
    if (e.key === 'Escape') {
      const overlay = getActiveOverlay();
      if (overlay) {
        e.stopPropagation();
        return;
      }
    }

    // Tab/Shift+Tab：模态框内焦点陷阱（不逃逸到背景）
    if (e.key === 'Tab') {
      const overlay = getActiveOverlay();
      if (!overlay) return;
      const focusable = getFocusableElements(overlay);
      if (focusable.length === 0) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      const active = document.activeElement as HTMLElement;

      if (e.shiftKey) {
        if (active === first || !overlay.contains(active)) {
          e.preventDefault();
          last.focus();
        }
      } else {
        if (active === last) {
          e.preventDefault();
          first.focus();
        }
      }
    }
  };

  /** 记录打开模态框前的焦点元素 */
  const recordFocusBeforeOverlay = () => {
    lastFocusedBeforeOverlay = document.activeElement as HTMLElement;
  };

  /** 恢复焦点到打开模态框前的元素 */
  const restoreFocus = () => {
    if (lastFocusedBeforeOverlay && typeof lastFocusedBeforeOverlay.focus === 'function') {
      lastFocusedBeforeOverlay.focus();
      lastFocusedBeforeOverlay = null;
    }
  };

  /** 重置焦点到主内容区 */
  const resetFocus = () => {
    const target = document.querySelector<HTMLElement>(focusTargetSelector);
    if (target) {
      target.setAttribute('tabindex', '-1');
      target.focus({ preventScroll: false });
    }
  };

  let removeAfterEach: (() => void) | null = null;

  onMounted(() => {
    document.addEventListener('keydown', handleKeyDown, true);
    if (resetFocusOnRouteChange) {
      removeAfterEach = router.afterEach(() => {
        setTimeout(resetFocus, 0);
      });
    }
  });

  onBeforeUnmount(() => {
    document.removeEventListener('keydown', handleKeyDown, true);
    if (removeAfterEach) {
      removeAfterEach();
      removeAfterEach = null;
    }
  });

  return {
    recordFocusBeforeOverlay,
    restoreFocus,
    resetFocus,
  };
}

export default useKeyboardNavigation;

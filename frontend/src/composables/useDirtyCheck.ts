/**
 * V15 P1-20-11 表单脏数据检测 composable（审计 batch-20 维度 24.15 缺陷 2）
 *
 * 业务背景：审计报告指出所有表单组件未实现"表单有未保存修改时离开提示"，
 * 用户填写大表单后误点其他菜单数据丢失。
 *
 * 能力：
 * - 监听表单数据变化，记录初始快照用于 dirty 比较
 * - 浏览器刷新/关闭时通过 beforeunload 事件提示用户
 * - 路由切换时通过 onBeforeRouteLeave 提示用户（可自定义提示文案）
 * - 提交成功或主动重置后清除 dirty 状态
 *
 * 用法：
 * ```ts
 * const { isDirty, initDirtyCheck, resetDirtyCheck } = useDirtyCheck()
 * initDirtyCheck(formModel)        // 初始化快照（formModel 为 reactive 对象）
 * // ... 用户编辑表单 ...
 * resetDirtyCheck()                // 提交成功后调用，清除 dirty 状态
 * ```
 */
import { ref, watch, onBeforeUnmount, type Ref } from 'vue';
import { onBeforeRouteLeave } from 'vue-router';

interface DirtyCheckOptions {
  /** 路由离开时的提示文案（默认"表单有未保存的修改，确定离开吗？"） */
  leaveMessage?: string;
  /** 是否启用浏览器 beforeunload 事件（默认 true） */
  enableBeforeUnload?: boolean;
}

interface DirtyCheckResult {
  /** 当前表单是否有未保存修改 */
  isDirty: Ref<boolean>;
  /** 初始化脏数据检测，传入表单的响应式 model 对象 */
  initDirtyCheck: (formModel: unknown) => void;
  /** 重置 dirty 状态（提交成功后调用），可传入新快照 */
  resetDirtyCheck: (newSnapshot?: unknown) => void;
  /** 手动设置 dirty 状态 */
  setDirty: (dirty: boolean) => void;
}

/** 深拷贝快照（JSON 序列化，足以覆盖表单对象场景） */
function snapshot(data: unknown): string {
  try {
    return JSON.stringify(data ?? null);
  } catch {
    return '';
  }
}

export function useDirtyCheck(options: DirtyCheckOptions = {}): DirtyCheckResult {
  const {
    leaveMessage = '表单有未保存的修改，确定离开吗？',
    enableBeforeUnload = true,
  } = options;

  const isDirty = ref(false);
  let initialSnapshot = '';
  let stopWatch: (() => void) | null = null;

  /** beforeunload 事件处理器（浏览器刷新/关闭） */
  const beforeunloadHandler = (e: BeforeUnloadEvent) => {
    if (isDirty.value) {
      e.preventDefault();
      e.returnValue = leaveMessage;
      return leaveMessage;
    }
  };

  /** 初始化脏数据检测 */
  const initDirtyCheck = (formModel: unknown) => {
    initialSnapshot = snapshot(formModel);
    isDirty.value = false;

    // 停止之前的监听（支持重复初始化）
    if (stopWatch) stopWatch();

    // 深度监听表单数据变化
    stopWatch = watch(
      () => formModel,
      newVal => {
        isDirty.value = snapshot(newVal) !== initialSnapshot;
      },
      { deep: true }
    );

    if (enableBeforeUnload && typeof window !== 'undefined') {
      window.addEventListener('beforeunload', beforeunloadHandler);
    }
  };

  /** 重置 dirty 状态 */
  const resetDirtyCheck = (newSnapshot?: unknown) => {
    initialSnapshot = snapshot(newSnapshot);
    isDirty.value = false;
  };

  /** 手动设置 dirty 状态 */
  const setDirty = (dirty: boolean) => {
    isDirty.value = dirty;
  };

  // 路由切换前提示（onBeforeRouteLeave 在组件 setup 内自动注册）
  onBeforeRouteLeave((_to, _from, next) => {
    if (isDirty.value) {
      if (window.confirm(leaveMessage)) {
        next();
      } else {
        next(false);
      }
    } else {
      next();
    }
  });

  // 组件卸载时清理监听
  onBeforeUnmount(() => {
    if (stopWatch) {
      stopWatch();
      stopWatch = null;
    }
    if (enableBeforeUnload && typeof window !== 'undefined') {
      window.removeEventListener('beforeunload', beforeunloadHandler);
    }
  });

  return {
    isDirty,
    initDirtyCheck,
    resetDirtyCheck,
    setDirty,
  };
}

export default useDirtyCheck;

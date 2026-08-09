/**
 * 事件总线 - batch-20 P3: 事件总线
 * 用于组件间通信，替代 Vuex actions
 */
type EventHandler = (...args: unknown[]) => void

class EventBus {
  private events: Map<string, Set<EventHandler>> = new Map()

  /** 注册事件监听器 */
  on(event: string, handler: EventHandler) {
    if (!this.events.has(event)) {
      this.events.set(event, new Set())
    }
    this.events.get(event)!.add(handler)
  }

  /** 注册一次性事件监听器 */
  once(event: string, handler: EventHandler) {
    const wrapper = (...args: unknown[]) => {
      handler(...args)
      this.off(event, wrapper)
    }
    this.on(event, wrapper)
  }

  /** 移除事件监听器 */
  off(event: string, handler: EventHandler) {
    const handlers = this.events.get(event)
    if (handlers) {
      handlers.delete(handler)
      if (handlers.size === 0) {
        this.events.delete(event)
      }
    }
  }

  /** 触发事件 */
  emit(event: string, ...args: unknown[]) {
    const handlers = this.events.get(event)
    if (handlers) {
      handlers.forEach(handler => {
        try {
          handler(...args)
        } catch (e) {
          console.error(`Event handler error for "${event}":`, e)
        }
      })
    }
  }

  /** 移除所有事件监听器 */
  clear() {
    this.events.clear()
  }
}

export const eventBus = new EventBus()

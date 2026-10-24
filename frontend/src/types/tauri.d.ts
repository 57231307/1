/**
 * 安卓客户端壳注入的 Tauri 全局桥声明。
 *
 * 调用方 远端 ERP 页面在不打包任何前端依赖的前提下经 window.__TAURI__ 调命令。
 * 传什么 命令名与参数对象，命令白名单由壳内 capabilities 控制。
 * 存什么 不存储，声明只给 TypeScript 提供类型。
 * 存哪里 不涉及持久化。
 */
interface TauriCoreBridge {
  invoke<T = unknown>(command: string, args?: Record<string, unknown>): Promise<T>;
}

interface TauriGlobalBridge {
  core?: TauriCoreBridge;
  invoke?: TauriCoreBridge['invoke'];
}

interface Window {
  __TAURI__?: TauriGlobalBridge;
}

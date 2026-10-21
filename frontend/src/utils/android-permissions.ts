/**
 * 安卓原生运行时权限申请桥。
 *
 * 调用方 扫码对话框在启动 getUserMedia 前调用，确保系统授权先于相机流建立。
 * 入参 无，命令按权限类型弹系统授权框，已授权时立即返回。
 * 传给谁 经 window.__TAURI__ 调 Rust 命令 app_request_camera_permission，
 *        再由安卓插件的 requestCameraPermission 走 Tauri 权限别名。
 * 存什么 不存储，授权结果由系统保存在应用权限表。
 * 存哪里 不落库、不写文件。
 */
import { isAndroidClient } from '@/utils/bluetooth-print';

export interface AndroidPermissionResult {
  granted: boolean;
  sdkInt: number;
}

function invokeAndroid<T>(command: string): Promise<T> {
  const bridge = window.__TAURI__;
  if (bridge?.core?.invoke) {
    return bridge.core.invoke<T>(command);
  }
  if (bridge?.invoke) {
    return bridge.invoke<T>(command) as Promise<T>;
  }
  return Promise.reject(new Error('当前环境没有安卓权限桥'));
}

/** 申请相机运行时权限，返回系统授权结果。 */
export function requestCameraPermission(): Promise<AndroidPermissionResult> {
  if (!isAndroidClient()) {
    return Promise.resolve({ granted: false, sdkInt: 0 });
  }
  return invokeAndroid<AndroidPermissionResult>('app_request_camera_permission');
}

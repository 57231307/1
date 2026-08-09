import { defineStore } from 'pinia';
import { ref } from 'vue';
import { request } from '@/api/request';

/** 系统信息 */
export interface SystemInfo {
  version: string;
  buildTime: string;
  gitHash: string;
  environment: string;
}

/** 系统配置 */
export interface SystemConfig {
  maintenanceMode: boolean;
  maxUploadSize: number;
  sessionTimeout: number;
  features: Record<string, boolean>;
}

/** 系统 store - batch-20 P3: system store */
export const useSystemStore = defineStore('system', () => {
  /** 系统信息 */
  const systemInfo = ref<SystemInfo | null>(null);

  /** 系统配置 */
  const systemConfig = ref<SystemConfig | null>(null);

  /** 加载状态 */
  const loading = ref(false);

  /** 错误信息 */
  const error = ref<string | null>(null);

  /** 获取系统信息 */
  async function fetchSystemInfo() {
    loading.value = true;
    error.value = null;
    try {
      const res = await request.get<SystemInfo>('/api/v1/erp/system/info');
      systemInfo.value = res;
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : '获取系统信息失败';
    } finally {
      loading.value = false;
    }
  }

  /** 获取系统配置 */
  async function fetchSystemConfig() {
    loading.value = true;
    error.value = null;
    try {
      const res = await request.get<SystemConfig>('/api/v1/erp/system/config');
      systemConfig.value = res;
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : '获取系统配置失败';
    } finally {
      loading.value = false;
    }
  }

  return {
    systemInfo,
    systemConfig,
    loading,
    error,
    fetchSystemInfo,
    fetchSystemConfig,
  };
});

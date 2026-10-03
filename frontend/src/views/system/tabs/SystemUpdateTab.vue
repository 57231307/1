<!--
  SystemUpdateTab.vue - 系统更新 Tab
  来源：原 system/index.vue 中 系统更新 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="system-update-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('system.systemUpdate.title') }}</h2>
    </div>
    <el-card shadow="hover">
      <el-descriptions :column="2" border>
        <el-descriptions-item :label="t('system.systemUpdate.label.currentVersion')">{{
          systemVersion
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('system.systemUpdate.label.lastUpdate')">{{
          lastUpdate
        }}</el-descriptions-item>
      </el-descriptions>
      <div style="margin-top: 20px">
        <el-button type="primary" :loading="checkUpdateLoading" @click="checkUpdate">
          {{ t('system.systemUpdate.button.check') }}
        </el-button>
        <el-button
          v-if="hasUpdate"
          type="success"
          :loading="applyUpdateLoading"
          @click="applyUpdate"
        >
          {{ t('system.systemUpdate.button.apply') }}
        </el-button>
      </div>
      <el-alert
        v-if="updateInfo"
        :title="updateInfo"
        type="info"
        show-icon
        style="margin-top: 16px"
      />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { logger } from '@/utils/logger';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { request } from '@/api/request';
import type { ApiResponse } from '@/types/api';

const { t } = useI18n({ useScope: 'global' });

// 对应 backend/src/handlers/system_update_handler.rs get_version → VersionResponse
interface VersionResponse {
  version: string;
  release_date: string;
  changelog: string | null;
}

// 对应 backend/src/handlers/system_update_handler.rs check_for_updates → CheckUpdateResponse
interface CheckUpdateResponse {
  has_update: boolean;
  current_version: string;
  latest_version: string;
  download_url: string | null;
  file_size: number | null;
  release_notes: string | null;
  published_at: string | null;
  current_release_notes: string | null;
  current_published_at: string | null;
}

const systemVersion = ref('-');
const lastUpdate = ref('-');
const hasUpdate = ref(false);
const updateInfo = ref('');
const checkUpdateLoading = ref(false);
const applyUpdateLoading = ref(false);

const checkUpdate = async () => {
  checkUpdateLoading.value = true;
  try {
    const res = await request.get<ApiResponse<CheckUpdateResponse>>('/system-update/check');
    hasUpdate.value = res.data.has_update;
    updateInfo.value = res.data.has_update
      ? t('system.systemUpdate.message.newVersion', { version: res.data.latest_version })
      : t('system.systemUpdate.message.upToDate');
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('system.systemUpdate.message.checkFailed'));
  } finally {
    checkUpdateLoading.value = false;
  }
};

const applyUpdate = async () => {
  applyUpdateLoading.value = true;
  try {
    await request.post('/system-update/update');
    ElMessage.success(t('system.systemUpdate.message.updateSubmitted'));
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('system.systemUpdate.message.updateFailed'));
  } finally {
    applyUpdateLoading.value = false;
  }
};

const fetchSystemVersion = async () => {
  try {
    const res = await request.get<ApiResponse<VersionResponse>>('/system-update/version');
    systemVersion.value = res.data.version;
    lastUpdate.value = res.data.release_date;
  } catch (_e) {
    logger.error(t('system.systemUpdate.message.loadVersionFailed'), _e);
  }
};

defineExpose({ refresh: fetchSystemVersion });

onMounted(() => {
  fetchSystemVersion();
});
</script>

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
          lastUpdate !== null ? lastUpdate : t('system.systemUpdate.value.lastUpdateUnknown')
        }}</el-descriptions-item>
      </el-descriptions>
      <div style="margin-top: 20px">
        <el-button
          v-permission="PERMISSIONS.SYSTEM_UPDATE_CHECK_READ"
          type="primary"
          :loading="checkUpdateLoading"
          @click="checkUpdate"
        >
          {{ t('system.systemUpdate.button.check') }}
        </el-button>
        <el-button
          v-if="hasUpdate"
          v-permission="PERMISSIONS.SYSTEM_UPDATE_UPDATE_CREATE"
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
import { logAuxLoadFailure } from '@/utils/logger';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { request } from '@/api/request';
import { applyUpdate as applyUpdateApi } from '@/api/system-update';
import { PERMISSIONS } from '@/constants/permissions';
import type { ApiResponse } from '@/types/api';

const { t } = useI18n({ useScope: 'global' });

// 对应 backend/src/handlers/system_update_handler.rs get_version → VersionResponse
interface VersionResponse {
  version: string;
  release_date: string | null;
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
const lastUpdate = ref<string | null>(null);
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
    // 后端 download_and_update 返回权威重启结论 data.message（是否已自退拉起 / 需人工重启），
    // 如实转述，不替后端断言"服务已重启"
    const res = await applyUpdateApi();
    updateInfo.value = res.data.message;
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
    logAuxLoadFailure(t('system.systemUpdate.message.loadVersionFailed'), _e);
  }
};

defineExpose({ refresh: fetchSystemVersion });

onMounted(() => {
  fetchSystemVersion();
});
</script>

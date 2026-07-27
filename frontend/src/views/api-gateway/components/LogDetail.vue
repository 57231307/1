<!--
  LogDetail.vue - API 调用日志详情对话框
  拆分自 api-gateway/index.vue（P14 批 1 B3 I-2）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('apiGateway.logDetail.title')"
    width="800px"
    :aria-label="t('apiGateway.logDetail.ariaLabel')"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-descriptions :column="2" border>
      <el-descriptions-item :label="t('apiGateway.logDetail.endpointPath')">{{
        currentLog?.endpoint_path
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('apiGateway.logDetail.method')">{{
        currentLog?.method
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('apiGateway.logDetail.statusCode')">{{
        currentLog?.status_code
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('apiGateway.logDetail.responseTime')"
        >{{ currentLog?.response_time }}ms</el-descriptions-item
      >
      <el-descriptions-item :label="t('apiGateway.logDetail.ipAddress')">{{
        currentLog?.ip_address
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('apiGateway.logDetail.user')">{{
        currentLog?.user_name
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('apiGateway.logDetail.requestTime')" :span="2">{{
        currentLog?.created_at
      }}</el-descriptions-item>
    </el-descriptions>
    <div class="log-section">
      <h4>{{ t('apiGateway.logDetail.requestBody') }}</h4>
      <pre>{{ currentLog?.request_body || t('apiGateway.logDetail.empty') }}</pre>
    </div>
    <div class="log-section">
      <h4>{{ t('apiGateway.logDetail.responseBody') }}</h4>
      <pre>{{ currentLog?.response_body || t('apiGateway.logDetail.empty') }}</pre>
    </div>
    <template #footer>
      <el-button @click="emit('update:visible', false)">{{
        t('apiGateway.logDetail.close')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { ApiLog } from '@/api/api-gateway';

const { t } = useI18n({ useScope: 'global' });

/**
 * 日志详情对话框组件
 * 仅做展示，对话框状态由父组件控制
 */
const props = defineProps<{
  visible: boolean;
  currentLog: ApiLog | null;
}>();

const emit = defineEmits<{
  'update:visible': [v: boolean];
}>();

void props;
</script>

<style scoped>
.log-section {
  margin-top: 16px;
}
.log-section h4 {
  margin-bottom: 8px;
  color: #303133;
}
.log-section pre {
  background-color: #f5f7fa;
  padding: 12px;
  border-radius: 4px;
  max-height: 200px;
  overflow-y: auto;
  font-size: 12px;
  line-height: 1.5;
}
</style>

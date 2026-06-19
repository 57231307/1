<!--
  LogDetail.vue - API 调用日志详情对话框
  拆分自 api-gateway/index.vue（P14 批 1 B3 I-2）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    title="日志详情"
    width="800px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-descriptions :column="2" border>
      <el-descriptions-item label="接口路径">{{
        currentLog?.endpoint_path
      }}</el-descriptions-item>
      <el-descriptions-item label="请求方法">{{ currentLog?.method }}</el-descriptions-item>
      <el-descriptions-item label="状态码">{{ currentLog?.status_code }}</el-descriptions-item>
      <el-descriptions-item label="响应时间"
        >{{ currentLog?.response_time }}ms</el-descriptions-item
      >
      <el-descriptions-item label="IP地址">{{ currentLog?.ip_address }}</el-descriptions-item>
      <el-descriptions-item label="用户">{{ currentLog?.user_name }}</el-descriptions-item>
      <el-descriptions-item label="请求时间" :span="2">{{
        currentLog?.created_at
      }}</el-descriptions-item>
    </el-descriptions>
    <div class="log-section">
      <h4>请求体</h4>
      <pre>{{ currentLog?.request_body || '无' }}</pre>
    </div>
    <div class="log-section">
      <h4>响应体</h4>
      <pre>{{ currentLog?.response_body || '无' }}</pre>
    </div>
    <template #footer>
      <el-button @click="emit('update:visible', false)">关闭</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import type { ApiLog } from '@/api/api-gateway'

/**
 * 日志详情对话框组件
 * 仅做展示，对话框状态由父组件控制
 */
const props = defineProps<{
  visible: boolean
  currentLog: ApiLog | null
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
}>()

void props
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

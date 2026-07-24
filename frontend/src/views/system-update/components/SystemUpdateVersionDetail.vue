<!--
  SystemUpdateVersionDetail.vue - 系统版本详情对话框
  拆分自 system-update/index.vue（P14 批 2 I-3 第 1 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    title="版本详情"
    width="700px"
    aria-label="系统版本详情对话框"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-descriptions :column="2" border>
      <el-descriptions-item label="版本号">{{ currentVersionDetail?.version }}</el-descriptions-item>
      <el-descriptions-item label="发布日期">{{
        currentVersionDetail?.release_date
      }}</el-descriptions-item>
      <el-descriptions-item label="文件大小" :span="2">{{
        formatFileSize(currentVersionDetail?.file_size || 0)
      }}</el-descriptions-item>
    </el-descriptions>
    <div class="detail-section">
      <h4>更新说明</h4>
      <p>{{ currentVersionDetail?.release_notes || '暂无说明' }}</p>
    </div>
    <div class="detail-section">
      <h4>新功能</h4>
      <ul>
        <li v-for="(feature, index) in currentVersionDetail?.features || []" :key="index">
          {{ feature }}
        </li>
        <li v-if="!currentVersionDetail?.features?.length">暂无</li>
      </ul>
    </div>
    <div class="detail-section">
      <h4>问题修复</h4>
      <ul>
        <li v-for="(fix, index) in currentVersionDetail?.bug_fixes || []" :key="index">
          {{ fix }}
        </li>
        <li v-if="!currentVersionDetail?.bug_fixes?.length">暂无</li>
      </ul>
    </div>
    <div class="detail-section">
      <h4>重大变更</h4>
      <ul>
        <li v-for="(change, index) in currentVersionDetail?.breaking_changes || []" :key="index">
          {{ change }}
        </li>
        <li v-if="!currentVersionDetail?.breaking_changes?.length">暂无</li>
      </ul>
    </div>
    <template #footer>
      <el-button @click="emit('update:visible', false)">关闭</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import type { SystemVersion } from '@/api/system-update'
import { formatFileSize } from '../composables/sysUpdFmts'

/**
 * 系统版本详情对话框组件
 */
const props = defineProps<{
  visible: boolean
  currentVersionDetail: SystemVersion | null
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
}>()

void props
</script>

<style scoped>
.detail-section {
  margin-top: 16px;
}
.detail-section h4 {
  margin-bottom: 8px;
  color: #303133;
}
.detail-section ul {
  margin: 0;
  padding-left: 20px;
}
.detail-section li {
  margin-bottom: 4px;
  color: #606266;
}
</style>

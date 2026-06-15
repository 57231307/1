<!--
  quality/index.vue - 质量管理主入口（容器组件）
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-4）：
  原 800 行"上帝组件"已拆分为以下 3 个 Tab 子组件 + 4 个对话框，
  位于 views/quality/tabs/ 目录：

  | Tab         | 子组件                              |
  | ----------- | ----------------------------------- |
  | 质量标准    | tabs/StandardTab.vue                |
  | 检验记录    | tabs/RecordTab.vue                  |
  | 缺陷管理    | tabs/DefectTab.vue                  |
  | 标准编辑    | tabs/StandardFormDialogTab.vue      |
  | 审批        | tabs/ApproveDialogTab.vue           |
  | 版本历史    | tabs/VersionHistoryDialogTab.vue    |
  | 检验编辑    | tabs/RecordFormDialogTab.vue        |

  本主入口仅承担：Tab 切换 + 公共样式。
-->
<template>
  <div class="quality-page">
    <el-tabs v-model="activeTab">
      <el-tab-pane label="质量标准" name="standard">
        <StandardTab @open-approve="openApprove" @open-history="openVersionHistory" />
      </el-tab-pane>

      <el-tab-pane label="检验记录" name="record">
        <RecordTab />
      </el-tab-pane>

      <el-tab-pane label="缺陷管理" name="defect">
        <DefectTab />
      </el-tab-pane>
    </el-tabs>

    <StandardFormDialogTab
      v-model="standardDialogVisible"
      :current-row="currentStandardRow"
      @submitted="handleStandardSubmitted"
    />

    <ApproveDialogTab
      v-model="approveDialogVisible"
      :current-row="approveStandardItem"
      @submitted="handleApproveSubmitted"
    />

    <VersionHistoryDialogTab v-model="versionHistoryVisible" :history-list="versionHistoryList" />

    <RecordFormDialogTab
      v-model="recordDialogVisible"
      :current-row="currentRecordRow"
      @submitted="handleRecordSubmitted"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, provide } from 'vue'
import { ElMessage } from 'element-plus'
import { logger } from '@/utils/logger'
import {
  approveQualityStandard,
  getQualityStandardVersions,
  type QualityStandard,
  type QualityRecord,
} from '@/api/quality'
import StandardTab from './tabs/StandardTab.vue'
import RecordTab from './tabs/RecordTab.vue'
import DefectTab from './tabs/DefectTab.vue'
import StandardFormDialogTab from './tabs/StandardFormDialogTab.vue'
import ApproveDialogTab from './tabs/ApproveDialogTab.vue'
import VersionHistoryDialogTab from './tabs/VersionHistoryDialogTab.vue'
import RecordFormDialogTab from './tabs/RecordFormDialogTab.vue'

const activeTab = ref('standard')

const standardDialogVisible = ref(false)
const currentStandardRow = ref<QualityStandard | null>(null)

const approveDialogVisible = ref(false)
const approveStandardItem = ref<QualityStandard | null>(null)

const versionHistoryVisible = ref(false)
const versionHistoryList = ref<QualityStandard[]>([])

const recordDialogVisible = ref(false)
const currentRecordRow = ref<QualityRecord | null>(null)

// 跨子组件通知机制
const openStandardDialog = (row: QualityStandard | null) => {
  currentStandardRow.value = row
  standardDialogVisible.value = true
}

const openApprove = (row: QualityStandard) => {
  approveStandardItem.value = row
  approveDialogVisible.value = true
}

const openVersionHistory = async (row: QualityStandard) => {
  try {
    const res = await getQualityStandardVersions(row.id)
    versionHistoryList.value = (res.data as QualityStandard[] | undefined) || []
    versionHistoryVisible.value = true
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '获取版本历史失败')
    logger.error('获取版本历史失败', err.message)
  }
}

const openRecordDialog = (row: QualityRecord | null) => {
  currentRecordRow.value = row
  recordDialogVisible.value = true
}

const handleStandardSubmitted = () => {
  // 通知子组件刷新
}

const handleApproveSubmitted = async (row: QualityStandard) => {
  try {
    await approveQualityStandard(row.id)
    ElMessage.success('审批成功')
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '操作失败')
  }
}

const handleRecordSubmitted = () => {
  // 通知子组件刷新
}

provide('qualityActions', {
  openStandardDialog,
  openRecordDialog,
})
</script>

<style scoped>
.quality-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
:deep(.page-header) {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
:deep(.page-title) {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
</style>

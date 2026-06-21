<!--
  system-update/index.vue - 系统更新管理（拆分重构版）
  任务编号: P14 批 2 I-3 第 1 批
  拆分：725 行 → ~150 行 + 6 子组件 + 2 composable + 1 工具
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="system-update-page">
    <div class="page-header">
      <h2 class="page-title">系统更新</h2>
      <div class="header-actions">
        <el-button type="primary" @click="handleCheckUpdate">
          <el-icon><Refresh /></el-icon>
          检查更新
        </el-button>
        <el-button @click="onOpenBackupDialog">
          <el-icon><FolderAdd /></el-icon>
          创建备份
        </el-button>
      </div>
    </div>

    <SuInfoCards
      :current-version="currentVersion"
      :latest-version="latestVersion"
      :has-update="hasUpdate"
    />

    <el-tabs v-model="activeTab">
      <el-tab-pane label="版本列表" name="versions">
        <SystemUpdateVersionTab
          :versions="versions"
          :loading="versionLoading"
          :total="versionTotal"
          :query-params="versionQuery"
          :version-status-type-map="versionStatusTypeMap"
          :version-status-map="versionStatusMap"
          :format-file-size="formatFileSize"
          @download="handleDownload"
          @install="handleInstall"
          @view-detail="viewVersionDetail"
          @fetch="fetchVersions"
        />
      </el-tab-pane>

      <el-tab-pane label="更新任务" name="tasks">
        <SystemUpdateTaskTab
          :tasks="tasks"
          :loading="taskLoading"
          :total="taskTotal"
          :query-params="taskQuery"
          :task-status-type-map="taskStatusTypeMap"
          :task-status-map="taskStatusMap"
          @rollback="handleRollback"
          @cancel="handleCancelTask"
          @fetch="fetchTasks"
        />
      </el-tab-pane>

      <el-tab-pane label="系统备份" name="backups">
        <SystemUpdateBackupTab
          :backups="backups"
          :loading="backupLoading"
          :total="backupTotal"
          :query-params="backupQuery"
          :backup-type-map="backupTypeMap"
          :backup-status-type-map="backupStatusTypeMap"
          :backup-status-map="backupStatusMap"
          :format-file-size="formatFileSize"
          @download="handleDownloadBackup"
          @restore="handleRestore"
          @delete="handleDeleteBackup"
          @fetch="fetchBackups"
        />
      </el-tab-pane>
    </el-tabs>

    <SuVerDetail
      v-model:visible="versionDetailVisible"
      :current-version-detail="currentVersionDetail"
    />

    <SuBkpForm
      v-model:visible="backupDialogVisible"
      :form="backupForm"
      :submit-loading="backupSubmitLoading"
      @submit="onBackupSubmit"
    />
  </div>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import { ref, onMounted } from 'vue'
import { Refresh, FolderAdd } from '@element-plus/icons-vue'
import { useSysUpd } from './composables/useSysUpd'
import { useSysUpdProc } from './composables/useSysUpdProc'
import * as sysUpdFmts from './composables/sysUpdFmts'
import SystemUpdateVersionTab from './tabs/SystemUpdateVersionTab.vue'
import SystemUpdateTaskTab from './tabs/SystemUpdateTaskTab.vue'
import SystemUpdateBackupTab from './tabs/SystemUpdateBackupTab.vue'
import SuInfoCards from './components/SuInfoCards.vue'
import SuVerDetail from './components/SuVerDetail.vue'
import SuBkpForm from './components/SuBkpForm.vue'

const activeTab = ref('versions')

// 解构 useSysUpd 到顶层（useSysUpd 直接返回 ref/function 集合，不包装 reactive）
const {
  currentVersion,
  latestVersion,
  hasUpdate,
  fetchCurrentVersion,
  handleCheckUpdate,
  versions,
  versionTotal,
  versionLoading,
  versionQuery,
  fetchVersions,
  tasks,
  taskTotal,
  taskLoading,
  taskQuery,
  fetchTasks,
  backups,
  backupTotal,
  backupLoading,
  backupQuery,
  fetchBackups,
  backupForm,
  backupSubmitLoading,
  resetBackupForm,
  handleBackupSubmit,
  currentVersionDetail,
  viewVersionDetail,
} = useSysUpd()

// 流程性方法（下载/安装/回滚/取消/恢复/下载备份/删除）
const proc = useSysUpdProc({
  fetchVersions,
  fetchTasks,
  fetchBackups,
})
const {
  handleDownload,
  handleInstall,
  handleRollback,
  handleCancelTask,
  handleDownloadBackup,
  handleRestore,
  handleDeleteBackup,
} = proc

// 状态映射 + 文件大小格式化（来自 sysUpdFmts 工具）
const {
  VERSION_STATUS_LABEL,
  VERSION_STATUS_TYPE,
  TASK_STATUS_LABEL,
  TASK_STATUS_TYPE,
  BACKUP_TYPE_LABEL,
  BACKUP_STATUS_LABEL,
  BACKUP_STATUS_TYPE,
  formatFileSize,
} = sysUpdFmts

// 模板里用 statusTypeMap/statusMap 短名（与子组件 props 名称对齐）
const versionStatusTypeMap = VERSION_STATUS_TYPE
const versionStatusMap = VERSION_STATUS_LABEL
const taskStatusTypeMap = TASK_STATUS_TYPE
const taskStatusMap = TASK_STATUS_LABEL
const backupTypeMap = BACKUP_TYPE_LABEL
const backupStatusTypeMap = BACKUP_STATUS_TYPE
const backupStatusMap = BACKUP_STATUS_LABEL

// 对话框可见性本地 ref
const versionDetailVisible = ref(false)
const backupDialogVisible = ref(false)

/** 打开创建备份对话框 */
const onOpenBackupDialog = () => {
  resetBackupForm()
  backupDialogVisible.value = true
}

/** 提交备份表单 */
const onBackupSubmit = async () => {
  const ok = await handleBackupSubmit()
  if (ok) backupDialogVisible.value = false
}

onMounted(() => {
  fetchCurrentVersion()
  fetchVersions()
  fetchTasks()
  fetchBackups()
})
</script>

<style scoped>
.system-update-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.page-title {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
</style>

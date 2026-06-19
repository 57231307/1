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
        <el-button type="primary" @click="upd.handleCheckUpdate">
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
      :current-version="upd.currentVersion"
      :latest-version="upd.latestVersion"
      :has-update="upd.hasUpdate"
    />

    <el-tabs v-model="activeTab">
      <el-tab-pane label="版本列表" name="versions">
        <SuVerTbl
          :versions="upd.versions"
          :loading="upd.versionLoading"
          :total="upd.versionTotal"
          :query="upd.versionQuery"
          @download="updProc.handleDownload"
          @install="updProc.handleInstall"
          @view-detail="onViewVersionDetail"
          @refresh="upd.fetchVersions"
        />
      </el-tab-pane>

      <el-tab-pane label="更新任务" name="tasks">
        <SuTaskTbl
          :tasks="upd.tasks"
          :loading="upd.taskLoading"
          :total="upd.taskTotal"
          :query="upd.taskQuery"
          @rollback="updProc.handleRollback"
          @cancel="updProc.handleCancelTask"
          @refresh="upd.fetchTasks"
        />
      </el-tab-pane>

      <el-tab-pane label="系统备份" name="backups">
        <SuBkpTbl
          :backups="upd.backups"
          :loading="upd.backupLoading"
          :total="upd.backupTotal"
          :query="upd.backupQuery"
          @download-backup="updProc.handleDownloadBackup"
          @restore="updProc.handleRestore"
          @delete="updProc.handleDeleteBackup"
          @refresh="upd.fetchBackups"
        />
      </el-tab-pane>
    </el-tabs>

    <SuVerDetail
      v-model:visible="versionDetailVisible"
      :current-version-detail="upd.currentVersionDetail"
    />

    <SuBkpForm
      v-model:visible="backupDialogVisible"
      :form="upd.backupForm"
      :submit-loading="upd.backupSubmitLoading"
      @submit="onBackupSubmit"
    />
  </div>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import { ref, onMounted } from 'vue'
import { Refresh, FolderAdd } from '@element-plus/icons-vue'
import type { SystemVersion } from '@/api/system-update'
import { useSysUpd } from './composables/useSysUpd'
import { useSysUpdProc } from './composables/useSysUpdProc'
import SuInfoCards from './components/SuInfoCards.vue'
import SuVerTbl from './components/SuVerTbl.vue'
import SuTaskTbl from './components/SuTaskTbl.vue'
import SuBkpTbl from './components/SuBkpTbl.vue'
import SuVerDetail from './components/SuVerDetail.vue'
import SuBkpForm from './components/SuBkpForm.vue'

const upd = useSysUpd()
const updProc = useSysUpdProc({
  fetchVersions: upd.fetchVersions,
  fetchTasks: upd.fetchTasks,
  fetchBackups: upd.fetchBackups,
})

const activeTab = ref('versions')

// 对话框可见性本地 ref
const versionDetailVisible = ref(false)
const backupDialogVisible = ref(false)

/** 打开版本详情 */
const onViewVersionDetail = (row: SystemVersion) => {
  upd.viewVersionDetail(row)
  versionDetailVisible.value = true
}

/** 打开创建备份对话框 */
const onOpenBackupDialog = () => {
  upd.resetBackupForm()
  backupDialogVisible.value = true
}

/** 提交备份表单 */
const onBackupSubmit = async () => {
  const ok = await upd.handleBackupSubmit()
  if (ok) backupDialogVisible.value = false
}

onMounted(() => {
  upd.fetchCurrentVersion()
  upd.fetchVersions()
  upd.fetchTasks()
  upd.fetchBackups()
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

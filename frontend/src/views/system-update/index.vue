<!--
  system-update/index.vue - 系统更新管理（拆分重构版）
  任务编号: P14 批 2 I-3 第 1 批
  拆分：725 行 → ~150 行 + 6 子组件 + 2 composable + 1 工具
  批次 283：useSysUpd 返回改为 reactive 包装，父组件改为 upd.xxx 访问 + v-model:page/page-size
-->
<template>
  <div class="system-update-page">
    <div class="page-header">
      <h2 class="page-title">{{ t('systemUpdate.index.title') }}</h2>
      <div class="header-actions">
        <el-button type="primary" @click="upd.handleCheckUpdate">
          <el-icon><Refresh /></el-icon>
          {{ t('systemUpdate.index.buttonCheckUpdate') }}
        </el-button>
        <el-button @click="onOpenBackupDialog">
          <el-icon><FolderAdd /></el-icon>
          {{ t('systemUpdate.index.buttonCreateBackup') }}
        </el-button>
      </div>
    </div>

    <SystemUpdateInfoCards
      :current-version="upd.currentVersion"
      :latest-version="upd.latestVersion"
      :has-update="upd.hasUpdate"
    />

    <el-tabs v-model="activeTab">
      <el-tab-pane :label="t('systemUpdate.index.tabVersions')" name="versions">
        <SystemUpdateVersionTab
          v-model:page="upd.versionPage"
          v-model:page-size="upd.versionPageSize"
          :versions="upd.versions"
          :loading="upd.versionLoading"
          :total="upd.versionTotal"
          :version-status-type-map="versionStatusTypeMap"
          :format-file-size="formatFileSize"
          @download="proc.handleDownload"
          @install="proc.handleInstall"
          @view-detail="upd.viewVersionDetail"
        />
      </el-tab-pane>

      <el-tab-pane :label="t('systemUpdate.index.tabTasks')" name="tasks">
        <SystemUpdateTaskTab
          v-model:page="upd.taskPage"
          v-model:page-size="upd.taskPageSize"
          :tasks="upd.tasks"
          :loading="upd.taskLoading"
          :total="upd.taskTotal"
          :task-status-type-map="taskStatusTypeMap"
          @rollback="proc.handleRollback"
          @cancel="proc.handleCancelTask"
          @view-detail="upd.viewTaskDetail"
        />
      </el-tab-pane>

      <el-tab-pane :label="t('systemUpdate.index.tabBackups')" name="backups">
        <SystemUpdateBackupTab
          v-model:page="upd.backupPage"
          v-model:page-size="upd.backupPageSize"
          :backups="upd.backups"
          :loading="upd.backupLoading"
          :total="upd.backupTotal"
          :backup-status-type-map="backupStatusTypeMap"
          :format-file-size="formatFileSize"
          @download="proc.handleDownloadBackup"
          @restore="proc.handleRestore"
          @delete="proc.handleDeleteBackup"
          @view-detail="upd.viewBackupDetail"
        />
      </el-tab-pane>
    </el-tabs>

    <!-- 备份详情（getSystemBackup 回源） -->
    <el-dialog
      v-model="backupDetailVisible"
      :title="t('systemUpdate.backupTab.buttonDetail')"
      width="520"
    >
      <el-descriptions v-if="upd.currentBackupDetail" :column="2" border>
        <el-descriptions-item label="ID">{{ upd.currentBackupDetail.id }}</el-descriptions-item>
        <el-descriptions-item label="备份名称">{{
          upd.currentBackupDetail.backup_code
        }}</el-descriptions-item>
        <el-descriptions-item label="状态">{{
          upd.currentBackupDetail.status
        }}</el-descriptions-item>
        <el-descriptions-item label="大小">{{
          formatFileSize(upd.currentBackupDetail.file_size)
        }}</el-descriptions-item>
        <el-descriptions-item label="创建时间" :span="2">{{
          upd.currentBackupDetail.created_at
        }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>

    <!-- 任务详情（getUpdateTask 回源） -->
    <el-dialog
      v-model="taskDetailVisible"
      :title="t('systemUpdate.taskTab.buttonDetail')"
      width="520"
    >
      <el-descriptions v-if="upd.currentTaskDetail" :column="2" border>
        <el-descriptions-item label="ID">{{ upd.currentTaskDetail.id }}</el-descriptions-item>
        <el-descriptions-item label="任务类型">{{
          upd.currentTaskDetail.task_type
        }}</el-descriptions-item>
        <el-descriptions-item label="状态">{{ upd.currentTaskDetail.status }}</el-descriptions-item>
        <el-descriptions-item label="进度 %">{{
          upd.currentTaskDetail.progress ?? '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="创建时间" :span="2">{{
          upd.currentTaskDetail.created_at
        }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>

    <SystemUpdateVersionDetail
      v-model:visible="versionDetailVisible"
      :current-version-detail="upd.currentVersionDetail"
    />

    <SystemUpdateBackupForm
      v-model:visible="backupDialogVisible"
      :form="upd.backupForm"
      :submit-loading="upd.backupSubmitLoading"
      @update:form="v => Object.assign(upd.backupForm, v)"
      @submit="onBackupSubmit"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { Refresh, FolderAdd } from '@element-plus/icons-vue';
import { useSysUpd } from './composables/useSysUpd';
import { useSysUpdProc } from './composables/useSysUpdProc';
import * as sysUpdFmts from './composables/sysUpdFmts';
import SystemUpdateVersionTab from './tabs/SystemUpdateVersionTab.vue';
import SystemUpdateTaskTab from './tabs/SystemUpdateTaskTab.vue';
import SystemUpdateBackupTab from './tabs/SystemUpdateBackupTab.vue';
import SystemUpdateInfoCards from './components/SystemUpdateInfoCards.vue';
import SystemUpdateVersionDetail from './components/SystemUpdateVersionDetail.vue';
import SystemUpdateBackupForm from './components/SystemUpdateBackupForm.vue';

const { t } = useI18n({ useScope: 'global' });

const activeTab = ref('versions');

// 批次 283：useSysUpd 返回 reactive 包装，改为 upd.xxx 访问
const upd = useSysUpd();

// 备份/任务详情对话框可见性（内容来自 useSysUpd 的 currentBackupDetail/currentTaskDetail）
const backupDetailVisible = ref(false);
const taskDetailVisible = ref(false);
watch(
  () => [upd.currentBackupDetail, upd.currentTaskDetail] as const,
  ([b, task]) => {
    if (b) backupDetailVisible.value = true;
    if (task) taskDetailVisible.value = true;
  }
);

// 流程性方法（下载/安装/回滚/取消/恢复/下载备份/删除）
const proc = useSysUpdProc({
  fetchVersions: upd.fetchVersions,
  fetchTasks: upd.fetchTasks,
  fetchBackups: upd.fetchBackups,
});

// 状态类型映射（el-tag type，非文本，不需 i18n）+ 文件大小格式化（来自 sysUpdFmts 工具）
const { VERSION_STATUS_TYPE, TASK_STATUS_TYPE, BACKUP_STATUS_TYPE, formatFileSize } = sysUpdFmts;

// 模板里用 statusTypeMap 短名（与子组件 props 名称对齐）
const versionStatusTypeMap = VERSION_STATUS_TYPE;
const taskStatusTypeMap = TASK_STATUS_TYPE;
const backupStatusTypeMap = BACKUP_STATUS_TYPE;

// 对话框可见性本地 ref
const versionDetailVisible = ref(false);
const backupDialogVisible = ref(false);

/** 打开创建备份对话框 */
const onOpenBackupDialog = () => {
  upd.resetBackupForm();
  backupDialogVisible.value = true;
};

/** 提交备份表单 */
const onBackupSubmit = async () => {
  const ok = await upd.handleBackupSubmit();
  if (ok) backupDialogVisible.value = false;
};

// 批次 283：移除 3 个 fetch（useTableApi setup 自动加载），保留 fetchCurrentVersion
onMounted(() => {
  upd.fetchCurrentVersion();
});
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

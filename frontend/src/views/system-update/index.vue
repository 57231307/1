<template>
  <div class="system-update-page">
    <div class="page-header">
      <h2 class="page-title">系统更新</h2>
      <div class="header-actions">
        <el-button type="primary" @click="handleCheckUpdate">
          <el-icon><Refresh /></el-icon>
          检查更新
        </el-button>
        <el-button @click="openBackupDialog()">
          <el-icon><FolderAdd /></el-icon>
          创建备份
        </el-button>
      </div>
    </div>

    <el-row :gutter="20" class="info-cards">
      <el-col :span="8">
        <el-card shadow="hover">
          <template #header>
            <span>当前版本</span>
          </template>
          <div class="info-content">
            <div class="version">{{ currentVersion?.version || '-' }}</div>
            <div class="date">构建日期: {{ currentVersion?.build_date || '-' }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card shadow="hover">
          <template #header>
            <span>最新版本</span>
          </template>
          <div class="info-content">
            <div class="version">{{ latestVersion?.version || '-' }}</div>
            <div class="date">发布日期: {{ latestVersion?.release_date || '-' }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card shadow="hover">
          <template #header>
            <span>更新状态</span>
          </template>
          <div class="info-content">
            <el-tag :type="hasUpdate ? 'warning' : 'success'" size="large">
              {{ hasUpdate ? '有可用更新' : '已是最新版本' }}
            </el-tag>
          </div>
        </el-card>
      </el-col>
    </el-row>

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

    <el-dialog v-model="versionDetailVisible" title="版本详情" width="700px">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="版本号">{{
          currentVersionDetail?.version
        }}</el-descriptions-item>
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
        <el-button @click="versionDetailVisible = false">关闭</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="backupDialogVisible" title="创建备份" width="500px">
      <el-form ref="backupFormRef" :model="backupForm" :rules="backupRules" label-width="100px">
        <el-form-item label="备份类型" prop="backup_type">
          <el-select v-model="backupForm.backup_type" style="width: 100%">
            <el-option label="完整备份" value="full" />
            <el-option label="增量备份" value="incremental" />
            <el-option label="数据库备份" value="database" />
            <el-option label="文件备份" value="files" />
          </el-select>
        </el-form-item>
        <el-form-item label="描述" prop="description">
          <el-input
            v-model="backupForm.description"
            type="textarea"
            :rows="3"
            placeholder="请输入备份描述"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="backupDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="backupSubmitLoading" @click="handleBackupSubmit"
          >开始备份</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Refresh, FolderAdd } from '@element-plus/icons-vue'
import {
  getCurrentVersion,
  checkForUpdates,
  listSystemVersions,
  downloadUpdate,
  installUpdate,
  listUpdateTasks,
  cancelUpdateTask,
  rollbackUpdate,
  listSystemBackups,
  createSystemBackup,
  deleteSystemBackup,
  restoreFromBackup,
  downloadBackup,
  type SystemVersion,
  type UpdateTask,
  type SystemBackup,
} from '@/api/system-update'
import { logger } from '@/utils/logger'
import SystemUpdateVersionTab from './tabs/SystemUpdateVersionTab.vue'
import SystemUpdateTaskTab from './tabs/SystemUpdateTaskTab.vue'
import SystemUpdateBackupTab from './tabs/SystemUpdateBackupTab.vue'

const activeTab = ref('versions')

// 当前版本
const currentVersion = ref<{ version: string; build_date: string } | null>(null)
const latestVersion = ref<SystemVersion | null>(null)
const hasUpdate = computed(() => {
  if (!currentVersion.value || !latestVersion.value) return false
  return currentVersion.value.version !== latestVersion.value.version
})

const fetchCurrentVersion = async () => {
  try {
    const res = await getCurrentVersion()
    currentVersion.value = res.data
  } catch (error: any) {
    logger.error('获取当前版本失败:', error)
  }
}

const handleCheckUpdate = async () => {
  try {
    const res = await checkForUpdates()
    latestVersion.value = res.data
    if (hasUpdate.value) {
      ElMessage.success(`发现新版本: ${res.data.version}`)
    } else {
      ElMessage.info('当前已是最新版本')
    }
  } catch (error: any) {
    ElMessage.error(error.message || '检查更新失败')
  }
}

// 版本列表
const versions = ref<SystemVersion[]>([])
const versionTotal = ref(0)
const versionLoading = ref(false)
const versionQuery = reactive({
  page: 1,
  page_size: 20,
})

const versionStatusMap: Record<string, string> = {
  available: '可下载',
  downloading: '下载中',
  downloaded: '已下载',
  installing: '安装中',
  installed: '已安装',
  failed: '失败',
}

const versionStatusTypeMap: Record<string, string> = {
  available: 'info',
  downloading: 'warning',
  downloaded: 'success',
  installing: 'warning',
  installed: 'success',
  failed: 'danger',
}

const fetchVersions = async () => {
  versionLoading.value = true
  try {
    const res = await listSystemVersions(versionQuery)
    versions.value = res.data || []
    versionTotal.value = res.total || 0
  } catch (error: any) {
    ElMessage.error(error.message || '获取版本列表失败')
  } finally {
    versionLoading.value = false
  }
}

const handleDownload = async (row: SystemVersion) => {
  try {
    await ElMessageBox.confirm(`确定要下载版本 ${row.version} 吗？`, '确认下载', {
      type: 'warning',
    })
    await downloadUpdate(row.id)
    ElMessage.success('下载任务已创建')
    fetchVersions()
    fetchTasks()
  } catch (error: any) {
    if (error !== 'cancel') ElMessage.error(error.message || '下载失败')
  }
}

const handleInstall = async (row: SystemVersion) => {
  try {
    await ElMessageBox.confirm(
      `确定要安装版本 ${row.version} 吗？安装过程中系统可能会重启。`,
      '确认安装',
      { type: 'warning' }
    )
    await installUpdate(row.id)
    ElMessage.success('安装任务已创建')
    fetchVersions()
    fetchTasks()
  } catch (error: any) {
    if (error !== 'cancel') ElMessage.error(error.message || '安装失败')
  }
}

const versionDetailVisible = ref(false)
const currentVersionDetail = ref<SystemVersion | null>(null)

const viewVersionDetail = (row: SystemVersion) => {
  currentVersionDetail.value = row
  versionDetailVisible.value = true
}

// 更新任务
const tasks = ref<UpdateTask[]>([])
const taskTotal = ref(0)
const taskLoading = ref(false)
const taskQuery = reactive({
  page: 1,
  page_size: 20,
})

const taskStatusMap: Record<string, string> = {
  pending: '待处理',
  downloading: '下载中',
  downloaded: '已下载',
  installing: '安装中',
  completed: '已完成',
  failed: '失败',
  rolled_back: '已回滚',
}

const taskStatusTypeMap: Record<string, string> = {
  pending: 'info',
  downloading: 'warning',
  downloaded: 'success',
  installing: 'warning',
  completed: 'success',
  failed: 'danger',
  rolled_back: 'info',
}

const fetchTasks = async () => {
  taskLoading.value = true
  try {
    const res = await listUpdateTasks(taskQuery)
    tasks.value = res.data || []
    taskTotal.value = res.total || 0
  } catch (error: any) {
    ElMessage.error(error.message || '获取任务列表失败')
  } finally {
    taskLoading.value = false
  }
}

const handleCancelTask = async (row: UpdateTask) => {
  try {
    await ElMessageBox.confirm('确定要取消此任务吗？', '确认取消', { type: 'warning' })
    await cancelUpdateTask(row.id)
    ElMessage.success('任务已取消')
    fetchTasks()
  } catch (error: any) {
    if (error !== 'cancel') ElMessage.error(error.message || '取消失败')
  }
}

const handleRollback = async (row: UpdateTask) => {
  try {
    await ElMessageBox.confirm(`确定要回滚到版本 ${row.from_version} 吗？`, '确认回滚', {
      type: 'warning',
    })
    await rollbackUpdate(row.id)
    ElMessage.success('回滚任务已创建')
    fetchTasks()
  } catch (error: any) {
    if (error !== 'cancel') ElMessage.error(error.message || '回滚失败')
  }
}

// 系统备份
const backups = ref<SystemBackup[]>([])
const backupTotal = ref(0)
const backupLoading = ref(false)
const backupQuery = reactive({
  page: 1,
  page_size: 20,
})

const backupTypeMap: Record<string, string> = {
  full: '完整备份',
  incremental: '增量备份',
  database: '数据库备份',
  files: '文件备份',
}

const backupStatusMap: Record<string, string> = {
  creating: '创建中',
  completed: '已完成',
  failed: '失败',
}

const backupStatusTypeMap: Record<string, string> = {
  creating: 'warning',
  completed: 'success',
  failed: 'danger',
}

const fetchBackups = async () => {
  backupLoading.value = true
  try {
    const res = await listSystemBackups(backupQuery)
    backups.value = res.data || []
    backupTotal.value = res.total || 0
  } catch (error: any) {
    ElMessage.error(error.message || '获取备份列表失败')
  } finally {
    backupLoading.value = false
  }
}

const backupDialogVisible = ref(false)
const backupFormRef = ref<FormInstance>()
const backupSubmitLoading = ref(false)
const backupForm = reactive({
  backup_type: 'full' as 'full' | 'incremental' | 'database' | 'files',
  description: '',
})

const backupRules: FormRules = {
  backup_type: [{ required: true, message: '请选择备份类型', trigger: 'change' }],
}

const openBackupDialog = () => {
  backupForm.backup_type = 'full'
  backupForm.description = ''
  backupDialogVisible.value = true
}

const handleBackupSubmit = async () => {
  if (!backupFormRef.value) return
  await backupFormRef.value.validate(async valid => {
    if (!valid) return

    backupSubmitLoading.value = true
    try {
      await createSystemBackup(backupForm)
      ElMessage.success('备份任务已创建')
      backupDialogVisible.value = false
      fetchBackups()
    } catch (error: any) {
      ElMessage.error(error.message || '创建备份失败')
    } finally {
      backupSubmitLoading.value = false
    }
  })
}

const handleDeleteBackup = async (row: SystemBackup) => {
  try {
    await ElMessageBox.confirm('确定要删除此备份吗？', '确认删除', { type: 'warning' })
    await deleteSystemBackup(row.id)
    ElMessage.success('删除成功')
    fetchBackups()
  } catch (error: any) {
    if (error !== 'cancel') ElMessage.error(error.message || '删除失败')
  }
}

const handleRestore = async (row: SystemBackup) => {
  try {
    await ElMessageBox.confirm('确定要从此备份恢复系统吗？此操作不可撤销。', '确认恢复', {
      type: 'warning',
    })
    await restoreFromBackup(row.id)
    ElMessage.success('恢复任务已创建')
    fetchTasks()
  } catch (error: any) {
    if (error !== 'cancel') ElMessage.error(error.message || '恢复失败')
  }
}

const handleDownloadBackup = async (row: SystemBackup) => {
  try {
    const blob = await downloadBackup(row.id)
    const link = document.createElement('a')
    link.href = URL.createObjectURL(blob)
    link.download = `backup_${row.backup_code}.zip`
    link.click()
    ElMessage.success('备份下载成功')
  } catch (error: any) {
    ElMessage.error(error.message || '下载失败')
  }
}

const formatFileSize = (bytes: number) => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
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
.info-cards {
  margin-bottom: 20px;
}
.info-content {
  text-align: center;
  padding: 10px 0;
}
.version {
  font-size: 24px;
  font-weight: 600;
  color: #303133;
  margin-bottom: 8px;
}
.date {
  font-size: 14px;
  color: #909399;
}
.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
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

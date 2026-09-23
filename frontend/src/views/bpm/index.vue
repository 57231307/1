<!--
  bpm/index.vue - 审批管理
  D05 Batch 2：接入 useI18n，所有硬编码中文迁移到 locales/zh-CN.ts + en-US.ts
-->
<template>
  <div class="bpm-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">{{ $t('bpm.title') }}</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">{{
            $t('bpm.breadcrumb.home')
          }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ $t('bpm.breadcrumb.bpm') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ $t('bpm.breadcrumb.myApproval') }}</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
    </div>

    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card warning">
          <div class="stat-content">
            <div class="stat-icon pending-icon">
              <el-icon><Clock /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">{{ $t('bpm.stats.pendingTasks') }}</div>
              <div class="stat-value">{{ stats.pendingTasks }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon completed-icon">
              <el-icon><CircleCheck /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">{{ $t('bpm.stats.completedTasks') }}</div>
              <div class="stat-value">{{ stats.completedTasks }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card highlight">
          <div class="stat-content">
            <div class="stat-icon urgent-icon">
              <el-icon><Warning /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">{{ $t('bpm.stats.urgentTasks') }}</div>
              <div class="stat-value">{{ stats.urgentTasks }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon avg-icon">
              <el-icon><Timer /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">{{ $t('bpm.stats.avgProcessingTime') }}</div>
              <div class="stat-value">{{ avgProcessingTimeText }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-tabs v-model="activeTab" @tab-change="handleTabChange">
      <el-tab-pane :label="$t('bpm.tab.pending')" name="pending">
        <el-card shadow="hover" class="table-card">
          <el-table :data="pendingTasks" stripe :aria-label="$t('bpm.pendingTable.ariaLabel')">
            <el-table-column
              prop="task_no"
              :label="$t('bpm.pendingTable.taskNo')"
              min-width="150"
              fixed
            />
            <el-table-column
              prop="node_name"
              :label="$t('bpm.pendingTable.taskName')"
              min-width="150"
            />
            <el-table-column
              prop="actual_handler_name"
              :label="$t('bpm.pendingTable.applicant')"
              width="120"
            />
            <el-table-column
              prop="created_at"
              :label="$t('bpm.pendingTable.applyTime')"
              width="160"
            />
            <el-table-column prop="due_date" :label="$t('bpm.pendingTable.dueDate')" width="160">
              <template #default="{ row }">
                <span v-if="row.due_date" :class="{ overdue: isOverdue(row.due_date) }">
                  {{ row.due_date }}
                </span>
                <span v-else>-</span>
              </template>
            </el-table-column>
            <el-table-column prop="priority" :label="$t('bpm.pendingTable.priority')" width="100">
              <template #default="{ row }">
                <el-tag :type="getPriorityType(row.priority)" size="small">
                  {{ getPriorityText(row.priority) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column :label="$t('bpm.pendingTable.operation')" width="180" fixed="right">
              <template #default="{ row }">
                <el-button
                  v-permission="'bpm_task:approve'"
                  type="primary"
                  link
                  size="small"
                  @click="handleApprove(row as BPMTask)"
                  >{{ $t('bpm.pendingTable.approve') }}</el-button
                >
                <el-button type="warning" link size="small" @click="handleDetail(row as BPMTask)">{{
                  $t('bpm.pendingTable.detail')
                }}</el-button>
                <el-button
                  v-permission="'bpm_task:transfer'"
                  type="info"
                  link
                  size="small"
                  @click="handleTransfer(row as BPMTask)"
                  >{{ $t('bpm.pendingTable.transfer') }}</el-button
                >
                <el-button
                  v-permission="'bpm_task:urge'"
                  type="danger"
                  link
                  size="small"
                  @click="handleUrge(row as BPMTask)"
                  >{{ $t('bpm.pendingTable.urge') }}</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane :label="$t('bpm.tab.processed')" name="processed">
        <el-card shadow="hover" class="table-card">
          <el-table :data="processedTasks" stripe :aria-label="$t('bpm.processedTable.ariaLabel')">
            <el-table-column prop="task_no" :label="$t('bpm.processedTable.taskNo')" fixed />
            <el-table-column
              prop="node_name"
              :label="$t('bpm.processedTable.taskName')"
              min-width="150"
            />
            <el-table-column
              prop="actual_handler_name"
              :label="$t('bpm.processedTable.handler')"
              width="120"
            />
            <el-table-column prop="handled_at" :label="$t('bpm.processedTable.completedAt')" />
            <el-table-column :label="$t('bpm.processedTable.status')" width="100">
              <template #default="{ row }">
                <el-tag :type="getTaskStatusType(row.status)" size="small">
                  {{ getTaskStatusText(row.status) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column
              prop="approval_opinion"
              :label="$t('bpm.processedTable.comment')"
              min-width="150"
              show-overflow-tooltip
            />
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane :label="$t('bpm.tab.monitor')" name="monitor">
        <div class="monitor-toolbar">
          <el-button type="primary" @click="openStartProcessDialog">{{
            $t('bpm.monitorTable.startProcess')
          }}</el-button>
          <el-button @click="openBusinessRelationDialog">{{
            $t('bpm.monitorTable.businessRelation')
          }}</el-button>
        </div>
        <el-card shadow="hover" class="table-card">
          <template #header>{{ $t('bpm.monitorTable.instancesTitle') }}</template>
          <el-table :data="processInstances" stripe :aria-label="$t('bpm.monitorTable.ariaLabel')">
            <el-table-column
              prop="instance_no"
              :label="$t('bpm.monitorTable.instanceId')"
              min-width="160"
              fixed
            />
            <el-table-column prop="title" :label="$t('bpm.monitorTable.title')" min-width="150" />
            <el-table-column
              prop="business_type"
              :label="$t('bpm.monitorTable.businessType')"
              width="140"
            />
            <el-table-column
              prop="initiator_name"
              :label="$t('bpm.monitorTable.startUser')"
              width="120"
            />
            <el-table-column prop="started_at" :label="$t('bpm.monitorTable.startTime')" />
            <el-table-column
              prop="current_node_name"
              :label="$t('bpm.monitorTable.currentNode')"
              width="140"
            />
            <el-table-column prop="status" :label="$t('bpm.monitorTable.status')" width="120">
              <template #default="{ row }">
                <el-tag :type="getProcessStatusType(row.status)" size="small">
                  {{ getProcessStatusText(row.status) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column :label="$t('bpm.monitorTable.operation')" width="240" fixed="right">
              <template #default="{ row }">
                <el-button
                  type="primary"
                  link
                  size="small"
                  @click="handleViewProcess(row as BPMInstance)"
                  >{{ $t('bpm.monitorTable.view') }}</el-button
                >
                <el-button
                  type="info"
                  link
                  size="small"
                  @click="handleProcessImage(row as BPMInstance)"
                  >{{ $t('bpm.monitorTable.processImage') }}</el-button
                >
                <el-button
                  type="primary"
                  link
                  size="small"
                  @click="handleTrace(row as BPMInstance)"
                  >{{ $t('bpm.initiatedTable.trace') }}</el-button
                >
                <el-button
                  v-permission="'bpm_process:cancel'"
                  type="info"
                  link
                  size="small"
                  @click="handleCancel(row as BPMInstance)"
                  >{{ $t('bpm.initiatedTable.cancel') }}</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>
    </el-tabs>

    <el-dialog
      v-model="startProcessDialog.visible"
      :title="$t('bpm.startProcessDialog.title')"
      width="500px"
      destroy-on-close
      :aria-label="$t('bpm.startProcessDialog.ariaLabel')"
    >
      <el-form
        :model="startProcessDialog"
        label-width="100px"
        :aria-label="$t('bpm.startProcessDialog.formAriaLabel')"
      >
        <el-form-item :label="$t('bpm.startProcessDialog.processKey')">
          <el-input
            v-model="startProcessDialog.processKey"
            :placeholder="$t('bpm.startProcessDialog.processKeyPlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="$t('bpm.startProcessDialog.businessType')">
          <el-input
            v-model="startProcessDialog.businessType"
            :placeholder="$t('bpm.startProcessDialog.businessTypePlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="$t('bpm.startProcessDialog.businessId')">
          <el-input-number
            v-model="startProcessDialog.businessId"
            :placeholder="$t('bpm.startProcessDialog.businessIdPlaceholder')"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="$t('bpm.startProcessDialog.processTitle')">
          <el-input
            v-model="startProcessDialog.processTitle"
            :placeholder="$t('bpm.startProcessDialog.processTitlePlaceholder')"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="startProcessDialog.visible = false">{{
          $t('bpm.startProcessDialog.cancel')
        }}</el-button>
        <el-button
          type="primary"
          :loading="startProcessDialog.loading"
          @click="handleStartProcess"
          >{{ $t('bpm.startProcessDialog.confirm') }}</el-button
        >
      </template>
    </el-dialog>

    <el-dialog
      v-model="businessRelation.visible"
      :title="$t('bpm.businessRelationDialog.title')"
      width="520px"
      destroy-on-close
      :aria-label="$t('bpm.businessRelationDialog.ariaLabel')"
    >
      <el-form label-width="100px" :aria-label="$t('bpm.businessRelationDialog.formAriaLabel')">
        <el-form-item :label="$t('bpm.businessRelationDialog.businessType')">
          <el-input
            v-model="businessRelation.businessType"
            :placeholder="$t('bpm.businessRelationDialog.businessTypePlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="$t('bpm.businessRelationDialog.businessId')">
          <el-input-number
            v-model="businessRelation.businessId"
            :placeholder="$t('bpm.businessRelationDialog.businessIdPlaceholder')"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQueryBusinessRelation">{{
            $t('bpm.businessRelationDialog.query')
          }}</el-button>
        </el-form-item>
      </el-form>
      <div v-if="businessRelation.result" class="relation-result">
        <el-descriptions :column="1" border>
          <el-descriptions-item :label="$t('bpm.businessRelationDialog.hasProcess')">{{
            businessRelation.result.has_process
              ? $t('bpm.businessRelationDialog.yes')
              : $t('bpm.businessRelationDialog.no')
          }}</el-descriptions-item>
          <el-descriptions-item :label="$t('bpm.businessRelationDialog.instanceId')">{{
            businessRelation.result.instance_no
          }}</el-descriptions-item>
          <el-descriptions-item :label="$t('bpm.businessRelationDialog.status')">{{
            getProcessStatusText(businessRelation.result.process_status)
          }}</el-descriptions-item>
          <el-descriptions-item :label="$t('bpm.businessRelationDialog.startTime')">{{
            businessRelation.result.started_at
          }}</el-descriptions-item>
          <el-descriptions-item :label="$t('bpm.businessRelationDialog.endTime')">{{
            businessRelation.result.completed_at
          }}</el-descriptions-item>
          <el-descriptions-item :label="$t('bpm.businessRelationDialog.taskProgress')">{{
            businessRelation.result.completed_tasks + ' / ' + businessRelation.result.task_count
          }}</el-descriptions-item>
        </el-descriptions>
      </div>
    </el-dialog>

    <el-dialog
      v-model="transfer.visible"
      :title="$t('bpm.message.transferTitle')"
      width="500px"
      destroy-on-close
      :aria-label="$t('bpm.message.transferTitle')"
    >
      <el-form label-width="100px" :aria-label="$t('bpm.message.transferTitle')">
        <el-form-item :label="$t('bpm.approval.transferDialog.targetAssignee')">
          <el-select
            v-model="transfer.assigneeId"
            filterable
            :placeholder="$t('bpm.approval.transferDialog.assigneePlaceholder')"
            style="width: 100%"
          >
            <el-option
              v-for="u in transfer.candidates"
              :key="u.id"
              :label="`${u.real_name}（${u.username}）`"
              :value="u.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('bpm.message.transferComment')">
          <el-input
            v-model="transfer.reason"
            type="textarea"
            :rows="3"
            :placeholder="$t('bpm.approval.transferDialog.commentPlaceholder')"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="transfer.visible = false">{{ $t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="transfer.loading" @click="confirmTransfer">
          {{ $t('common.confirm') }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, reactive, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Clock, CircleCheck, Warning, Timer } from '@element-plus/icons-vue';
// D14 Batch 5b：原 bpmApi 对象已转风格 B 函数
import {
  getBpmTaskList,
  getBpmInstanceListForMonitor,
  approveBpmTask,
  getBpmInstanceById,
  transferBpmTask,
  urgeBpmTask,
  getBpmApprovalChain,
  getBpmProcessVisualization,
  cancelBpmInstance,
  startBpmProcess,
  getBpmBusinessRelation,
  getBpmMonitorStats,
} from '@/api/bpm';
import type {
  BPMTask,
  BPMInstance,
  BpmBusinessRelation,
  BpmInstanceDetail,
  BPMTaskStatus,
} from '@/api/bpm';
import { useUserStore } from '@/store/user';
import { getUserList, type User } from '@/api/user';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });
const userStore = useUserStore();

// v11 批次 162 P2-1 修复：el-tag type 联合字面量类型，替代 Record<string, any>
type TagType = 'success' | 'warning' | 'info' | 'primary' | 'danger';

const activeTab = ref('pending');

const stats = ref({
  pendingTasks: 0,
  completedTasks: 0,
  urgentTasks: 0,
  /** 后端给分钟数，无已完成样本时为 null（不用 0 冒充"处理很快"） */
  avgProcessingTime: null as number | null,
});

const avgProcessingTimeText = computed(() =>
  stats.value.avgProcessingTime === null
    ? t('bpm.stats.noSample')
    : `${Math.round(stats.value.avgProcessingTime)}${t('bpm.stats.minuteUnit')}`
);

// v11 批次 162 P2-1 修复：any[] 改为具体类型 BPMTask[]/BPMInstance[]
const pendingTasks = ref<BPMTask[]>([]);
const processedTasks = ref<BPMTask[]>([]);
const processInstances = ref<BPMInstance[]>([]);
// 业务关系查询结果（后端扁平 BpmBusinessRelation）
const businessRelation = reactive<{
  visible: boolean;
  businessType: string;
  businessId: number | null;
  result: BpmBusinessRelation | null;
}>({
  visible: false,
  businessType: '',
  businessId: null,
  result: null,
});
// 启动流程对话框
// 字段以后端 StartProcessRequest 为准（models/dto/bpm_dto.rs:53-64）：
// process_key / business_type / business_id / title 必填，
// initiator_id / initiator_name 取登录用户，无 business_key 字段。
const startProcessDialog = reactive<{
  visible: boolean;
  loading: boolean;
  processKey: string;
  businessType: string;
  businessId: number | null;
  processTitle: string;
}>({
  visible: false,
  loading: false,
  processKey: '',
  businessType: '',
  businessId: null,
  processTitle: '',
});

const getPriorityType = (priority: string): TagType => {
  const map: Record<string, TagType> = { high: 'danger', medium: 'warning', low: 'info' };
  return map[priority] || 'info';
};

const getPriorityText = (priority: string) => {
  const map: Record<string, string> = {
    high: t('bpm.priority.high'),
    medium: t('bpm.priority.medium'),
    low: t('bpm.priority.low'),
  };
  return map[priority] || priority;
};

// 流程实例状态词表为大写（models/status/bpm_crm_contract.rs bpm_instance）
const getProcessStatusType = (status: string): TagType => {
  const map: Record<string, TagType> = {
    PROCESSING: 'primary',
    COMPLETED: 'success',
    TERMINATED: 'danger',
    CANCELLED: 'info',
  };
  return map[status] || 'info';
};

const getProcessStatusText = (status: string) => {
  const map: Record<string, string> = {
    PROCESSING: t('bpm.processStatus.processing'),
    COMPLETED: t('bpm.processStatus.completed'),
    TERMINATED: t('bpm.processStatus.terminated'),
    CANCELLED: t('bpm.processStatus.cancelled'),
  };
  return map[status] || status;
};

// 任务状态词表为小写（models/status/bpm_crm_contract.rs bpm_task）
const getTaskStatusType = (status: string | null | undefined): TagType => {
  const map: Record<BPMTaskStatus, TagType> = {
    pending: 'warning',
    completed: 'success',
    rejected: 'danger',
    cancelled: 'info',
  };
  return status ? map[status as BPMTaskStatus] || 'info' : 'info';
};

const getTaskStatusText = (status: string | null | undefined) => {
  const map: Record<BPMTaskStatus, string> = {
    pending: t('bpm.taskStatus.pending'),
    completed: t('bpm.taskStatus.completed'),
    rejected: t('bpm.taskStatus.rejected'),
    cancelled: t('bpm.taskStatus.cancelled'),
  };
  return status ? map[status as BPMTaskStatus] || status : status || '';
};

const isOverdue = (dueDate: string) => {
  return new Date(dueDate) < new Date();
};

const handleTabChange = (tabName: string) => {
  if (tabName === 'pending') {
    fetchPendingTasks();
  } else if (tabName === 'processed') {
    fetchProcessedTasks();
  } else if (tabName === 'monitor') {
    fetchProcessInstances();
    fetchMonitorStats();
  }
};

const fetchPendingTasks = async () => {
  try {
    const res = await getBpmTaskList({ status: 'pending' });
    pendingTasks.value = res.data?.data || [];
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('bpm.message.fetchPendingFailed')
    );
    pendingTasks.value = [];
  }
};

const fetchProcessedTasks = async () => {
  try {
    const res = await getBpmTaskList({ status: 'completed' });
    processedTasks.value = res.data?.data || [];
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('bpm.message.fetchProcessedFailed')
    );
    processedTasks.value = [];
  }
};

const fetchProcessInstances = async () => {
  try {
    const res = await getBpmInstanceListForMonitor();
    processInstances.value = res.data?.data || [];
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('bpm.message.fetchInstancesFailed')
    );
    processInstances.value = [];
  }
};

// 监控统计：接入 getBpmMonitorStats（/bpm/monitor/stats）填充首页 4 个统计卡片
const fetchMonitorStats = async () => {
  try {
    const res = await getBpmMonitorStats();
    const d = res.data;
    if (d) {
      stats.value.pendingTasks = d.pending_tasks;
      stats.value.completedTasks = d.completed_tasks;
      stats.value.urgentTasks = d.overdue_tasks;
      stats.value.avgProcessingTime = d.avg_process_duration_minutes;
    }
  } catch (error: unknown) {
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('bpm.message.fetchMonitorStatsFailed')
    );
  }
};

// 启动流程：接入 startBpmProcess（/bpm/process/start）
const handleStartProcess = async () => {
  if (!startProcessDialog.processKey) {
    ElMessage.warning(t('bpm.message.processKeyRequired'));
    return;
  }
  if (!startProcessDialog.businessType) {
    ElMessage.warning(t('bpm.message.businessTypeRequired'));
    return;
  }
  if (startProcessDialog.businessId == null) {
    ElMessage.warning(t('bpm.message.businessIdRequired'));
    return;
  }
  if (!startProcessDialog.processTitle) {
    ElMessage.warning(t('bpm.message.processTitleRequired'));
    return;
  }
  // 发起人身份取登录态：后端 StartProcessRequest 必填 initiator_id / initiator_name
  //（models/dto/bpm_dto.rs:58-59），不伪造、不硬编码
  const user = userStore.userInfo;
  if (!user) {
    ElMessage.error(t('bpm.message.currentUserMissing'));
    return;
  }
  startProcessDialog.loading = true;
  try {
    const res = await startBpmProcess({
      process_key: startProcessDialog.processKey,
      business_type: startProcessDialog.businessType,
      business_id: startProcessDialog.businessId,
      title: startProcessDialog.processTitle,
      initiator_id: user.id,
      initiator_name: user.real_name || user.username,
      initiator_department_id: user.department_id,
    });
    ElMessage.success(
      t('bpm.message.startProcessSuccess', { instanceId: res.data?.instance_id || '' })
    );
    startProcessDialog.visible = false;
    startProcessDialog.processKey = '';
    startProcessDialog.businessType = '';
    startProcessDialog.businessId = null;
    startProcessDialog.processTitle = '';
    fetchMonitorStats();
    fetchProcessInstances();
  } catch (error: unknown) {
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('bpm.message.startProcessFailed')
    );
  } finally {
    startProcessDialog.loading = false;
  }
};

// 业务关系查询：接入 getBpmBusinessRelation（/bpm/business-relation）
const handleQueryBusinessRelation = async () => {
  if (!businessRelation.businessType || businessRelation.businessId == null) {
    ElMessage.warning(t('bpm.message.businessRelationInputRequired'));
    return;
  }
  try {
    const res = await getBpmBusinessRelation(
      businessRelation.businessType,
      businessRelation.businessId
    );
    // 后端恒返回扁平 BpmBusinessRelation：未关联时 has_process=false / instance_id=0，
    // 而非 null（services/bpm_ops/instance.rs:283-295）。整包存入 result。
    businessRelation.result = res.data ?? null;
    if (businessRelation.result && !businessRelation.result.has_process) {
      ElMessage.info(t('bpm.message.businessRelationNotFound'));
    }
  } catch (error: unknown) {
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('bpm.message.fetchBusinessRelationFailed')
    );
    businessRelation.result = null;
  }
};

const openStartProcessDialog = () => {
  startProcessDialog.visible = true;
};

const openBusinessRelationDialog = () => {
  businessRelation.visible = true;
};

const handleApprove = async (row: BPMTask) => {
  // 审批人身份取登录态：后端 ApproveTaskRequest 必填 handler_id / handler_name
  //（models/dto/bpm_dto.rs:76-77）
  const user = userStore.userInfo;
  if (!user) {
    ElMessage.error(t('bpm.message.currentUserMissing'));
    return;
  }
  try {
    await ElMessageBox.confirm(
      t('bpm.message.approveConfirm'),
      t('bpm.message.approveConfirmTitle'),
      { type: 'info' }
    );
    // 任务主键为 id（models/bpm_task.rs:13）；审批意见字段为 approval_opinion；
    // action 取值域 approve | reject（services/bpm_ops/task.rs:45-49）
    await approveBpmTask({
      task_id: row.id,
      handler_id: user.id,
      handler_name: user.real_name || user.username,
      action: 'approve',
      approval_opinion: t('bpm.message.approveComment'),
    });
    ElMessage.success(t('bpm.message.approveSuccess'));
    fetchPendingTasks();
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e));
  }
};

// 后端详情为嵌套 ProcessInstanceDetail（services/bpm_service_dto.rs:52-57）：
// instance 为 bpm_process_instance::Model，流程名称取顶层 definition_name。
const buildDetailLines = (d: BpmInstanceDetail): string[] => [
  `${t('bpm.detail.instanceId')}：${d.instance.instance_no}`,
  `${t('bpm.detail.processName')}：${d.definition_name}`,
  `${t('bpm.detail.startUser')}：${d.instance.initiator_name}`,
  `${t('bpm.detail.startTime')}：${d.instance.started_at ?? ''}`,
  `${t('bpm.detail.endTime')}：${d.instance.completed_at ?? ''}`,
  `${t('bpm.detail.currentStatus')}：${getProcessStatusText(d.instance.status ?? '')}`,
  `${t('bpm.detail.currentNode')}：${d.instance.current_node_name ?? ''}`,
];

// 批次 157a P1-1 修复：接入 getInstanceDetail API 展示任务关联的流程实例详情
const handleDetail = async (row: BPMTask) => {
  try {
    // 任务指向的实例主键为外键 instance_id（models/bpm_task.rs:17）
    const instanceId = row.instance_id;
    if (!instanceId) {
      ElMessage.warning(t('bpm.message.instanceIdNotFound'));
      return;
    }
    const res = await getBpmInstanceById(String(instanceId));
    const d = res.data;
    if (!d) {
      ElMessage.warning(t('bpm.message.instanceDetailNotFound'));
      return;
    }
    const lines = buildDetailLines(d);
    await ElMessageBox.alert(lines.join('\n'), t('bpm.detail.taskDetailTitle'), {
      confirmButtonText: t('bpm.message.close'),
    });
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e));
    const err = e as Error;
    ElMessage.error(err.message || t('bpm.message.fetchDetailFailed'));
  }
};

const transfer = reactive<{
  visible: boolean;
  taskId: number;
  assigneeId?: number;
  reason: string;
  candidates: User[];
  loading: boolean;
}>({
  visible: false,
  taskId: 0,
  assigneeId: undefined,
  reason: '',
  candidates: [],
  loading: false,
});

const handleTransfer = async (row: BPMTask) => {
  transfer.taskId = row.id;
  transfer.assigneeId = undefined;
  transfer.reason = '';
  transfer.visible = true;
  try {
    const res = await getUserList({ page: 1, page_size: 200 });
    transfer.candidates = res.data.users;
  } catch (e) {
    // 候选人取不到时不猜接收人：清空列表并显式报错，避免退化成手填内部 ID。
    transfer.candidates = [];
    logger.error(String(e));
    ElMessage.error(t('message.loadFailed'));
  }
};

/** 提交转办：接收人必选（后端 new_assignee_id: i32），转交意见按操作者所填原样提交 */
const confirmTransfer = async () => {
  if (transfer.assigneeId === undefined) {
    ElMessage.warning(t('bpm.approval.transferDialog.assigneeRequired'));
    return;
  }
  transfer.loading = true;
  try {
    // 任务主键为 id（models/bpm_task.rs:13）；三参对齐后端
    // TransferTaskRequest { new_assignee_id, transfer_reason }（handlers/bpm_handler.rs:214-217）
    await transferBpmTask(transfer.taskId, transfer.assigneeId, transfer.reason);
    ElMessage.success(t('bpm.message.transferSuccess'));
    transfer.visible = false;
    fetchPendingTasks();
  } catch (e) {
    logger.error(String(e));
  } finally {
    transfer.loading = false;
  }
};

const handleUrge = async (row: BPMTask) => {
  try {
    await ElMessageBox.confirm(t('bpm.message.urgeConfirm'), t('bpm.message.urgeConfirmTitle'), {
      type: 'warning',
    });
    // urgeBpmTask 第二参为必填催办消息（后端 UrgeTaskRequest.urge_message，
    // handlers/bpm_handler.rs:238-240）；任务主键为 id（models/bpm_task.rs:13）
    await urgeBpmTask(row.id, t('bpm.message.urgeMessage'));
    ElMessage.success(t('bpm.message.urgeSuccess'));
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e));
  }
};

// 批次 157a P1-1 修复：接入 getApprovalChain API 展示流程审批链追溯
// v11 批次 162 P2-1 修复：row 联合类型 BPMTask | BPMInstance，替代 any
const handleTrace = async (row: BPMTask | BPMInstance) => {
  try {
    // BPMTask 的外键字段为 instance_id（models/bpm_task.rs:17），
    // BPMInstance 的主键即 id（models/bpm_process_instance.rs:13）
    const instanceId = 'instance_id' in row ? row.instance_id : row.id;
    if (!instanceId) {
      ElMessage.warning(t('bpm.message.instanceIdNotFound'));
      return;
    }
    const res = await getBpmApprovalChain(String(instanceId));
    const chain = res.data || [];
    if (chain.length === 0) {
      await ElMessageBox.alert(
        t('bpm.message.noApprovalChain'),
        t('bpm.message.approvalChainTitle'),
        { confirmButtonText: t('bpm.message.close') }
      );
      return;
    }
    // 审批链条目字段以后端 ApprovalChainNode 为准
    //（services/bpm_service_dto.rs:24-34）：无 order/approver_name/approved_at，
    // 序号由前端按下标生成，处理人=assignee_name，完成时间=completed_at
    const lines = chain.map(
      (item, index) =>
        `${index + 1}. ${item.node_name}${item.assignee_name ? `（${item.assignee_name}）` : ''} - ${item.status}${item.comment ? `（${item.comment}）` : ''}${item.completed_at ? ` @ ${item.completed_at}` : ''}`
    );
    await ElMessageBox.alert(lines.join('\n'), t('bpm.detail.traceTitle', { instanceId }), {
      confirmButtonText: t('bpm.message.close'),
    });
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e));
    const err = e as Error;
    ElMessage.error(err.message || t('bpm.message.fetchApprovalChainFailed'));
  }
};
// 批次 157d-3 修复：接入 cancelInstance API 真实撤回流程
const handleCancel = async (row: BPMInstance) => {
  try {
    const confirmRes = await ElMessageBox.confirm(
      // 面向用户展示实例单号 instance_no（models/bpm_process_instance.rs:15）
      t('bpm.message.cancelConfirm', { instanceId: row.instance_no }),
      t('bpm.message.cancelConfirmTitle'),
      {
        type: 'warning',
        confirmButtonText: t('bpm.message.cancelConfirmButton'),
        cancelButtonText: t('bpm.message.cancelCancelButton'),
        inputPlaceholder: t('bpm.message.cancelReasonPlaceholder'),
        showInput: true,
        inputType: 'textarea',
      }
    );
    const reason =
      typeof confirmRes === 'string' && confirmRes.trim() ? confirmRes.trim() : undefined;
    // 后端 cancel_instance 接收 i32 主键 id（非字符串 instance_no）
    await cancelBpmInstance(row.id, reason);
    ElMessage.success(t('bpm.message.cancelSuccess'));
    // 撤回按钮位于监控 tab 的流程实例表内，刷新该列表即可
    fetchProcessInstances();
  } catch (e: unknown) {
    if (e === 'cancel' || e === 'close') return;
    const err = e as Error;
    ElMessage.error(err.message || t('bpm.message.cancelFailed'));
    logger.error(t('bpm.message.withdrawFailed'), err.message);
  }
};
// 批次 157a P1-1 修复：接入 getInstanceDetail API 展示流程实例详情
const handleViewProcess = async (row: BPMInstance) => {
  try {
    // 后端 detail 路径参数为实例主键 id（handlers/bpm_handler.rs:156-158，
    // services/bpm_ops/instance.rs:386 find_by_id）
    const res = await getBpmInstanceById(String(row.id));
    const d = res.data;
    if (!d) {
      ElMessage.warning(t('bpm.message.instanceDetailNotFound'));
      return;
    }
    const lines = buildDetailLines(d);
    await ElMessageBox.alert(lines.join('\n'), t('bpm.detail.processDetailTitle'), {
      confirmButtonText: t('bpm.message.close'),
    });
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e));
    const err = e as Error;
    ElMessage.error(err.message || t('bpm.message.fetchProcessDetailFailed'));
  }
};
// 批次 157a P1-1 修复：接入 getProcessVisualization API 展示流程图信息
const handleProcessImage = async (row: BPMInstance) => {
  try {
    // 后端 visualization 路径参数为实例主键 id（handlers/bpm_handler.rs:81-88 find_by_id）
    const res = await getBpmProcessVisualization(String(row.id));
    const d = res.data;
    if (!d) {
      ElMessage.warning(t('bpm.message.processImageNotFound'));
      return;
    }
    // 后端返回嵌套 { instance, definition, tasks, timeline }（handlers/bpm_handler.rs:120-138）
    const lines = [
      `${t('bpm.detail.instanceId')}：${d.instance.instance_no}`,
      `${t('bpm.detail.processName')}：${d.definition?.name ?? ''}`,
      `${t('bpm.detail.currentStatus')}：${getProcessStatusText(d.instance.status ?? '')}`,
      `${t('bpm.detail.activityHistory')}：${d.tasks.map(node => node.node_name).join(' → ')}`,
    ];
    await ElMessageBox.alert(
      lines.join('\n'),
      // 面向用户展示实例单号 instance_no（models/bpm_process_instance.rs:15）
      t('bpm.detail.processImageTitle', { instanceId: row.instance_no }),
      {
        confirmButtonText: t('bpm.message.close'),
      }
    );
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e));
    const err = e as Error;
    ElMessage.error(err.message || t('bpm.message.fetchProcessImageFailed'));
  }
};

onMounted(() => {
  fetchPendingTasks();
  fetchMonitorStats();
});
</script>

<style scoped>
.bpm-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 24px;
}
.header-left .page-title {
  font-size: 28px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 12px 0;
}
.stats-row {
  margin-bottom: 20px;
}
.stat-card {
  border-radius: 12px;
  transition: all 0.3s ease;
}
.stat-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
}
.stat-card.highlight {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}
.stat-card.highlight .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}
.stat-card.highlight .stat-label,
.stat-card.highlight .stat-value {
  color: white;
}
.stat-card.warning {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
}
.stat-card.warning .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}
.stat-card.warning .stat-label,
.stat-card.warning .stat-value {
  color: white;
}
.stat-content {
  display: flex;
  align-items: center;
  gap: 16px;
}
.stat-icon {
  width: 56px;
  height: 56px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 28px;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
}
.stat-icon.pending-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}
.stat-icon.completed-icon {
  background: linear-gradient(135deg, #43e97b 0%, #38f9d7 100%);
}
.stat-icon.urgent-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}
.stat-icon.avg-icon {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
}
.stat-info {
  flex: 1;
}
.stat-label {
  font-size: 14px;
  color: #909399;
  margin-bottom: 4px;
}
.stat-value {
  font-size: 28px;
  font-weight: 700;
  color: #303133;
  line-height: 1.2;
}
.table-card {
  margin-bottom: 20px;
}
.monitor-toolbar {
  display: flex;
  gap: 12px;
  margin-bottom: 16px;
}
.relation-result {
  margin-top: 16px;
}
.overdue {
  color: #f56c6c;
  font-weight: 600;
}
:deep(.el-tabs__header) {
  margin-bottom: 20px;
}
:deep(.el-card__header) {
  padding: 16px 20px;
  border-bottom: 1px solid #ebeef5;
}
:deep(.el-card__body) {
  padding: 20px;
}
</style>

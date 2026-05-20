<template>
  <div class="bpm-approval-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">审批中心</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>审批管理</el-breadcrumb-item>
          <el-breadcrumb-item>审批中心</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
    </div>

    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card pending">
          <div class="stat-content">
            <div class="stat-icon pending-icon"><el-icon><Clock /></el-icon></div>
            <div class="stat-info">
              <div class="stat-label">待办任务</div>
              <div class="stat-value">{{ stats.pending }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card completed">
          <div class="stat-content">
            <div class="stat-icon completed-icon"><el-icon><CircleCheck /></el-icon></div>
            <div class="stat-info">
              <div class="stat-label">已办任务</div>
              <div class="stat-value">{{ stats.completed }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card urgent">
          <div class="stat-content">
            <div class="stat-icon urgent-icon"><el-icon><Warning /></el-icon></div>
            <div class="stat-info">
              <div class="stat-label">紧急任务</div>
              <div class="stat-value">{{ stats.urgent }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card avg">
          <div class="stat-content">
            <div class="stat-icon avg-icon"><el-icon><Timer /></el-icon></div>
            <div class="stat-info">
              <div class="stat-label">平均处理时长</div>
              <div class="stat-value">{{ stats.avgTime }}h</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-tabs v-model="activeTab" @tab-change="handleTabChange">
      <el-tab-pane label="待办任务" name="pending">
        <el-card shadow="hover" class="table-card">
          <el-table :data="pendingTasks" stripe v-loading="pendingLoading">
            <el-table-column prop="task_name" label="任务名称" min-width="180" fixed />
            <el-table-column prop="process_name" label="流程名称" width="150" />
            <el-table-column prop="start_user_name" label="申请人" width="120" />
            <el-table-column prop="business_key" label="业务单号" width="160" />
            <el-table-column prop="created_at" label="申请时间" width="160" />
            <el-table-column prop="due_date" label="截止时间" width="160">
              <template #default="{ row }">
                <span v-if="row.due_date" :class="{ 'overdue': isOverdue(row.due_date) }">{{ row.due_date }}</span>
                <span v-else>-</span>
              </template>
            </el-table-column>
            <el-table-column prop="priority" label="优先级" width="100">
              <template #default="{ row }">
                <el-tag :type="getPriorityType(row.priority)" size="small">{{ getPriorityText(row.priority) }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="220" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="handleApprove(row)">同意</el-button>
                <el-button type="danger" link size="small" @click="handleReject(row)">拒绝</el-button>
                <el-button type="warning" link size="small" @click="handleTransfer(row)">转交</el-button>
                <el-button type="info" link size="small" @click="handleViewChain(row)">审批链</el-button>
              </template>
            </el-table-column>
          </el-table>
          <div class="pagination-wrapper">
            <el-pagination
              v-model:current-page="pendingPagination.page"
              v-model:page-size="pendingPagination.page_size"
              :total="pendingPagination.total"
              :page-sizes="[10, 20, 50]"
              layout="total, sizes, prev, pager, next"
              @size-change="fetchPendingTasks"
              @current-change="fetchPendingTasks"
            />
          </div>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="已办任务" name="completed">
        <el-card shadow="hover" class="table-card">
          <el-table :data="completedTasks" stripe v-loading="completedLoading">
            <el-table-column prop="task_name" label="任务名称" min-width="180" />
            <el-table-column prop="process_name" label="流程名称" width="150" />
            <el-table-column prop="start_user_name" label="申请人" width="120" />
            <el-table-column prop="business_key" label="业务单号" width="160" />
            <el-table-column prop="approved_at" label="审批时间" width="160" />
            <el-table-column prop="result" label="审批结果" width="100">
              <template #default="{ row }">
                <el-tag :type="row.result === 'approved' ? 'success' : 'danger'" size="small">
                  {{ row.result === 'approved' ? '同意' : '拒绝' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="comment" label="审批意见" min-width="200" show-overflow-tooltip />
            <el-table-column label="操作" width="120">
              <template #default="{ row }">
                <el-button type="info" link size="small" @click="handleViewChain(row)">审批链</el-button>
              </template>
            </el-table-column>
          </el-table>
          <div class="pagination-wrapper">
            <el-pagination
              v-model:current-page="completedPagination.page"
              v-model:page-size="completedPagination.page_size"
              :total="completedPagination.total"
              :page-sizes="[10, 20, 50]"
              layout="total, sizes, prev, pager, next"
              @size-change="fetchCompletedTasks"
              @current-change="fetchCompletedTasks"
            />
          </div>
        </el-card>
      </el-tab-pane>
    </el-tabs>

    <el-dialog v-model="approveDialogVisible" :title="approveAction === 'approve' ? '审批通过' : '审批拒绝'" width="500px" destroy-on-close>
      <el-form :model="approveForm" label-width="80px">
        <el-form-item label="任务名称">
          <span>{{ currentTask?.task_name }}</span>
        </el-form-item>
        <el-form-item label="审批意见">
          <el-input v-model="approveForm.comment" type="textarea" :rows="4" placeholder="请输入审批意见" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="approveDialogVisible = false">取消</el-button>
        <el-button :type="approveAction === 'approve' ? 'success' : 'danger'" @click="confirmApproval" :loading="submitLoading">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="transferDialogVisible" title="转交任务" width="500px" destroy-on-close>
      <el-form :model="transferForm" :rules="transferRules" ref="transferFormRef" label-width="100px">
        <el-form-item label="任务名称">
          <span>{{ currentTask?.task_name }}</span>
        </el-form-item>
        <el-form-item label="接收人 ID" prop="target_user_id">
          <el-input-number v-model="transferForm.target_user_id" :min="1" style="width: 100%" />
        </el-form-item>
        <el-form-item label="转交原因">
          <el-input v-model="transferForm.comment" type="textarea" :rows="3" placeholder="请输入转交原因" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="transferDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="confirmTransfer" :loading="submitLoading">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="chainDialogVisible" title="审批链" width="700px" destroy-on-close>
      <div v-if="approvalChain.length > 0" class="approval-chain">
        <div v-for="(node, index) in approvalChain" :key="index" class="chain-item">
          <div class="chain-node" :class="getNodeStatusClass(node.status)">
            <div class="node-order">{{ node.order }}</div>
            <div class="node-content">
              <div class="node-name">{{ node.node_name }}</div>
              <div class="node-type">{{ getNodeTypeName(node.node_type) }}</div>
              <div class="node-approver" v-if="node.approver_name">审批人：{{ node.approver_name }}</div>
              <div class="node-time" v-if="node.approved_at">{{ node.approved_at }}</div>
              <div class="node-comment" v-if="node.comment">意见：{{ node.comment }}</div>
              <div class="node-duration" v-if="node.duration">耗时：{{ node.duration }}分钟</div>
            </div>
          </div>
          <div v-if="index < approvalChain.length - 1" class="chain-arrow">
            <el-icon><ArrowDown /></el-icon>
          </div>
        </div>
      </div>
      <el-empty v-else description="暂无审批链数据" />
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { Clock, CircleCheck, Warning, Timer, ArrowDown } from '@element-plus/icons-vue'
import { bpmEnhancedApi } from '@/api/bpm-enhanced'
import type { ApprovalTask, ApprovalChainNode } from '@/api/bpm-enhanced'

const activeTab = ref('pending')

const stats = reactive({ pending: 0, completed: 0, urgent: 0, avgTime: 0 })

const pendingLoading = ref(false)
const completedLoading = ref(false)
const submitLoading = ref(false)
const pendingTasks = ref<ApprovalTask[]>([])
const completedTasks = ref<ApprovalTask[]>([])

const pendingPagination = reactive({ page: 1, page_size: 10, total: 0 })
const completedPagination = reactive({ page: 1, page_size: 10, total: 0 })

const approveDialogVisible = ref(false)
const approveAction = ref<'approve' | 'reject'>('approve')
const currentTask = ref<ApprovalTask | null>(null)
const approveForm = reactive({ comment: '' })

const transferDialogVisible = ref(false)
const transferFormRef = ref<FormInstance>()
const transferForm = reactive({ target_user_id: 1, comment: '' })
const transferRules: FormRules = {
  target_user_id: [{ required: true, message: '请输入接收人 ID', trigger: 'blur' }]
}

const chainDialogVisible = ref(false)
const approvalChain = ref<ApprovalChainNode[]>([])

const getPriorityType = (priority: string) => {
  const map: Record<string, any> = { high: 'danger', medium: 'warning', low: 'info' }
  return map[priority] || 'info'
}

const getPriorityText = (priority: string) => {
  const map: Record<string, string> = { high: '高', medium: '中', low: '低' }
  return map[priority] || priority
}

const isOverdue = (dueDate: string) => new Date(dueDate) < new Date()

const getNodeStatusClass = (status: string) => {
  const map: Record<string, string> = { pending: 'status-pending', approved: 'status-approved', rejected: 'status-rejected', skipped: 'status-skipped' }
  return map[status] || ''
}

const getNodeTypeName = (type: string) => {
  const map: Record<string, string> = { start: '开始', end: '结束', approval: '审批', condition: '条件', notify: '通知' }
  return map[type] || type
}

const fetchPendingTasks = async () => {
  pendingLoading.value = true
  try {
    const res = await bpmEnhancedApi.getPendingTasks({ page: pendingPagination.page, page_size: pendingPagination.page_size })
    pendingTasks.value = res.data.list
    pendingPagination.total = res.data.total
    stats.pending = res.data.total
    stats.urgent = res.data.list.filter(t => t.priority === 'high' && !isOverdue(t.due_date || '')).length
  } catch (e) {
    console.error(e)
  } finally {
    pendingLoading.value = false
  }
}

const fetchCompletedTasks = async () => {
  completedLoading.value = true
  try {
    const res = await bpmEnhancedApi.getCompletedTasks({ page: completedPagination.page, page_size: completedPagination.page_size })
    completedTasks.value = res.data.list
    completedPagination.total = res.data.total
    stats.completed = res.data.total
  } catch (e) {
    console.error(e)
  } finally {
    completedLoading.value = false
  }
}

const handleTabChange = (tab: string) => {
  if (tab === 'pending') fetchPendingTasks()
  else if (tab === 'completed') fetchCompletedTasks()
}

const handleApprove = (row: ApprovalTask) => {
  currentTask.value = row
  approveAction.value = 'approve'
  approveForm.comment = ''
  approveDialogVisible.value = true
}

const handleReject = (row: ApprovalTask) => {
  currentTask.value = row
  approveAction.value = 'reject'
  approveForm.comment = ''
  approveDialogVisible.value = true
}

const confirmApproval = async () => {
  if (!currentTask.value) return
  submitLoading.value = true
  try {
    await bpmEnhancedApi.executeApproval({
      task_id: currentTask.value.task_id,
      action: approveAction.value,
      comment: approveForm.comment
    })
    ElMessage.success(approveAction.value === 'approve' ? '审批通过' : '审批拒绝')
    approveDialogVisible.value = false
    fetchPendingTasks()
  } catch (e) {
    console.error(e)
  } finally {
    submitLoading.value = false
  }
}

const handleTransfer = (row: ApprovalTask) => {
  currentTask.value = row
  transferForm.target_user_id = 1
  transferForm.comment = ''
  transferDialogVisible.value = true
}

const confirmTransfer = async () => {
  if (!currentTask.value || !transferFormRef.value) return
  await transferFormRef.value.validate(async (valid) => {
    if (!valid) return
    submitLoading.value = true
    try {
      await bpmEnhancedApi.executeApproval({
        task_id: currentTask.value!.task_id,
        action: 'transfer',
        target_user_id: transferForm.target_user_id,
        comment: transferForm.comment
      })
      ElMessage.success('转交成功')
      transferDialogVisible.value = false
      fetchPendingTasks()
    } catch (e) {
      console.error(e)
    } finally {
      submitLoading.value = false
    }
  })
}

const handleViewChain = async (row: ApprovalTask) => {
  currentTask.value = row
  chainDialogVisible.value = true
  try {
    const res = await bpmEnhancedApi.getApprovalChain(row.process_instance_id)
    approvalChain.value = res.data
  } catch (e) {
    console.error(e)
  }
}

onMounted(() => {
  fetchPendingTasks()
  fetchCompletedTasks()
})
</script>

<style scoped>
.bpm-approval-page { padding: 24px; background-color: #f5f7fa; min-height: 100%; }
.page-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 24px; }
.header-left .page-title { font-size: 28px; font-weight: 600; color: #303133; margin: 0 0 12px 0; }
.stats-row { margin-bottom: 20px; }
.stat-card { border-radius: 12px; transition: all 0.3s ease; }
.stat-card:hover { transform: translateY(-4px); box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12); }
.stat-content { display: flex; align-items: center; gap: 16px; }
.stat-icon { width: 56px; height: 56px; border-radius: 12px; display: flex; align-items: center; justify-content: center; font-size: 28px; color: white; }
.pending-icon { background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%); }
.completed-icon { background: linear-gradient(135deg, #43e97b 0%, #38f9d7 100%); }
.urgent-icon { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); }
.avg-icon { background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%); }
.stat-info { flex: 1; }
.stat-label { font-size: 14px; color: #909399; margin-bottom: 4px; }
.stat-value { font-size: 28px; font-weight: 700; color: #303133; line-height: 1.2; }
.table-card { margin-bottom: 20px; }
.pagination-wrapper { display: flex; justify-content: flex-end; margin-top: 20px; }
.overdue { color: #f56c6c; font-weight: 600; }
.approval-chain { max-height: 500px; overflow-y: auto; padding: 8px 0; }
.chain-item { display: flex; flex-direction: column; align-items: center; }
.chain-node { display: flex; gap: 16px; padding: 16px; border-radius: 8px; border: 2px solid #e4e7ed; background: #fff; width: 100%; }
.chain-node.status-approved { border-color: #67c23a; background: #f0f9eb; }
.chain-node.status-rejected { border-color: #f56c6c; background: #fef0f0; }
.chain-node.status-skipped { border-color: #909399; background: #f4f4f5; opacity: 0.7; }
.node-order { width: 36px; height: 36px; border-radius: 50%; background: #409eff; color: white; display: flex; align-items: center; justify-content: center; font-weight: 600; flex-shrink: 0; }
.chain-node.status-approved .node-order { background: #67c23a; }
.chain-node.status-rejected .node-order { background: #f56c6c; }
.node-content { flex: 1; }
.node-name { font-size: 16px; font-weight: 600; color: #303133; }
.node-type { font-size: 12px; color: #909399; margin-top: 4px; }
.node-approver { font-size: 14px; color: #606266; margin-top: 8px; }
.node-time { font-size: 12px; color: #909399; margin-top: 4px; }
.node-comment { font-size: 13px; color: #409eff; margin-top: 8px; padding: 8px; background: #ecf5ff; border-radius: 4px; }
.node-duration { font-size: 12px; color: #e6a23c; margin-top: 4px; }
.chain-arrow { color: #c0c4cc; padding: 4px 0; }
</style>

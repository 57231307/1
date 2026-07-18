<template>
  <div class="color-card-issue">
    <el-page-header :icon="ArrowLeft" content="返回列表" @back="$router.push('/color-cards/list')" />

    <el-card style="margin-top: 16px">
      <template #header>
        <span>发放 / 归还 / 遗失 / 损坏 / 取消登记</span>
      </template>

      <el-tabs v-model="activeTab">
        <!-- 发放 -->
        <el-tab-pane label="发放色卡" name="issue">
          <ColorCardIssueForm
            :available-cards="availableCards"
            :loading="actionLoading"
            @submit="handleIssueSubmit"
          />
        </el-tab-pane>

        <!-- 发放中列表 -->
        <el-tab-pane :label="`发放中 (${activeIssues.length})`" name="active">
          <el-table :data="activeIssues" border>
            <el-table-column prop="color_card_id" label="色卡 ID" width="100" />
            <el-table-column label="客户 ID" prop="customer_id" width="100" />
            <el-table-column label="发放数量" prop="issue_qty" width="100" />
            <el-table-column label="经办人" prop="issued_by" width="100" />
            <el-table-column label="发放时间" width="180">
              <template #default="{ row }">{{ formatDate(row.issued_at) }}</template>
            </el-table-column>
            <el-table-column label="预计归还" width="180">
              <template #default="{ row }">{{ formatDate(row.expected_return_date) }}</template>
            </el-table-column>
            <el-table-column label="染色批号" prop="dye_lot_no" width="140" />
            <el-table-column label="用途" prop="purpose" />
            <el-table-column label="操作" width="360" fixed="right">
              <template #default="{ row }">
                <el-button link type="primary" @click="openAction(row, 'return')">归还</el-button>
                <el-button link type="warning" @click="openAction(row, 'damaged')">损坏</el-button>
                <el-button link type="danger" @click="openAction(row, 'lost')">遗失</el-button>
                <el-button link type="info" @click="openAction(row, 'cancel')">取消</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>

        <!-- 历史 -->
        <el-tab-pane label="历史记录" name="history">
          <el-table :data="historyRecords" border>
            <el-table-column prop="id" label="记录 ID" width="100" />
            <el-table-column prop="color_card_id" label="色卡 ID" width="100" />
            <el-table-column prop="customer_id" label="客户 ID" width="100" />
            <el-table-column prop="issue_qty" label="发放数量" width="100" />
            <el-table-column label="状态" width="100">
              <template #default="{ row }">
                <el-tag :type="(ISSUE_STATUS_COLORS[row.status] as 'success' | 'warning' | 'danger' | 'info')">
                  {{ ISSUE_STATUS[row.status as keyof typeof ISSUE_STATUS] || row.status }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="发放时间" width="180">
              <template #default="{ row }">{{ formatDate(row.issued_at) }}</template>
            </el-table-column>
            <el-table-column label="实际归还" width="180">
              <template #default="{ row }">{{ formatDate(row.actual_return_date) }}</template>
            </el-table-column>
            <el-table-column label="赔付金额" width="120">
              <template #default="{ row }">{{ row.compensation_amount || '-' }}</template>
            </el-table-column>
            <el-table-column label="染色批号" prop="dye_lot_no" width="140" />
            <el-table-column label="用途" prop="purpose" />
          </el-table>
        </el-tab-pane>
      </el-tabs>
    </el-card>

    <!-- V15 P0-F11：详情操作对话框（归还/遗失/损坏/取消 4 合 1） -->
    <ColorCardIssueDetail
      :record="currentRecord"
      :action="currentAction"
      :visible="detailVisible"
      :loading="actionLoading"
      @update:visible="detailVisible = $event"
      @confirm="handleConfirm"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { ArrowLeft } from '@element-plus/icons-vue'
import { ISSUE_STATUS, ISSUE_STATUS_COLORS } from '@/api/color-card'
import type { IssueRecordInfo } from '@/api/color-card'
import ColorCardIssueForm from '@/components/color-cards/ColorCardIssueForm.vue'
import ColorCardIssueDetail from '@/components/color-cards/ColorCardIssueDetail.vue'
import { useColorCardIssue } from '@/composables/useColorCardIssue'
import type { IssueFormState, IssueAction, ReturnDialogState, LostDialogState, DamagedDialogState, CancelDialogState } from '@/types/colorCardIssue'

// V15 P0-F11：从 composable 获取状态与业务函数
const {
  availableCards,
  activeIssues,
  historyRecords,
  actionLoading,
  init,
  handleIssue,
  handleReturn,
  handleMarkLost,
  handleMarkDamaged,
  handleCancel,
} = useColorCardIssue()

const activeTab = ref('issue')

// 详情对话框状态
const detailVisible = ref(false)
const currentRecord = ref<IssueRecordInfo | null>(null)
const currentAction = ref<IssueAction | null>(null)

function openAction(row: IssueRecordInfo, action: IssueAction): void {
  currentRecord.value = row
  currentAction.value = action
  detailVisible.value = true
}

async function handleIssueSubmit(form: IssueFormState): Promise<void> {
  const ok = await handleIssue(form)
  if (ok) {
    ElMessage.success('发放成功')
  }
}

async function handleConfirm(payload: {
  action: IssueAction
  recordId: number
  data: ReturnDialogState | LostDialogState | DamagedDialogState | CancelDialogState
}): Promise<void> {
  const { action, recordId, data } = payload
  let ok = false
  let msg = ''
  switch (action) {
    case 'return':
      ok = await handleReturn(recordId, data as ReturnDialogState)
      msg = '归还成功'
      break
    case 'lost':
      ok = await handleMarkLost(recordId, data as LostDialogState)
      msg = '已登记遗失'
      break
    case 'damaged':
      ok = await handleMarkDamaged(recordId, data as DamagedDialogState)
      msg = '已标记损坏'
      break
    case 'cancel':
      ok = await handleCancel(recordId, (data as CancelDialogState).remark)
      msg = '已取消发放'
      break
  }
  if (ok) {
    ElMessage.success(msg)
  }
}

const formatDate = (s?: string): string => (s ? new Date(s).toLocaleString('zh-CN') : '-')

onMounted(() => {
  init()
})
</script>

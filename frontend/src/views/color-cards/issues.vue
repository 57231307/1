<template>
  <div class="color-card-issue">
    <el-page-header :icon="ArrowLeft" content="返回列表" @back="$router.push('/color-cards/list')" />

    <el-card style="margin-top: 16px">
      <template #header>
        <span>发放 / 归还 / 遗失 / 损坏 / 取消登记</span>
      </template>

      <el-tabs v-model="activeTab" aria-label="色卡发放管理标签页">
        <!-- 发放 -->
        <el-tab-pane label="发放色卡" name="issue">
          <el-form :model="issueForm" :rules="issueRules" ref="issueFormRef" label-width="100px" style="max-width: 600px">
            <el-form-item label="选择色卡" prop="color_card_id">
              <el-select v-model="issueForm.color_card_id" filterable placeholder="搜索色卡编号或名称" style="width: 100%">
                <el-option
                  v-for="card in availableCards"
                  :key="card.id"
                  :label="`${card.card_no} - ${card.card_name}`"
                  :value="card.id"
                />
              </el-select>
            </el-form-item>
            <el-form-item label="客户 ID" prop="customer_id">
              <el-input-number v-model="issueForm.customer_id" :min="1" style="width: 100%" placeholder="请输入客户 ID" />
            </el-form-item>
            <el-form-item label="发放数量">
              <el-input-number v-model="issueForm.issue_qty" :min="1" :step="1" style="width: 100%" />
            </el-form-item>
            <el-form-item label="染色批号">
              <el-input v-model="issueForm.dye_lot_no" placeholder="可选: 染色批号（lot 概念，防色差混批）" />
            </el-form-item>
            <el-form-item label="预计归还">
              <el-date-picker
                v-model="issueForm.expected_return_date"
                type="date"
                placeholder="可选: 预计归还日期（不超过 30 天）"
                style="width: 100%"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
            <el-form-item label="用途">
              <el-input v-model="issueForm.purpose" placeholder="例如: 客户选色 / 展会展示" />
            </el-form-item>
            <el-form-item label="备注">
              <el-input v-model="issueForm.remark" type="textarea" :rows="2" />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" :loading="issuing" @click="handleIssue">确认发放</el-button>
            </el-form-item>
          </el-form>
        </el-tab-pane>

        <!-- 发放中列表 -->
        <el-tab-pane :label="`发放中 (${activeIssues.length})`" name="active">
          <el-table :data="activeIssues" border aria-label="发放中色卡列表">
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
                <el-button link type="primary" aria-label="归还色卡" @click="handleReturn(row)">归还</el-button>
                <el-button link type="warning" aria-label="标记色卡损坏" @click="handleMarkDamaged(row)">损坏</el-button>
                <el-button link type="danger" aria-label="登记色卡遗失" @click="handleMarkLost(row)">遗失</el-button>
                <el-button link type="info" aria-label="取消色卡发放" @click="handleCancel(row)">取消</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>

        <!-- 历史 -->
        <el-tab-pane label="历史记录" name="history">
          <el-table :data="historyRecords" border aria-label="色卡发放历史记录">
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

    <!-- 归还对话框 -->
    <el-dialog v-model="showReturnDialog" title="归还色卡" aria-label="归还色卡对话框" width="480px">
      <el-form label-width="80px" aria-label="归还色卡表单">
        <el-form-item label="实际归还">
          <el-date-picker v-model="returnForm.actual_return_date" type="date" value-format="YYYY-MM-DD" style="width: 100%" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="returnForm.remark" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showReturnDialog = false">取消</el-button>
        <el-button type="primary" :loading="actionLoading" @click="confirmReturn">确认归还</el-button>
      </template>
    </el-dialog>

    <!-- 遗失对话框 -->
    <el-dialog v-model="showLostDialog" title="登记遗失" aria-label="登记遗失对话框" width="480px">
      <el-alert type="warning" :closable="false" style="margin-bottom: 16px">
        登记遗失后色卡状态将变为「遗失」，需要填写赔付金额。
      </el-alert>
      <el-form label-width="100px" aria-label="登记遗失表单">
        <el-form-item label="赔付金额" required>
          <el-input-number v-model="lostForm.compensation_amount" :min="0.01" :precision="2" :step="100" style="width: 100%" />
        </el-form-item>
        <el-form-item label="遗失原因">
          <el-input v-model="lostForm.remark" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showLostDialog = false">取消</el-button>
        <el-button type="danger" :loading="actionLoading" @click="confirmLost">确认登记</el-button>
      </template>
    </el-dialog>

    <!-- 损坏对话框 -->
    <el-dialog v-model="showDamagedDialog" title="标记损坏" aria-label="标记损坏对话框" width="480px">
      <el-form label-width="100px" aria-label="标记损坏表单">
        <el-form-item label="赔付金额">
          <el-input-number v-model="damagedForm.compensation_amount" :min="0" :precision="2" :step="100" style="width: 100%" />
        </el-form-item>
        <el-form-item label="损坏原因">
          <el-input v-model="damagedForm.remark" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showDamagedDialog = false">取消</el-button>
        <el-button type="warning" :loading="actionLoading" @click="confirmDamaged">确认标记</el-button>
      </template>
    </el-dialog>

    <!-- 取消对话框 -->
    <el-dialog v-model="showCancelDialog" title="取消发放" aria-label="取消发放对话框" width="480px">
      <el-alert type="info" :closable="false" style="margin-bottom: 16px">
        取消发放后记录将变为「已取消」终态，不可再变更。
      </el-alert>
      <el-form label-width="80px" aria-label="取消发放表单">
        <el-form-item label="取消原因">
          <el-input v-model="cancelForm.remark" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showCancelDialog = false">关闭</el-button>
        <el-button type="info" :loading="actionLoading" @click="confirmCancel">确认取消</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, computed } from 'vue'
import { ElMessage } from 'element-plus'
import { ArrowLeft } from '@element-plus/icons-vue'
import {
  listColorCards,
  listIssues,
  issueColorCard,
  returnIssue,
  markIssueLost,
  markIssueDamaged,
  cancelIssue,
  ISSUE_STATUS,
  ISSUE_STATUS_COLORS,
  type IssueRecordInfo,
  type ColorCardListItem,
} from '@/api/color-card'

const activeTab = ref('issue')
const availableCards = ref<ColorCardListItem[]>([])
const issueRecords = ref<IssueRecordInfo[]>([])

// 发放中：状态为 issued 的记录
const activeIssues = computed(() => issueRecords.value.filter((r) => r.status === 'issued'))
// 历史：状态非 issued 的记录（已归还/遗失/损坏/已取消）
const historyRecords = computed(() => issueRecords.value.filter((r) => r.status !== 'issued'))

const issueFormRef = ref()
const issuing = ref(false)
const issueForm = reactive({
  color_card_id: undefined as number | undefined,
  customer_id: 1,
  issue_qty: 1,
  expected_return_date: '',
  purpose: '',
  remark: '',
  dye_lot_no: '',
})

const issueRules = {
  color_card_id: [{ required: true, message: '请选择色卡', trigger: 'change' }],
  customer_id: [{ required: true, message: '请输入客户 ID', trigger: 'blur' }],
}

const showReturnDialog = ref(false)
const showLostDialog = ref(false)
const showDamagedDialog = ref(false)
const showCancelDialog = ref(false)
const actionLoading = ref(false)
const currentRecordId = ref<number | null>(null)

const returnForm = reactive({ actual_return_date: '', remark: '' })
const lostForm = reactive({ compensation_amount: 0, remark: '' })
const damagedForm = reactive({ compensation_amount: 0, remark: '' })
const cancelForm = reactive({ remark: '' })

const loadCards = async () => {
  const res = await listColorCards({ status: 'active', page_size: 200 })
  availableCards.value = res.data?.items || []
}

const loadRecords = async () => {
  const res = await listIssues({ page_size: 100 })
  issueRecords.value = res.data?.items || []
}

const handleIssue = async () => {
  if (!issueFormRef.value) return
  await issueFormRef.value.validate(async (valid: boolean) => {
    if (!valid) return
    issuing.value = true
    try {
      await issueColorCard({
        color_card_id: issueForm.color_card_id!,
        customer_id: issueForm.customer_id,
        issue_qty: issueForm.issue_qty || 1,
        expected_return_date: issueForm.expected_return_date || undefined,
        purpose: issueForm.purpose || undefined,
        remark: issueForm.remark || undefined,
        dye_lot_no: issueForm.dye_lot_no || undefined,
      })
      ElMessage.success('发放成功')
      issueForm.color_card_id = undefined
      issueForm.purpose = ''
      issueForm.remark = ''
      issueForm.dye_lot_no = ''
      loadRecords()
    } finally {
      issuing.value = false
    }
  })
}

const handleReturn = (row: IssueRecordInfo) => {
  currentRecordId.value = row.id
  returnForm.actual_return_date = ''
  returnForm.remark = ''
  showReturnDialog.value = true
}

const confirmReturn = async () => {
  if (!currentRecordId.value) return
  actionLoading.value = true
  try {
    await returnIssue(currentRecordId.value, {
      actual_return_date: returnForm.actual_return_date || undefined,
      remark: returnForm.remark || undefined,
    })
    ElMessage.success('归还成功')
    showReturnDialog.value = false
    loadRecords()
  } finally {
    actionLoading.value = false
  }
}

const handleMarkLost = (row: IssueRecordInfo) => {
  currentRecordId.value = row.id
  lostForm.compensation_amount = 0
  lostForm.remark = ''
  showLostDialog.value = true
}

const confirmLost = async () => {
  if (!currentRecordId.value || lostForm.compensation_amount <= 0) {
    ElMessage.warning('请填写赔付金额（必须 > 0）')
    return
  }
  actionLoading.value = true
  try {
    await markIssueLost(currentRecordId.value, {
      compensation_amount: lostForm.compensation_amount,
      remark: lostForm.remark || undefined,
    })
    ElMessage.success('已登记遗失')
    showLostDialog.value = false
    loadRecords()
  } finally {
    actionLoading.value = false
  }
}

const handleMarkDamaged = (row: IssueRecordInfo) => {
  currentRecordId.value = row.id
  damagedForm.compensation_amount = 0
  damagedForm.remark = ''
  showDamagedDialog.value = true
}

const confirmDamaged = async () => {
  if (!currentRecordId.value) return
  actionLoading.value = true
  try {
    await markIssueDamaged(currentRecordId.value, {
      compensation_amount: damagedForm.compensation_amount || undefined,
      remark: damagedForm.remark || undefined,
    })
    ElMessage.success('已标记损坏')
    showDamagedDialog.value = false
    loadRecords()
  } finally {
    actionLoading.value = false
  }
}

const handleCancel = (row: IssueRecordInfo) => {
  currentRecordId.value = row.id
  cancelForm.remark = ''
  showCancelDialog.value = true
}

const confirmCancel = async () => {
  if (!currentRecordId.value) return
  actionLoading.value = true
  try {
    await cancelIssue(currentRecordId.value, {
      remark: cancelForm.remark || undefined,
    })
    ElMessage.success('已取消发放')
    showCancelDialog.value = false
    loadRecords()
  } finally {
    actionLoading.value = false
  }
}

const formatDate = (s?: string) => (s ? new Date(s).toLocaleString('zh-CN') : '-')

onMounted(() => {
  loadCards()
  loadRecords()
})
</script>

<style scoped>
.color-card-issue { padding: 16px; }
</style>

<template>
  <div class="color-card-issue">
    <el-page-header :icon="ArrowLeft" :content="$t('colorCards.issue.back')" @back="$router.push('/color-cards/list')" />

    <el-card style="margin-top: 16px">
      <template #header>
        <span>{{ $t('colorCards.issue.title') }}</span>
      </template>

      <el-tabs v-model="activeTab" :aria-label="$t('colorCards.issue.tabAriaLabel')">
        <!-- 发放 -->
        <el-tab-pane :label="$t('colorCards.issue.tab.issue')" name="issue">
          <el-form :model="issueForm" :rules="issueRules" ref="issueFormRef" label-width="100px" style="max-width: 600px" :aria-label="$t('colorCards.issue.form.ariaLabel')">
            <el-form-item :label="$t('colorCards.issue.form.selectCard')" prop="color_card_id">
              <el-select v-model="issueForm.color_card_id" filterable :placeholder="$t('colorCards.issue.form.selectCardPlaceholder')" style="width: 100%">
                <el-option
                  v-for="card in availableCards"
                  :key="card.id"
                  :label="`${card.card_no} - ${card.card_name}`"
                  :value="card.id"
                />
              </el-select>
            </el-form-item>
            <el-form-item :label="$t('colorCards.issue.form.customerId')" prop="customer_id">
              <el-input-number v-model="issueForm.customer_id" :min="1" style="width: 100%" :placeholder="$t('colorCards.issue.form.customerIdPlaceholder')" />
            </el-form-item>
            <el-form-item :label="$t('colorCards.issue.form.issueQty')">
              <el-input-number v-model="issueForm.issue_qty" :min="1" :step="1" style="width: 100%" />
            </el-form-item>
            <el-form-item :label="$t('colorCards.issue.form.dyeLotNo')">
              <el-input v-model="issueForm.dye_lot_no" :placeholder="$t('colorCards.issue.form.dyeLotNoPlaceholder')" />
            </el-form-item>
            <el-form-item :label="$t('colorCards.issue.form.expectedReturn')">
              <el-date-picker
                v-model="issueForm.expected_return_date"
                type="date"
                :placeholder="$t('colorCards.issue.form.expectedReturnPlaceholder')"
                style="width: 100%"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
            <el-form-item :label="$t('colorCards.issue.form.purpose')">
              <el-input v-model="issueForm.purpose" :placeholder="$t('colorCards.issue.form.purposePlaceholder')" />
            </el-form-item>
            <el-form-item :label="$t('colorCards.issue.form.remark')">
              <el-input v-model="issueForm.remark" type="textarea" :rows="2" />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" :loading="issuing" @click="handleIssue">{{ $t('colorCards.issue.form.confirmIssue') }}</el-button>
            </el-form-item>
          </el-form>
        </el-tab-pane>

        <!-- 发放中列表 -->
        <el-tab-pane :label="$t('colorCards.issue.tab.active', { count: activeIssues.length })" name="active">
          <el-table :data="activeIssues" border :aria-label="$t('colorCards.issue.activeTable.ariaLabel')">
            <el-table-column prop="color_card_id" :label="$t('colorCards.issue.activeTable.cardId')" width="100" />
            <el-table-column :label="$t('colorCards.issue.activeTable.customerId')" prop="customer_id" width="100" />
            <el-table-column :label="$t('colorCards.issue.activeTable.issueQty')" prop="issue_qty" width="100" />
            <el-table-column :label="$t('colorCards.issue.activeTable.issuedBy')" prop="issued_by" width="100" />
            <el-table-column :label="$t('colorCards.issue.activeTable.issuedAt')" width="180">
              <template #default="{ row }">{{ formatDate(row.issued_at) }}</template>
            </el-table-column>
            <el-table-column :label="$t('colorCards.issue.activeTable.expectedReturn')" width="180">
              <template #default="{ row }">{{ formatDate(row.expected_return_date) }}</template>
            </el-table-column>
            <el-table-column :label="$t('colorCards.issue.activeTable.dyeLotNo')" prop="dye_lot_no" width="140" />
            <el-table-column :label="$t('colorCards.issue.activeTable.purpose')" prop="purpose" />
            <el-table-column :label="$t('colorCards.issue.activeTable.operation')" width="360" fixed="right">
              <template #default="{ row }">
                <el-button link type="primary" :aria-label="$t('colorCards.issue.activeTable.returnAriaLabel')" @click="handleReturn(row)">{{ $t('colorCards.issue.activeTable.return') }}</el-button>
                <el-button link type="warning" :aria-label="$t('colorCards.issue.activeTable.damagedAriaLabel')" @click="handleMarkDamaged(row)">{{ $t('colorCards.issue.activeTable.damaged') }}</el-button>
                <el-button link type="danger" :aria-label="$t('colorCards.issue.activeTable.lostAriaLabel')" @click="handleMarkLost(row)">{{ $t('colorCards.issue.activeTable.lost') }}</el-button>
                <el-button link type="info" :aria-label="$t('colorCards.issue.activeTable.cancelAriaLabel')" @click="handleCancel(row)">{{ $t('colorCards.issue.activeTable.cancel') }}</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>

        <!-- 历史 -->
        <el-tab-pane :label="$t('colorCards.issue.tab.history')" name="history">
          <el-table :data="historyRecords" border :aria-label="$t('colorCards.issue.historyTable.ariaLabel')">
            <el-table-column prop="id" :label="$t('colorCards.issue.historyTable.recordId')" width="100" />
            <el-table-column prop="color_card_id" :label="$t('colorCards.issue.historyTable.cardId')" width="100" />
            <el-table-column prop="customer_id" :label="$t('colorCards.issue.historyTable.customerId')" width="100" />
            <el-table-column prop="issue_qty" :label="$t('colorCards.issue.historyTable.issueQty')" width="100" />
            <el-table-column :label="$t('colorCards.issue.historyTable.status')" width="100">
              <template #default="{ row }">
                <el-tag :type="(ISSUE_STATUS_COLORS[row.status] as 'success' | 'warning' | 'danger' | 'info')">
                  {{ getIssueStatusLabel(row.status) || row.status }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column :label="$t('colorCards.issue.historyTable.issuedAt')" width="180">
              <template #default="{ row }">{{ formatDate(row.issued_at) }}</template>
            </el-table-column>
            <el-table-column :label="$t('colorCards.issue.historyTable.actualReturn')" width="180">
              <template #default="{ row }">{{ formatDate(row.actual_return_date) }}</template>
            </el-table-column>
            <el-table-column :label="$t('colorCards.issue.historyTable.compensationAmount')" width="120">
              <template #default="{ row }">{{ row.compensation_amount || '-' }}</template>
            </el-table-column>
            <el-table-column :label="$t('colorCards.issue.historyTable.dyeLotNo')" prop="dye_lot_no" width="140" />
            <el-table-column :label="$t('colorCards.issue.historyTable.purpose')" prop="purpose" />
          </el-table>
        </el-tab-pane>
      </el-tabs>
    </el-card>

    <!-- 归还对话框 -->
    <el-dialog v-model="showReturnDialog" :title="$t('colorCards.issue.returnDialog.title')" :aria-label="$t('colorCards.issue.returnDialog.ariaLabel')" width="480px">
      <el-form label-width="80px" :aria-label="$t('colorCards.issue.returnDialog.formAriaLabel')">
        <el-form-item :label="$t('colorCards.issue.returnDialog.actualReturn')">
          <el-date-picker v-model="returnForm.actual_return_date" type="date" value-format="YYYY-MM-DD" style="width: 100%" />
        </el-form-item>
        <el-form-item :label="$t('colorCards.issue.form.remark')">
          <el-input v-model="returnForm.remark" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showReturnDialog = false">{{ $t('colorCards.issue.returnDialog.cancel') }}</el-button>
        <el-button type="primary" :loading="actionLoading" @click="confirmReturn">{{ $t('colorCards.issue.returnDialog.confirm') }}</el-button>
      </template>
    </el-dialog>

    <!-- 遗失对话框 -->
    <el-dialog v-model="showLostDialog" :title="$t('colorCards.issue.lostDialog.title')" :aria-label="$t('colorCards.issue.lostDialog.ariaLabel')" width="480px">
      <el-alert type="warning" :closable="false" style="margin-bottom: 16px">
        {{ $t('colorCards.issue.lostDialog.alert') }}
      </el-alert>
      <el-form label-width="100px" :aria-label="$t('colorCards.issue.lostDialog.formAriaLabel')">
        <el-form-item :label="$t('colorCards.issue.lostDialog.compensationAmount')" required>
          <el-input-number v-model="lostForm.compensation_amount" :min="0.01" :precision="2" :step="100" style="width: 100%" />
        </el-form-item>
        <el-form-item :label="$t('colorCards.issue.lostDialog.lostReason')">
          <el-input v-model="lostForm.remark" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showLostDialog = false">{{ $t('colorCards.issue.lostDialog.cancel') }}</el-button>
        <el-button type="danger" :loading="actionLoading" @click="confirmLost">{{ $t('colorCards.issue.lostDialog.confirm') }}</el-button>
      </template>
    </el-dialog>

    <!-- 损坏对话框 -->
    <el-dialog v-model="showDamagedDialog" :title="$t('colorCards.issue.damagedDialog.title')" :aria-label="$t('colorCards.issue.damagedDialog.ariaLabel')" width="480px">
      <el-form label-width="100px" :aria-label="$t('colorCards.issue.damagedDialog.formAriaLabel')">
        <el-form-item :label="$t('colorCards.issue.damagedDialog.compensationAmount')">
          <el-input-number v-model="damagedForm.compensation_amount" :min="0" :precision="2" :step="100" style="width: 100%" />
        </el-form-item>
        <el-form-item :label="$t('colorCards.issue.damagedDialog.damagedReason')">
          <el-input v-model="damagedForm.remark" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showDamagedDialog = false">{{ $t('colorCards.issue.damagedDialog.cancel') }}</el-button>
        <el-button type="warning" :loading="actionLoading" @click="confirmDamaged">{{ $t('colorCards.issue.damagedDialog.confirm') }}</el-button>
      </template>
    </el-dialog>

    <!-- 取消对话框 -->
    <el-dialog v-model="showCancelDialog" :title="$t('colorCards.issue.cancelDialog.title')" :aria-label="$t('colorCards.issue.cancelDialog.ariaLabel')" width="480px">
      <el-alert type="info" :closable="false" style="margin-bottom: 16px">
        {{ $t('colorCards.issue.cancelDialog.alert') }}
      </el-alert>
      <el-form label-width="80px" :aria-label="$t('colorCards.issue.cancelDialog.formAriaLabel')">
        <el-form-item :label="$t('colorCards.issue.cancelDialog.cancelReason')">
          <el-input v-model="cancelForm.remark" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showCancelDialog = false">{{ $t('colorCards.issue.cancelDialog.close') }}</el-button>
        <el-button type="info" :loading="actionLoading" @click="confirmCancel">{{ $t('colorCards.issue.cancelDialog.confirm') }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { ArrowLeft } from '@element-plus/icons-vue'
import {
  getColorCardList,
  getIssueList,
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

const { t } = useI18n({ useScope: 'global' })

// 状态码 → 本地化标签（响应式：随语言切换自动更新）
const getIssueStatusLabel = (key: string) => t(`colorCards.issue.issueStatus.${key}`)

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

// 表单校验规则（响应式：随语言切换自动更新提示文案）
const issueRules = computed(() => ({
  color_card_id: [{ required: true, message: t('colorCards.issue.message.selectCardRequired'), trigger: 'change' }],
  customer_id: [{ required: true, message: t('colorCards.issue.message.customerIdRequired'), trigger: 'blur' }],
}))

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
  const res = await getColorCardList({ status: 'active', page_size: 200 })
  availableCards.value = res.data?.items || []
}

const loadRecords = async () => {
  const res = await getIssueList({ page_size: 100 })
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
      ElMessage.success(t('colorCards.issue.message.issueSuccess'))
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
    ElMessage.success(t('colorCards.issue.message.returnSuccess'))
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
    ElMessage.warning(t('colorCards.issue.message.compensationRequired'))
    return
  }
  actionLoading.value = true
  try {
    await markIssueLost(currentRecordId.value, {
      compensation_amount: lostForm.compensation_amount,
      remark: lostForm.remark || undefined,
    })
    ElMessage.success(t('colorCards.issue.message.lostSuccess'))
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
    ElMessage.success(t('colorCards.issue.message.damagedSuccess'))
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
    ElMessage.success(t('colorCards.issue.message.cancelSuccess'))
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

<template>
  <div class="color-card-borrow">
    <el-page-header :icon="ArrowLeft" content="返回列表" @back="$router.push('/color-cards/list')" />

    <el-card style="margin-top: 16px">
      <template #header>
        <span>借出 / 归还 / 遗失登记</span>
      </template>

      <el-tabs v-model="activeTab">
        <!-- 借出 -->
        <el-tab-pane label="借出色卡" name="borrow">
          <el-form :model="borrowForm" :rules="borrowRules" ref="borrowFormRef" label-width="100px" style="max-width: 600px">
            <el-form-item label="选择色卡" prop="color_card_id">
              <el-select v-model="borrowForm.color_card_id" filterable placeholder="搜索色卡编号或名称" style="width: 100%">
                <el-option
                  v-for="card in availableCards"
                  :key="card.id"
                  :label="`${card.card_no} - ${card.card_name}`"
                  :value="card.id"
                />
              </el-select>
            </el-form-item>
            <el-form-item label="客户 ID" prop="customer_id">
              <el-input-number v-model="borrowForm.customer_id" :min="1" style="width: 100%" placeholder="请输入客户 ID" />
            </el-form-item>
            <el-form-item label="预计归还">
              <el-date-picker
                v-model="borrowForm.expected_return_at"
                type="datetime"
                placeholder="可选: 预计归还时间"
                style="width: 100%"
                value-format="YYYY-MM-DDTHH:mm:ss[Z]"
              />
            </el-form-item>
            <el-form-item label="用途">
              <el-input v-model="borrowForm.purpose" placeholder="例如: 客户选色 / 展会展示" />
            </el-form-item>
            <el-form-item label="备注">
              <el-input v-model="borrowForm.notes" type="textarea" :rows="2" />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" :loading="borrowing" @click="handleBorrow">确认借出</el-button>
            </el-form-item>
          </el-form>
        </el-tab-pane>

        <!-- 借出中列表 -->
        <el-tab-pane :label="`借出中 (${activeBorrows.length})`" name="active">
          <el-table :data="activeBorrows" border>
            <el-table-column prop="color_card_id" label="色卡 ID" width="100" />
            <el-table-column label="客户 ID" prop="customer_id" width="100" />
            <el-table-column label="经办人" prop="borrowed_by" width="100" />
            <el-table-column label="借出时间" width="180">
              <template #default="{ row }">{{ formatDate(row.borrowed_at) }}</template>
            </el-table-column>
            <el-table-column label="预计归还" width="180">
              <template #default="{ row }">{{ formatDate(row.expected_return_at) }}</template>
            </el-table-column>
            <el-table-column label="用途" prop="purpose" />
            <el-table-column label="操作" width="280" fixed="right">
              <template #default="{ row }">
                <el-button link type="primary" @click="handleReturn(row)">归还</el-button>
                <el-button link type="warning" @click="handleMarkDamaged(row)">损坏</el-button>
                <el-button link type="danger" @click="handleMarkLost(row)">遗失</el-button>
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
            <el-table-column label="状态" width="100">
              <template #default="{ row }">
                <el-tag :type="(BORROW_STATUS_COLORS[row.status] as 'success' | 'warning' | 'danger' | 'info')">
                  {{ BORROW_STATUS[row.status as keyof typeof BORROW_STATUS] || row.status }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="借出时间" width="180">
              <template #default="{ row }">{{ formatDate(row.borrowed_at) }}</template>
            </el-table-column>
            <el-table-column label="实际归还" width="180">
              <template #default="{ row }">{{ formatDate(row.actual_return_at) }}</template>
            </el-table-column>
            <el-table-column label="赔付金额" width="120">
              <template #default="{ row }">{{ row.compensation_amount || '-' }}</template>
            </el-table-column>
            <el-table-column label="用途" prop="purpose" />
          </el-table>
        </el-tab-pane>
      </el-tabs>
    </el-card>

    <!-- 归还对话框 -->
    <el-dialog v-model="showReturnDialog" title="归还色卡" width="480px">
      <el-form label-width="80px">
        <el-form-item label="实际归还">
          <el-date-picker v-model="returnForm.actual_return_at" type="datetime" value-format="YYYY-MM-DDTHH:mm:ss[Z]" style="width: 100%" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="returnForm.notes" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showReturnDialog = false">取消</el-button>
        <el-button type="primary" :loading="actionLoading" @click="confirmReturn">确认归还</el-button>
      </template>
    </el-dialog>

    <!-- 遗失对话框 -->
    <el-dialog v-model="showLostDialog" title="登记遗失" width="480px">
      <el-alert type="warning" :closable="false" style="margin-bottom: 16px">
        登记遗失后色卡状态将变为「遗失」，需要填写赔付金额。
      </el-alert>
      <el-form label-width="100px">
        <el-form-item label="赔付金额" required>
          <el-input-number v-model="lostForm.compensation_amount" :min="0.01" :precision="2" :step="100" style="width: 100%" />
        </el-form-item>
        <el-form-item label="遗失原因">
          <el-input v-model="lostForm.notes" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showLostDialog = false">取消</el-button>
        <el-button type="danger" :loading="actionLoading" @click="confirmLost">确认登记</el-button>
      </template>
    </el-dialog>

    <!-- 损坏对话框 -->
    <el-dialog v-model="showDamagedDialog" title="标记损坏" width="480px">
      <el-form label-width="100px">
        <el-form-item label="赔付金额">
          <el-input-number v-model="damagedForm.compensation_amount" :min="0" :precision="2" :step="100" style="width: 100%" />
        </el-form-item>
        <el-form-item label="损坏原因">
          <el-input v-model="damagedForm.notes" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showDamagedDialog = false">取消</el-button>
        <el-button type="warning" :loading="actionLoading" @click="confirmDamaged">确认标记</el-button>
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
  listBorrowRecords,
  borrowColorCard,
  returnColorCard,
  markLostColorCard,
  markDamagedColorCard,
  BORROW_STATUS,
  BORROW_STATUS_COLORS,
  type BorrowRecordInfo,
  type ColorCardListItem,
} from '@/api/color-card'

const activeTab = ref('borrow')
const availableCards = ref<ColorCardListItem[]>([])
const borrowRecords = ref<BorrowRecordInfo[]>([])

const activeBorrows = computed(() => borrowRecords.value.filter((r) => r.status === 'borrowed'))
const historyRecords = computed(() => borrowRecords.value.filter((r) => r.status !== 'borrowed'))

const borrowFormRef = ref()
const borrowing = ref(false)
const borrowForm = reactive({
  color_card_id: undefined as number | undefined,
  customer_id: 1,
  expected_return_at: '',
  purpose: '',
  notes: '',
})

const borrowRules = {
  color_card_id: [{ required: true, message: '请选择色卡', trigger: 'change' }],
  customer_id: [{ required: true, message: '请输入客户 ID', trigger: 'blur' }],
}

const showReturnDialog = ref(false)
const showLostDialog = ref(false)
const showDamagedDialog = ref(false)
const actionLoading = ref(false)
const currentRecordId = ref<number | null>(null)

const returnForm = reactive({ actual_return_at: '', notes: '' })
const lostForm = reactive({ compensation_amount: 0, notes: '' })
const damagedForm = reactive({ compensation_amount: 0, notes: '' })

const loadCards = async () => {
  const res = await listColorCards({ status: 'active', page_size: 200 })
  availableCards.value = res.data?.items || []
}

const loadRecords = async () => {
  const res = await listBorrowRecords({ page_size: 100 })
  borrowRecords.value = res.data?.items || []
}

const handleBorrow = async () => {
  if (!borrowFormRef.value) return
  await borrowFormRef.value.validate(async (valid: boolean) => {
    if (!valid) return
    borrowing.value = true
    try {
      await borrowColorCard({
        color_card_id: borrowForm.color_card_id!,
        customer_id: borrowForm.customer_id,
        expected_return_at: borrowForm.expected_return_at || undefined,
        purpose: borrowForm.purpose || undefined,
        notes: borrowForm.notes || undefined,
      })
      ElMessage.success('借出成功')
      borrowForm.color_card_id = undefined
      borrowForm.purpose = ''
      borrowForm.notes = ''
      loadRecords()
    } finally {
      borrowing.value = false
    }
  })
}

const handleReturn = (row: BorrowRecordInfo) => {
  currentRecordId.value = row.id
  returnForm.actual_return_at = ''
  returnForm.notes = ''
  showReturnDialog.value = true
}

const confirmReturn = async () => {
  if (!currentRecordId.value) return
  actionLoading.value = true
  try {
    await returnColorCard(currentRecordId.value, {
      actual_return_at: returnForm.actual_return_at || undefined,
      notes: returnForm.notes || undefined,
    })
    ElMessage.success('归还成功')
    showReturnDialog.value = false
    loadRecords()
  } finally {
    actionLoading.value = false
  }
}

const handleMarkLost = (row: BorrowRecordInfo) => {
  currentRecordId.value = row.id
  lostForm.compensation_amount = 0
  lostForm.notes = ''
  showLostDialog.value = true
}

const confirmLost = async () => {
  if (!currentRecordId.value || lostForm.compensation_amount <= 0) {
    ElMessage.warning('请填写赔付金额（必须 > 0）')
    return
  }
  actionLoading.value = true
  try {
    await markLostColorCard(currentRecordId.value, {
      compensation_amount: lostForm.compensation_amount,
      notes: lostForm.notes || undefined,
    })
    ElMessage.success('已登记遗失')
    showLostDialog.value = false
    loadRecords()
  } finally {
    actionLoading.value = false
  }
}

const handleMarkDamaged = (row: BorrowRecordInfo) => {
  currentRecordId.value = row.id
  damagedForm.compensation_amount = 0
  damagedForm.notes = ''
  showDamagedDialog.value = true
}

const confirmDamaged = async () => {
  if (!currentRecordId.value) return
  actionLoading.value = true
  try {
    await markDamagedColorCard(currentRecordId.value, {
      compensation_amount: damagedForm.compensation_amount || undefined,
      notes: damagedForm.notes || undefined,
    })
    ElMessage.success('已标记损坏')
    showDamagedDialog.value = false
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
.color-card-borrow { padding: 16px; }
</style>

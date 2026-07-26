<!--
  PurchaseContractTab.vue - 采购合同 Tab
  来源：原 trading/index.vue 中 采购合同 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="purchase-contract-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('trading.purchaseContractTab.title') }}</h2>
      <el-button type="primary" @click="openPurchaseContractDialog()">
        <el-icon><Plus /></el-icon> {{ t('trading.purchaseContractTab.buttonCreate') }}
      </el-button>
    </div>
    <el-card shadow="hover">
      <el-table
        v-loading="purchaseContractLoading"
        :data="purchaseContracts"
        stripe
        :aria-label="t('trading.purchaseContractTab.tableAriaLabel')"
      >
        <el-table-column
          prop="contract_no"
          :label="t('trading.purchaseContractTab.columnContractNo')"
          width="140"
        />
        <el-table-column
          prop="supplier_name"
          :label="t('trading.purchaseContractTab.columnSupplier')"
          width="150"
        />
        <el-table-column
          prop="contract_date"
          :label="t('trading.purchaseContractTab.columnContractDate')"
          width="120"
        />
        <el-table-column
          prop="total_amount"
          :label="t('trading.purchaseContractTab.columnTotalAmount')"
          width="120"
          align="right"
        >
          <template #default="{ row }">{{ formatMoney(row.total_amount) }}</template>
        </el-table-column>
        <el-table-column
          prop="status"
          :label="t('trading.purchaseContractTab.columnStatus')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)" size="small">
              {{ getContractStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          :label="t('trading.purchaseContractTab.columnActions')"
          width="200"
          fixed="right"
        >
          <template #default="{ row }">
            <el-button
              type="primary"
              link
              size="small"
              @click="viewPurchaseContract(row as unknown as TradingContract)"
              >{{ t('trading.purchaseContractTab.buttonView') }}</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              type="success"
              link
              size="small"
              @click="approvePurchaseContract(row as unknown as TradingContract)"
              >{{ t('trading.purchaseContractTab.buttonApprove') }}</el-button
            >
            <el-button
              v-if="row.status === 'approved'"
              type="warning"
              link
              size="small"
              @click="executePurchaseContract(row as unknown as TradingContract)"
              >{{ t('trading.purchaseContractTab.buttonExecute') }}</el-button
            >
            <el-button
              type="danger"
              link
              size="small"
              @click="deletePurchaseContract(row as unknown as TradingContract)"
              >{{ t('trading.purchaseContractTab.buttonDelete') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  getTradingContractList,
  getTradingContract,
  createTradingContract,
  approveTradingContract,
  executeTradingContract,
  deleteTradingContract,
  type TradingContract,
} from '@/api/trading-contract'

const { t } = useI18n({ useScope: 'global' })

const purchaseContracts = ref<TradingContract[]>([])
const purchaseContractLoading = ref(false)

const formatMoney = (amount: number | undefined) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    draft: 'info',
    pending: 'warning',
    approved: 'primary',
    executed: 'success',
    completed: 'success',
    cancelled: 'danger',
  }
  return map[status] || 'info'
}

/** 采购合同状态 → i18n 标签（语言切换响应） */
const getContractStatusLabel = (status: string): string => {
  switch (status) {
    case 'draft':
      return t('trading.purchaseContractTab.statusDraft')
    case 'pending':
      return t('trading.purchaseContractTab.statusPending')
    case 'approved':
      return t('trading.purchaseContractTab.statusApproved')
    case 'executed':
      return t('trading.purchaseContractTab.statusExecuted')
    case 'completed':
      return t('trading.purchaseContractTab.statusCompleted')
    case 'cancelled':
      return t('trading.purchaseContractTab.statusCancelled')
    default:
      return status
  }
}

const fetchPurchaseContracts = async () => {
  purchaseContractLoading.value = true
  try {
    const res = await getTradingContractList({ type: 'purchase' })
    const d = res.data as
      | { list?: TradingContract[]; items?: TradingContract[] }
      | TradingContract[]
      | undefined
    if (d && typeof d === 'object' && !Array.isArray(d)) {
      purchaseContracts.value = d.list || d.items || []
    } else {
      purchaseContracts.value = (d as TradingContract[]) || []
    }
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || t('trading.purchaseContractTab.messageFetchFailed'))
  } finally {
    purchaseContractLoading.value = false
  }
}

const openPurchaseContractDialog = async () => {
  try {
    await createTradingContract({ type: 'purchase', status: 'draft' })
    ElMessage.success(t('trading.purchaseContractTab.messageCreateDraftSuccess'))
    fetchPurchaseContracts()
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || t('trading.purchaseContractTab.messageCreateFailed'))
  }
}

/** 构造合同详情多行文本（拆分以控制 viewPurchaseContract 行数） */
const buildContractDetailLines = (d: TradingContract): string[] => {
  return [
    t('trading.purchaseContractTab.detailContractNo', { value: d.contract_no }),
    t('trading.purchaseContractTab.detailSupplier', { value: d.supplier_name || '-' }),
    t('trading.purchaseContractTab.detailContractDate', { value: d.contract_date }),
    t('trading.purchaseContractTab.detailContractAmount', { value: formatMoney(d.total_amount) }),
    t('trading.purchaseContractTab.detailCurrentStatus', {
      value: getContractStatusLabel(d.status),
    }),
  ]
}

// 批次 157a P1-1 修复：接入 getTradingContract API 展示采购合同详情
const viewPurchaseContract = async (row: TradingContract) => {
  try {
    const res = await getTradingContract(row.id)
    const d = res.data
    if (!d) {
      ElMessage.warning(t('trading.purchaseContractTab.messageDetailNotFound'))
      return
    }
    const lines = buildContractDetailLines(d)
    await ElMessageBox.alert(lines.join('\n'), t('trading.purchaseContractTab.detailTitle'), {
      confirmButtonText: t('trading.purchaseContractTab.buttonClose'),
    })
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || t('trading.purchaseContractTab.messageFetchDetailFailed'))
  }
}

const approvePurchaseContract = async (row: TradingContract) => {
  try {
    await ElMessageBox.confirm(
      t('trading.purchaseContractTab.confirmApproveMessage'),
      t('trading.purchaseContractTab.confirmTitle'),
      { type: 'info' }
    )
    await approveTradingContract(row.id)
    ElMessage.success(t('trading.purchaseContractTab.messageApproveSuccess'))
    fetchPurchaseContracts()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || t('trading.purchaseContractTab.messageOperationFailed'))
    }
  }
}

const executePurchaseContract = async (row: TradingContract) => {
  try {
    await ElMessageBox.confirm(
      t('trading.purchaseContractTab.confirmExecuteMessage'),
      t('trading.purchaseContractTab.confirmTitle'),
      { type: 'info' }
    )
    await executeTradingContract(row.id)
    ElMessage.success(t('trading.purchaseContractTab.messageExecuteSuccess'))
    fetchPurchaseContracts()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || t('trading.purchaseContractTab.messageOperationFailed'))
    }
  }
}

const deletePurchaseContract = async (row: TradingContract) => {
  try {
    await ElMessageBox.confirm(
      t('trading.purchaseContractTab.confirmDeleteMessage'),
      t('trading.purchaseContractTab.confirmTitle'),
      { type: 'warning' }
    )
    await deleteTradingContract(row.id)
    ElMessage.success(t('trading.purchaseContractTab.messageDeleteSuccess'))
    fetchPurchaseContracts()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || t('trading.purchaseContractTab.messageOperationFailed'))
    }
  }
}

defineExpose({ refresh: fetchPurchaseContracts })

onMounted(() => {
  fetchPurchaseContracts()
})
</script>

<style scoped>
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

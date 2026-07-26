<!--
  SalesContractTab.vue - 销售合同 Tab
  来源：原 trading/index.vue 中 销售合同 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="sales-contract-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('trading.salesContractTab.title') }}</h2>
      <el-button type="primary" @click="openSalesContractDialog()">
        <el-icon><Plus /></el-icon> {{ t('trading.salesContractTab.buttonCreate') }}
      </el-button>
    </div>
    <el-card shadow="hover">
      <el-table
        v-loading="salesContractLoading"
        :data="salesContracts"
        stripe
        :aria-label="t('trading.salesContractTab.tableAriaLabel')"
      >
        <el-table-column
          prop="contract_no"
          :label="t('trading.salesContractTab.columnContractNo')"
          width="140"
        />
        <el-table-column
          prop="customer_name"
          :label="t('trading.salesContractTab.columnCustomer')"
          width="150"
        />
        <el-table-column
          prop="contract_date"
          :label="t('trading.salesContractTab.columnContractDate')"
          width="120"
        />
        <el-table-column
          prop="total_amount"
          :label="t('trading.salesContractTab.columnTotalAmount')"
          width="120"
          align="right"
        >
          <template #default="{ row }">{{ formatMoney(row.total_amount) }}</template>
        </el-table-column>
        <el-table-column
          prop="status"
          :label="t('trading.salesContractTab.columnStatus')"
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
          :label="t('trading.salesContractTab.columnActions')"
          width="200"
          fixed="right"
        >
          <template #default="{ row }">
            <el-button
              type="primary"
              link
              size="small"
              @click="viewSalesContract(row as unknown as TradingContract)"
              >{{ t('trading.salesContractTab.buttonView') }}</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              type="success"
              link
              size="small"
              @click="approveSalesContract(row as unknown as TradingContract)"
              >{{ t('trading.salesContractTab.buttonApprove') }}</el-button
            >
            <el-button
              v-if="row.status === 'approved'"
              type="warning"
              link
              size="small"
              @click="executeSalesContract(row as unknown as TradingContract)"
              >{{ t('trading.salesContractTab.buttonExecute') }}</el-button
            >
            <el-button
              type="danger"
              link
              size="small"
              @click="deleteSalesContract(row as unknown as TradingContract)"
              >{{ t('trading.salesContractTab.buttonDelete') }}</el-button
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

const salesContracts = ref<TradingContract[]>([])
const salesContractLoading = ref(false)

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

/** 销售合同状态 → i18n 标签（语言切换响应） */
const getContractStatusLabel = (status: string): string => {
  switch (status) {
    case 'draft':
      return t('trading.salesContractTab.statusDraft')
    case 'pending':
      return t('trading.salesContractTab.statusPending')
    case 'approved':
      return t('trading.salesContractTab.statusApproved')
    case 'executed':
      return t('trading.salesContractTab.statusExecuted')
    case 'completed':
      return t('trading.salesContractTab.statusCompleted')
    case 'cancelled':
      return t('trading.salesContractTab.statusCancelled')
    default:
      return status
  }
}

const fetchSalesContracts = async () => {
  salesContractLoading.value = true
  try {
    const res = await getTradingContractList({ type: 'sales' })
    const d = res.data as
      | { list?: TradingContract[]; items?: TradingContract[] }
      | TradingContract[]
      | undefined
    if (d && typeof d === 'object' && !Array.isArray(d)) {
      salesContracts.value = d.list || d.items || []
    } else {
      salesContracts.value = (d as TradingContract[]) || []
    }
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || t('trading.salesContractTab.messageFetchFailed'))
  } finally {
    salesContractLoading.value = false
  }
}

const openSalesContractDialog = async () => {
  try {
    await createTradingContract({ type: 'sales', status: 'draft' })
    ElMessage.success(t('trading.salesContractTab.messageCreateDraftSuccess'))
    fetchSalesContracts()
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || t('trading.salesContractTab.messageCreateFailed'))
  }
}

/** 构造销售合同详情多行文本（拆分以控制 viewSalesContract 行数） */
const buildContractDetailLines = (d: TradingContract): string[] => {
  return [
    t('trading.salesContractTab.detailContractNo', { value: d.contract_no }),
    t('trading.salesContractTab.detailCustomer', { value: d.customer_name || '-' }),
    t('trading.salesContractTab.detailContractDate', { value: d.contract_date }),
    t('trading.salesContractTab.detailContractAmount', { value: formatMoney(d.total_amount) }),
    t('trading.salesContractTab.detailCurrentStatus', {
      value: getContractStatusLabel(d.status),
    }),
  ]
}

// 批次 157a P1-1 修复：接入 getTradingContract API 展示销售合同详情
const viewSalesContract = async (row: TradingContract) => {
  try {
    const res = await getTradingContract(row.id)
    const d = res.data
    if (!d) {
      ElMessage.warning(t('trading.salesContractTab.messageDetailNotFound'))
      return
    }
    const lines = buildContractDetailLines(d)
    await ElMessageBox.alert(lines.join('\n'), t('trading.salesContractTab.detailTitle'), {
      confirmButtonText: t('trading.salesContractTab.buttonClose'),
    })
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || t('trading.salesContractTab.messageFetchDetailFailed'))
  }
}

const approveSalesContract = async (row: TradingContract) => {
  try {
    await ElMessageBox.confirm(
      t('trading.salesContractTab.confirmApproveMessage'),
      t('trading.salesContractTab.confirmTitle'),
      { type: 'info' }
    )
    await approveTradingContract(row.id)
    ElMessage.success(t('trading.salesContractTab.messageApproveSuccess'))
    fetchSalesContracts()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || t('trading.salesContractTab.messageOperationFailed'))
    }
  }
}

const executeSalesContract = async (row: TradingContract) => {
  try {
    await ElMessageBox.confirm(
      t('trading.salesContractTab.confirmExecuteMessage'),
      t('trading.salesContractTab.confirmTitle'),
      { type: 'info' }
    )
    await executeTradingContract(row.id)
    ElMessage.success(t('trading.salesContractTab.messageExecuteSuccess'))
    fetchSalesContracts()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || t('trading.salesContractTab.messageOperationFailed'))
    }
  }
}

const deleteSalesContract = async (row: TradingContract) => {
  try {
    await ElMessageBox.confirm(
      t('trading.salesContractTab.confirmDeleteMessage'),
      t('trading.salesContractTab.confirmTitle'),
      { type: 'warning' }
    )
    await deleteTradingContract(row.id)
    ElMessage.success(t('trading.salesContractTab.messageDeleteSuccess'))
    fetchSalesContracts()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || t('trading.salesContractTab.messageOperationFailed'))
    }
  }
}

defineExpose({ refresh: fetchSalesContracts })

onMounted(() => {
  fetchSalesContracts()
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

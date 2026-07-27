<!--
  SalesReturnTab.vue - 销售退货 Tab
  来源：原 trading/index.vue 中 销售退货 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="sales-return-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('trading.salesReturnTab.title') }}</h2>
      <el-button type="primary" @click="openSalesReturnDialog()">
        <el-icon><Plus /></el-icon> {{ t('trading.salesReturnTab.buttonCreate') }}
      </el-button>
    </div>
    <el-card shadow="hover">
      <el-table
        v-loading="salesReturnLoading"
        :data="salesReturns"
        stripe
        :aria-label="t('trading.salesReturnTab.tableAriaLabel')"
      >
        <el-table-column
          prop="return_no"
          :label="t('trading.salesReturnTab.columnReturnNo')"
          width="140"
        />
        <el-table-column
          prop="customer_name"
          :label="t('trading.salesReturnTab.columnCustomer')"
          width="150"
        />
        <el-table-column
          prop="return_date"
          :label="t('trading.salesReturnTab.columnReturnDate')"
          width="120"
        />
        <el-table-column
          prop="total_amount"
          :label="t('trading.salesReturnTab.columnAmount')"
          width="120"
          align="right"
        >
          <template #default="{ row }">{{ formatMoney(row.total_amount) }}</template>
        </el-table-column>
        <el-table-column
          prop="status"
          :label="t('trading.salesReturnTab.columnStatus')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)" size="small">
              {{ getReturnStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          :label="t('trading.salesReturnTab.columnActions')"
          width="180"
          fixed="right"
        >
          <template #default="{ row }">
            <el-button
              type="primary"
              link
              size="small"
              @click="viewSalesReturn(row as unknown as TradingReturn)"
              >{{ t('trading.salesReturnTab.buttonView') }}</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              type="success"
              link
              size="small"
              @click="approveSalesReturn(row as unknown as TradingReturn)"
              >{{ t('trading.salesReturnTab.buttonApprove') }}</el-button
            >
            <el-button
              type="danger"
              link
              size="small"
              @click="deleteSalesReturn(row as unknown as TradingReturn)"
              >{{ t('trading.salesReturnTab.buttonDelete') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import {
  getTradingReturnList,
  getTradingReturn,
  createTradingReturn,
  approveTradingReturn,
  deleteTradingReturn,
  type TradingReturn,
} from '@/api/trading-return';

const { t } = useI18n({ useScope: 'global' });

const salesReturns = ref<TradingReturn[]>([]);
const salesReturnLoading = ref(false);

const formatMoney = (amount: number | undefined) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00';
};

const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    draft: 'info',
    pending: 'warning',
    approved: 'primary',
    completed: 'success',
    cancelled: 'danger',
  };
  return map[status] || 'info';
};

/** 销售退货状态 → i18n 标签（语言切换响应） */
const getReturnStatusLabel = (status: string): string => {
  switch (status) {
    case 'draft':
      return t('trading.salesReturnTab.statusDraft');
    case 'pending':
      return t('trading.salesReturnTab.statusPending');
    case 'approved':
      return t('trading.salesReturnTab.statusApproved');
    case 'completed':
      return t('trading.salesReturnTab.statusCompleted');
    case 'cancelled':
      return t('trading.salesReturnTab.statusCancelled');
    default:
      return status;
  }
};

const fetchSalesReturns = async () => {
  salesReturnLoading.value = true;
  try {
    const res = await getTradingReturnList({ type: 'sales' });
    const d = res.data as
      | { list?: TradingReturn[]; items?: TradingReturn[] }
      | TradingReturn[]
      | undefined;
    if (d && typeof d === 'object' && !Array.isArray(d)) {
      salesReturns.value = d.list || d.items || [];
    } else {
      salesReturns.value = (d as TradingReturn[]) || [];
    }
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('trading.salesReturnTab.messageFetchFailed'));
  } finally {
    salesReturnLoading.value = false;
  }
};

const openSalesReturnDialog = async () => {
  try {
    await createTradingReturn({ type: 'sales', status: 'draft' });
    ElMessage.success(t('trading.salesReturnTab.messageCreateDraftSuccess'));
    fetchSalesReturns();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('trading.salesReturnTab.messageCreateFailed'));
  }
};

/** 构造销售退货详情多行文本（拆分以控制 viewSalesReturn 行数） */
const buildReturnDetailLines = (d: TradingReturn): string[] => {
  return [
    t('trading.salesReturnTab.detailReturnNo', { value: d.return_no }),
    t('trading.salesReturnTab.detailCustomer', { value: d.customer_name || '-' }),
    t('trading.salesReturnTab.detailOrderNo', { value: d.order_no || '-' }),
    t('trading.salesReturnTab.detailReturnDate', { value: d.return_date }),
    t('trading.salesReturnTab.detailReturnAmount', { value: formatMoney(d.total_amount) }),
    t('trading.salesReturnTab.detailCurrentStatus', {
      value: getReturnStatusLabel(d.status),
    }),
    t('trading.salesReturnTab.detailReason', { value: d.reason || '-' }),
  ];
};

// 批次 157a P1-1 修复：接入 getTradingReturn API 展示销售退货详情
const viewSalesReturn = async (row: TradingReturn) => {
  try {
    const res = await getTradingReturn(row.id);
    const d = res.data;
    if (!d) {
      ElMessage.warning(t('trading.salesReturnTab.messageDetailNotFound'));
      return;
    }
    const lines = buildReturnDetailLines(d);
    await ElMessageBox.alert(lines.join('\n'), t('trading.salesReturnTab.detailTitle'), {
      confirmButtonText: t('trading.salesReturnTab.buttonClose'),
    });
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('trading.salesReturnTab.messageFetchDetailFailed'));
  }
};

const approveSalesReturn = async (row: TradingReturn) => {
  try {
    await ElMessageBox.confirm(
      t('trading.salesReturnTab.confirmApproveMessage'),
      t('trading.salesReturnTab.confirmTitle'),
      { type: 'info' }
    );
    await approveTradingReturn(row.id);
    ElMessage.success(t('trading.salesReturnTab.messageApproveSuccess'));
    fetchSalesReturns();
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string };
      ElMessage.error(err.message || t('trading.salesReturnTab.messageOperationFailed'));
    }
  }
};

const deleteSalesReturn = async (row: TradingReturn) => {
  try {
    await ElMessageBox.confirm(
      t('trading.salesReturnTab.confirmDeleteMessage'),
      t('trading.salesReturnTab.confirmTitle'),
      { type: 'warning' }
    );
    await deleteTradingReturn(row.id);
    ElMessage.success(t('trading.salesReturnTab.messageDeleteSuccess'));
    fetchSalesReturns();
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string };
      ElMessage.error(err.message || t('trading.salesReturnTab.messageOperationFailed'));
    }
  }
};

defineExpose({ refresh: fetchSalesReturns });

onMounted(() => {
  fetchSalesReturns();
});
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

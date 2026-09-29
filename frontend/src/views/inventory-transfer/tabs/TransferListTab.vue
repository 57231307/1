<!--
  TransferListTab.vue - 库存调拨列表 Tab
  来源：原 inventoryTransfer/index.vue 中 列表/统计/过滤内容
  拆分日期：2026-06-15 B3-4
-->
<template>
  <div class="transfer-list">
    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon total-icon">
              <el-icon><Document /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">{{ t('inventoryTransfer.transferList.stat.total') }}</div>
              <div class="stat-value">{{ stats.total }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card warning">
          <div class="stat-content">
            <div class="stat-icon pending-icon">
              <el-icon><Clock /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">{{ t('inventoryTransfer.transferList.stat.pending') }}</div>
              <div class="stat-value">{{ stats.pending }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card success">
          <div class="stat-content">
            <div class="stat-icon approved-icon">
              <el-icon><CircleCheck /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">{{ t('inventoryTransfer.transferList.stat.approved') }}</div>
              <div class="stat-value">{{ stats.approved }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card highlight">
          <div class="stat-content">
            <div class="stat-icon amount-icon">
              <el-icon><Money /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">
                {{ t('inventoryTransfer.transferList.stat.totalAmount') }}
              </div>
              <div class="stat-value">{{ formatCurrency(stats.totalAmount) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" class="filter-card">
      <el-form
        :inline="true"
        :model="queryParams"
        class="filter-form"
        :aria-label="t('inventoryTransfer.transferList.filter.ariaLabel')"
      >
        <el-form-item :label="t('inventoryTransfer.transferList.filter.transferNo')">
          <el-input
            v-model="queryParams.transfer_no"
            :placeholder="t('inventoryTransfer.transferList.filter.transferNoPlaceholder')"
            clearable
          />
        </el-form-item>
        <el-form-item :label="t('inventoryTransfer.transferList.filter.status')">
          <el-select
            v-model="queryParams.status"
            :placeholder="t('inventoryTransfer.transferList.filter.statusPlaceholder')"
            clearable
          >
            <el-option
              v-for="s in INVENTORY_TRANSFER_STATUSES"
              :key="s"
              :label="t(inventoryTransferStatusLabelKey(s))"
              :value="s"
            />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">{{
            t('inventoryTransfer.transferList.button.query')
          }}</el-button>
          <el-button @click="handleReset">{{
            t('inventoryTransfer.transferList.button.reset')
          }}</el-button>
          <!-- P2-10 修复（批次 82 v1 复审）：补齐 v-permission 按钮权限 -->
          <el-button
            v-permission="'inventory:create'"
            type="primary"
            @click="emit('openForm', 'create', null)"
          >
            <el-icon><Plus /></el-icon>{{ t('inventoryTransfer.transferList.button.create') }}
          </el-button>
          <el-button plain @click="handleGenerateNo">
            {{ t('inventoryTransfer.transferList.button.generateNo') }}
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table
        v-loading="loading"
        :data="transfers"
        stripe
        :aria-label="t('inventoryTransfer.transferList.table.ariaLabel')"
      >
        <el-table-column
          prop="transfer_no"
          :label="t('inventoryTransfer.transferList.table.transferNo')"
          width="160"
          fixed
        />
        <el-table-column
          prop="transfer_date"
          :label="t('inventoryTransfer.transferList.table.transferDate')"
          width="120"
        />
        <el-table-column
          prop="from_warehouse_name"
          :label="t('inventoryTransfer.transferList.table.fromWarehouse')"
          width="120"
        />
        <el-table-column
          prop="to_warehouse_name"
          :label="t('inventoryTransfer.transferList.table.toWarehouse')"
          width="120"
        />
        <el-table-column
          prop="total_amount"
          :label="t('inventoryTransfer.transferList.table.amount')"
          width="120"
          align="right"
        >
          <template #default="{ row }">{{ formatCurrency(row.total_amount) }}</template>
        </el-table-column>
        <el-table-column
          prop="status"
          :label="t('inventoryTransfer.transferList.table.status')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="inventoryTransferStatusTagType(row.status)" size="small">
              {{ t(inventoryTransferStatusLabelKey(row.status)) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="created_by_name"
          :label="t('inventoryTransfer.transferList.table.createdBy')"
          width="100"
        />
        <el-table-column
          prop="created_at"
          :label="t('inventoryTransfer.transferList.table.createdAt')"
          width="160"
        />
        <el-table-column
          :label="t('inventoryTransfer.transferList.table.operation')"
          width="200"
          fixed="right"
        >
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="emit('openForm', 'view', row)">{{
              t('inventoryTransfer.transferList.button.detail')
            }}</el-button>
            <el-button
              v-if="row.status === INVENTORY_TRANSFER_STATUS.PENDING"
              type="primary"
              link
              size="small"
              @click="emit('openForm', 'edit', row)"
              >{{ t('inventoryTransfer.transferList.button.edit') }}</el-button
            >
            <el-button
              v-if="row.status === INVENTORY_TRANSFER_STATUS.PENDING"
              type="success"
              link
              size="small"
              @click="emit('openApprove', row)"
              >{{ t('inventoryTransfer.transferList.button.approve') }}</el-button
            >
            <el-button
              v-if="row.status === INVENTORY_TRANSFER_STATUS.APPROVED"
              type="warning"
              link
              size="small"
              @click="handleShip(row)"
              >{{ t('inventoryTransfer.transferList.button.ship') }}</el-button
            >
            <el-button
              v-if="row.status === INVENTORY_TRANSFER_STATUS.PENDING"
              type="danger"
              link
              size="small"
              @click="emit('delete', row)"
              >{{ t('inventoryTransfer.transferList.button.delete') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <!-- 批次 391：分页由 useTableApi watch 自动加载，v-model 双向绑定 page/pageSize -->
      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          :aria-label="t('inventoryTransfer.transferList.table.paginationAriaLabel')"
        />
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { reactive, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Document, Clock, CircleCheck, Money, Plus } from '@element-plus/icons-vue';
import { executeInventoryTransfer } from '@/api/inventory';
import {
  generateInventoryTransferNo,
  type InventoryTransferEntity,
} from '@/api/inventory-transfer';
import { useTableApi } from '@/composables/useTableApi';
import { logger } from '@/utils/logger';
import { formatCurrency } from '@/utils';
import {
  INVENTORY_TRANSFER_STATUS,
  INVENTORY_TRANSFER_STATUSES,
  inventoryTransferStatusLabelKey,
  inventoryTransferStatusTagType,
} from '@/utils/inventory-transfer-status';

// 批次 34 v9 P1：接入 i18n，替换硬编码中文 ElMessage
const { t } = useI18n({ useScope: 'global' });

const emit = defineEmits<{
  openForm: [mode: 'create' | 'edit' | 'view', row: InventoryTransferEntity | null];
  openApprove: [row: InventoryTransferEntity];
  delete: [row: InventoryTransferEntity];
}>();

// 批次 391：接入 useTableApi，统一分页规范（1-based），由 setup 自动加载 + watch page/pageSize 触发。
// 后端 list_transfers 返回裸数组 ApiResponse<Vec<Value>>：useTableApi 把 res.data 数组挂到 payload.data，
// 故显式钉住 listKey='data'，不依赖内部 list/items/data 探测（total 后端未返回，恒为 0）。
const {
  data: transfers,
  total,
  loading,
  page,
  pageSize,
  queryParams,
  refresh: fetchTransfers,
} = useTableApi<InventoryTransferEntity>({
  url: '/inventory/transfers',
  listKey: 'data',
  defaultPageSize: 20,
  defaultParams: {
    transfer_no: '',
    status: '',
  },
  onError: (err: unknown) => {
    logger.error(t('inventoryTransfer.transferList.message.loadListFailed'), err);
    ElMessage.error(t('message.loadFailed'));
  },
});

// stats 保留原语义：total 为后端总记录数，pending/approved/totalAmount 基于当前页数据计算。
// watch data 变化自动更新 stats，无需在每次 refresh 后手动赋值。
const stats = reactive({
  total: 0,
  pending: 0,
  approved: 0,
  totalAmount: 0,
});

watch(
  transfers,
  newData => {
    stats.total = total.value;
    stats.pending = newData.filter(
      item => item.status === INVENTORY_TRANSFER_STATUS.PENDING
    ).length;
    stats.approved = newData.filter(
      item => item.status === INVENTORY_TRANSFER_STATUS.APPROVED
    ).length;
    stats.totalAmount = newData.reduce((sum, item) => sum + Number(item.total_amount), 0);
  },
  { immediate: true }
);

// handleQuery 同步搜索表单到 useTableApi queryParams 后重置到第 1 页并加载。
// useTableApi watch 只监听 page/pageSize，不监听 queryParams，所以修改后需手动调 refresh。
const handleQuery = () => {
  page.value = 1;
  fetchTransfers();
};
const handleReset = () => {
  queryParams.value = {
    transfer_no: '',
    status: '',
  };
  handleQuery();
};

// 保留父组件调用接口：expose refresh 代替原 fetchTransfers
defineExpose({ fetchTransfers });

// 发货（executeInventoryTransfer：approved → shipped，按调拨明细扣减/增加在途库存）
const handleShip = async (row: InventoryTransferEntity) => {
  try {
    await ElMessageBox.confirm(
      t('inventoryTransfer.transferList.message.shipConfirm'),
      t('inventoryTransfer.transferList.message.shipTitle'),
      { type: 'warning' }
    );
  } catch {
    return;
  }
  try {
    await executeInventoryTransfer(row.id as number);
    ElMessage.success(t('inventoryTransfer.transferList.message.shipSuccess'));
    await fetchTransfers();
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error(
        (error as Error).message || t('inventoryTransfer.transferList.message.failure')
      );
    }
  }
};

// 取号：预生成调拨单号（供线下单据核对/登记使用）
const handleGenerateNo = async () => {
  try {
    const res = await generateInventoryTransferNo();
    const no =
      (res.data as { transfer_no?: string })?.transfer_no ??
      (res as { transfer_no?: string })?.transfer_no;
    ElMessageBox.alert(no || '-', t('inventoryTransfer.transferList.message.generateNoTitle'));
  } catch (error) {
    ElMessage.error(
      (error as Error).message || t('inventoryTransfer.transferList.message.failure')
    );
  }
};
</script>

<style scoped>
.stats-row {
  margin-bottom: 20px;
}
.stat-card {
  border-radius: 12px;
  transition: all 0.3s;
}
.stat-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
}
.stat-card.warning {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
}
.stat-card.warning :deep(.stat-icon) {
  background: rgba(255, 255, 255, 0.2);
}
.stat-card.success {
  background: linear-gradient(135deg, #43e97b 0%, #38f9d7 100%);
}
.stat-card.success :deep(.stat-icon) {
  background: rgba(255, 255, 255, 0.2);
}
.stat-card.highlight {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
}
.stat-card.highlight :deep(.stat-icon) {
  background: rgba(255, 255, 255, 0.2);
}
.stat-card.warning :deep(.stat-label),
.stat-card.warning :deep(.stat-value),
.stat-card.success :deep(.stat-label),
.stat-card.success :deep(.stat-value),
.stat-card.highlight :deep(.stat-label),
.stat-card.highlight :deep(.stat-value) {
  color: white;
}
:deep(.stat-content) {
  display: flex;
  align-items: center;
  gap: 16px;
}
:deep(.stat-icon) {
  width: 56px;
  height: 56px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 28px;
  color: white;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}
:deep(.stat-icon.total-icon) {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
}
:deep(.stat-icon.pending-icon),
:deep(.stat-icon.approved-icon),
:deep(.stat-icon.amount-icon) {
  background: rgba(255, 255, 255, 0.2);
}
:deep(.stat-info) {
  flex: 1;
}
:deep(.stat-label) {
  font-size: 14px;
  color: #909399;
  margin-bottom: 4px;
}
:deep(.stat-value) {
  font-size: 28px;
  font-weight: 700;
  color: #303133;
  line-height: 1.2;
}
.filter-card {
  margin-bottom: 20px;
}
.table-card {
  margin-bottom: 20px;
}
.pagination-wrapper {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}
</style>

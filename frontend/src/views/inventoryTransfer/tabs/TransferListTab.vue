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
              <div class="stat-label">调拨单数</div>
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
              <div class="stat-label">待审批</div>
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
              <div class="stat-label">已审批</div>
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
              <div class="stat-label">总调拨金额</div>
              <div class="stat-value">{{ formatCurrency(stats.totalAmount) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form">
        <el-form-item label="调拨单号">
          <el-input v-model="queryParams.transfer_no" placeholder="输入单号" clearable />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="queryParams.status" placeholder="选择状态" clearable>
            <el-option label="待审批" value="pending" />
            <el-option label="已审批" value="approved" />
            <el-option label="已执行" value="executed" />
            <el-option label="已取消" value="cancelled" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">查询</el-button>
          <el-button @click="handleReset">重置</el-button>
          <!-- P2-10 修复（批次 82 v1 复审）：补齐 v-permission 按钮权限 -->
          <el-button v-permission="'inventory:create'" type="primary" @click="emit('openForm', 'create', null)">
            <el-icon><Plus /></el-icon>新建
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table v-loading="loading" :data="transfers" stripe>
        <el-table-column prop="transfer_no" label="调拨单号" width="160" fixed />
        <el-table-column prop="transfer_date" label="调拨日期" width="120" />
        <el-table-column prop="from_warehouse_name" label="调出仓库" width="120" />
        <el-table-column prop="to_warehouse_name" label="调入仓库" width="120" />
        <el-table-column prop="total_amount" label="金额" width="120" align="right">
          <template #default="{ row }">{{ formatCurrency(row.total_amount) }}</template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)" size="small">
              {{ getStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_by_name" label="创建人" width="100" />
        <el-table-column prop="created_at" label="创建时间" width="160" />
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="emit('openForm', 'view', row)"
              >详情</el-button
            >
            <el-button
              v-if="row.status === 'pending'"
              type="primary"
              link
              size="small"
              @click="emit('openForm', 'edit', row)"
              >编辑</el-button
            >
            <el-button
              v-if="row.status === 'pending'"
              type="success"
              link
              size="small"
              @click="emit('openApprove', row)"
              >审批</el-button
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
        />
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { reactive, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { Document, Clock, CircleCheck, Money, Plus } from '@element-plus/icons-vue'
import { type InventoryTransferEntity } from '@/api/inventoryTransfer'
import { useTableApi } from '@/composables/useTableApi'
import { logger } from '@/utils/logger'

// 批次 34 v9 P1：接入 i18n，替换硬编码中文 ElMessage
const { t } = useI18n({ useScope: 'global' })

const emit = defineEmits<{
  openForm: [mode: 'create' | 'edit' | 'view', row: InventoryTransferEntity | null]
  openApprove: [row: InventoryTransferEntity]
}>()

// 批次 391：接入 useTableApi，统一分页规范（1-based），由 setup 自动加载 + watch page/pageSize 触发。
// 后端返回 { data: { list: [], total: 0 } }，listKey 默认 'list' 命中自动探测。
// 原手写 queryParams/transfers/loading/total/fetchTransfers 模板代码消除。
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
  defaultPageSize: 20,
  defaultParams: {
    transfer_no: '',
    status: '',
  },
  onError: (err: unknown) => {
    logger.error('获取库存调拨单失败', err)
    ElMessage.error(t('message.loadFailed'))
  },
})

// stats 保留原语义：total 为后端总记录数，pending/approved/totalAmount 基于当前页数据计算。
// watch data 变化自动更新 stats，无需在每次 refresh 后手动赋值。
const stats = reactive({
  total: 0,
  pending: 0,
  approved: 0,
  totalAmount: 0,
})

watch(
  transfers,
  newData => {
    stats.total = total.value
    stats.pending = newData.filter(item => item.status === 'pending').length
    stats.approved = newData.filter(item => item.status === 'approved').length
    stats.totalAmount = newData.reduce((sum, item) => sum + (item.total_amount || 0), 0)
  },
  { immediate: true }
)

const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    pending: '待审批',
    approved: '已审批',
    executed: '已执行',
    cancelled: '已取消',
  }
  return map[status] || status
}
const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    pending: 'warning',
    approved: 'success',
    executed: 'primary',
    cancelled: 'info',
  }
  return map[status] || 'info'
}
const formatCurrency = (amount: number) => `¥${(amount || 0).toFixed(2)}`

// handleQuery 同步搜索表单到 useTableApi queryParams 后重置到第 1 页并加载。
// useTableApi watch 只监听 page/pageSize，不监听 queryParams，所以修改后需手动调 refresh。
const handleQuery = () => {
  page.value = 1
  fetchTransfers()
}
const handleReset = () => {
  queryParams.value = {
    transfer_no: '',
    status: '',
  }
  handleQuery()
}

// 保留父组件调用接口：expose refresh 代替原 fetchTransfers
defineExpose({ fetchTransfers })
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

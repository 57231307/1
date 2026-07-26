<!--
  customer/index.vue - 客户管理主入口
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-3）：
  原 551 行"上帝组件"已拆分为：
  - tabs/CustomerFormTab.vue - 新建/编辑客户对话框

  本主入口承担：页面布局 + 列表数据 + 公共样式。
-->
<template>
  <div class="customer-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">{{ $t('customer.index.title') }}</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">{{ $t('customer.index.breadcrumb.home') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ $t('customer.index.breadcrumb.basicData') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ $t('customer.index.breadcrumb.customer') }}</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <!-- P2-10 修复（批次 82 v1 复审）：补齐 v-permission 按钮权限 -->
        <el-button v-permission="PERMISSIONS.CUSTOMER_CREATE" type="primary" @click="openCreateDialog">
          <el-icon><Plus /></el-icon>
          {{ $t('customer.index.button.create') }}
        </el-button>
        <el-button @click="handlePrint">
          <el-icon><Printer /></el-icon>
          {{ $t('customer.index.button.print') }}
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          {{ $t('customer.index.button.export') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form" :aria-label="$t('customer.index.filter.ariaLabel')">
        <el-form-item :label="$t('customer.index.filter.keyword')">
          <el-input v-model="queryParams.keyword" :placeholder="$t('customer.index.filter.keywordPlaceholder')" clearable />
        </el-form-item>
        <el-form-item :label="$t('customer.index.filter.customerType')">
          <el-select v-model="queryParams.customer_type" :placeholder="$t('customer.index.filter.customerTypePlaceholder')" clearable>
            <el-option :label="$t('customer.index.filterOption.typeNormal')" value="normal" />
            <el-option :label="$t('customer.index.filterOption.typeVip')" value="vip" />
            <el-option :label="$t('customer.index.filterOption.typeWholesale')" value="wholesale" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('customer.index.filter.status')">
          <el-select v-model="queryParams.status" :placeholder="$t('customer.index.filter.statusPlaceholder')" clearable>
            <el-option :label="$t('customer.index.filterOption.statusActive')" value="active" />
            <el-option :label="$t('customer.index.filterOption.statusInactive')" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">{{ $t('customer.index.button.query') }}</el-button>
          <el-button @click="handleReset">{{ $t('customer.index.button.reset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table v-loading="loading" :data="customers" stripe :aria-label="$t('customer.index.table.ariaLabel')">
        <el-table-column prop="customer_code" :label="$t('customer.index.table.column.customerCode')" width="120" fixed />
        <el-table-column prop="customer_name" :label="$t('customer.index.table.column.customerName')" min-width="180" fixed />
        <el-table-column prop="contact_person" :label="$t('customer.index.table.column.contactPerson')" width="100" />
        <el-table-column prop="contact_phone" :label="$t('customer.index.table.column.phone')" width="130" />
        <el-table-column prop="contact_email" :label="$t('customer.index.table.column.email')" width="180" show-overflow-tooltip />
        <el-table-column prop="customer_type" :label="$t('customer.index.table.column.type')" width="100">
          <template #default="{ row }">
            <el-tag :type="getCustomerTypeTag(row.customer_type)" size="small">
              {{ getCustomerTypeLabel(row.customer_type) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="province" :label="$t('customer.index.table.column.province')" width="100" />
        <el-table-column prop="credit_limit" :label="$t('customer.index.table.column.creditLimit')" width="120" align="right">
          <template #default="{ row }">
            {{ row.credit_limit ? formatCurrency(row.credit_limit) : '-' }}
          </template>
        </el-table-column>
        <el-table-column prop="payment_terms" :label="$t('customer.index.table.column.paymentTerms')" width="90" align="center" />
        <el-table-column prop="status" :label="$t('customer.index.table.column.status')" width="80">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ row.status === 'active' ? $t('customer.index.statusLabel.active') : $t('customer.index.statusLabel.inactive') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="$t('customer.index.table.column.action')" width="180" fixed="right">
          <template #default="{ row }">
            <!-- P3 维度 10 修复（批次 87）：编辑/删除按钮补齐 v-permission -->
            <el-button
              v-permission="PERMISSIONS.CUSTOMER_UPDATE"
              type="primary"
              link
              size="small"
              @click="openEditDialog(row)"
              >{{ $t('customer.index.button.edit') }}</el-button
            >
            <el-button
              v-permission="PERMISSIONS.CUSTOMER_DELETE"
              type="danger"
              link
              size="small"
              @click="handleDelete(row)"
              >{{ $t('customer.index.button.delete') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          :aria-label="$t('customer.index.table.paginationAria')"
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
        />
      </div>
    </el-card>

    <CustomerFormTab
      v-model="formDialogVisible"
      :title="formDialogTitle"
      :row-data="currentRow"
      @submitted="handleFormSubmitted"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Download, Printer } from '@element-plus/icons-vue'
import { deleteCustomer, type Customer } from '@/api/customer'
// V15 P0-S12 + P0-S15 修复（Batch 474）：客户导出改用后端带水印 xlsx 接口
// 保留 exportData 仅用于兼容场景（本视图已切换为 exportFromBackend）
import { exportFromBackend } from '@/utils/export'
import { printData } from '@/utils/print'
import { logger } from '@/utils/logger'
import { useTableApi } from '@/composables/useTableApi'
// Batch 468 P0-S28：引入权限码常量，与后端 customers 资源对齐
import { PERMISSIONS } from '@/constants/permissions'
import CustomerFormTab from './tabs/CustomerFormTab.vue'

const { t } = useI18n({ useScope: 'global' })

const formDialogVisible = ref(false)
const formDialogTitle = ref(t('customer.index.dialog.createTitle'))
const currentRow = ref<Customer | null>(null)

const queryParams = reactive({
  keyword: '',
  customer_type: '',
  status: '',
})

// 批次 276：接入 useTableApi，消除手写 customers/total/loading/fetchData 重复
// useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
const {
  data: customers,
  loading,
  page,
  pageSize,
  total,
  refresh: fetchData,
  setQueryParam,
} = useTableApi<Customer>({
  url: '/crm/customers',
  onError: (err: unknown) =>
    ElMessage.error((err instanceof Error ? err.message : String(err)) || t('customer.index.message.fetchListFailed')),
})

// 批次 276：同步筛选条件到 useTableApi.queryParams 并刷新
const syncQueryParams = () => {
  setQueryParam('keyword', queryParams.keyword || undefined)
  setQueryParam('customer_type', queryParams.customer_type || undefined)
  setQueryParam('status', queryParams.status || undefined)
}

const handleQuery = () => {
  syncQueryParams()
  page.value = 1
  fetchData()
}

const handleReset = () => {
  queryParams.keyword = ''
  queryParams.customer_type = ''
  queryParams.status = ''
  syncQueryParams()
  page.value = 1
  fetchData()
}

// 分页（useTableApi 自动 watch page/pageSize 变化触发重载）
const handlePageChange = (p: number) => {
  page.value = p
}

const handleSizeChange = (s: number) => {
  pageSize.value = s
  page.value = 1
}

const formatCurrency = (amount: number) => `¥${(amount || 0).toFixed(2)}`

const getCustomerTypeLabel = (type: string) => {
  const labelMap: Record<string, string> = {
    retail: t('customer.index.typeLabel.retail'),
    vip: t('customer.index.typeLabel.vip'),
    wholesale: t('customer.index.typeLabel.wholesale'),
  }
  return labelMap[type] || type
}

const getCustomerTypeTag = (type: string) => {
  const typeMap: Record<string, string> = {
    retail: '',
    vip: 'warning',
    wholesale: 'success',
  }
  return typeMap[type] || ''
}

const openCreateDialog = () => {
  currentRow.value = null
  formDialogTitle.value = t('customer.index.dialog.createTitle')
  formDialogVisible.value = true
}

const openEditDialog = (row: Customer) => {
  currentRow.value = row
  formDialogTitle.value = t('customer.index.dialog.editTitle')
  formDialogVisible.value = true
}

const handleFormSubmitted = () => {
  formDialogVisible.value = false
  fetchData()
}

const handleDelete = async (row: Customer) => {
  try {
    await ElMessageBox.confirm(
      t('customer.index.dialog.deleteConfirmMessage', { name: row.customer_name }),
      t('customer.index.dialog.deleteConfirmTitle'),
      {
        type: 'warning',
      }
    )
    await deleteCustomer(row.id)
    ElMessage.success(t('customer.index.message.deleteSuccess'))
    fetchData()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || t('customer.index.message.deleteFailed'))
    }
  }
}

const handleExport = async () => {
  // V15 P0-S12 + P0-S15 修复（Batch 474）：改用后端带水印 xlsx 接口
  // - 后端 GET /crm/customers/export 已注入水印（操作员/导出时间/导出条数）
  // - 水印在 xlsx 第 0 行（合并所有列），标题行下移到第 1 行，数据行从第 2 行起
  // - 行级数据权限与 list 一致（后端复用 list_customers_with_filter + to_data_scope_context）
  // - 异步记录审计日志（OperationType::Export）
  // queryParams 字段与 CustomerQueryParams 对齐（keyword/customer_type/status）
  // 空字符串改为 undefined 避免后端按空字符串过滤
  const params: Record<string, unknown> = {
    keyword: queryParams.keyword || undefined,
    customer_type: queryParams.customer_type || undefined,
    status: queryParams.status || undefined,
  }
  await exportFromBackend('/crm/customers/export', params, 'customers_export')
}

const handlePrint = () => {
  printData({
    title: t('customer.index.print.title'),
    columns: [
      { key: 'customer_code', title: t('customer.index.table.column.customerCode'), width: '100px' },
      { key: 'customer_name', title: t('customer.index.table.column.customerName') },
      { key: 'contact_person', title: t('customer.index.table.column.contactPerson'), width: '80px' },
      { key: 'contact_phone', title: t('customer.index.table.column.phone'), width: '120px' },
      {
        key: 'customer_type',
        title: t('customer.index.table.column.type'),
        width: '80px',
        formatter: v => getCustomerTypeLabel(String(v)),
      },
      {
        key: 'status',
        title: t('customer.index.table.column.status'),
        width: '60px',
        formatter: v => (v === 'active' ? t('customer.index.statusLabel.active') : t('customer.index.statusLabel.inactive')),
      },
    ],
    data: customers.value as unknown as Record<string, unknown>[],
  })
  logger.info(t('customer.index.print.logGenerated'))
}
</script>

<style scoped>
.customer-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 24px;
}
.header-left .page-title {
  font-size: 28px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 12px 0;
}
.header-actions {
  display: flex;
  gap: 12px;
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

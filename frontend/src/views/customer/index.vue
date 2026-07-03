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
        <h1 class="page-title">客户管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>基础数据</el-breadcrumb-item>
          <el-breadcrumb-item>客户管理</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <!-- P2-10 修复（批次 82 v1 复审）：补齐 v-permission 按钮权限 -->
        <el-button v-permission="'customers:create'" type="primary" @click="openCreateDialog">
          <el-icon><Plus /></el-icon>
          新建客户
        </el-button>
        <el-button @click="handlePrint">
          <el-icon><Printer /></el-icon>
          打印
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          导出
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form">
        <el-form-item label="关键词">
          <el-input v-model="queryParams.keyword" placeholder="客户编码/名称/联系人" clearable />
        </el-form-item>
        <el-form-item label="客户类型">
          <el-select v-model="queryParams.customer_type" placeholder="选择类型" clearable>
            <el-option label="普通客户" value="normal" />
            <el-option label="VIP客户" value="vip" />
            <el-option label="批发客户" value="wholesale" />
          </el-select>
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="queryParams.status" placeholder="选择状态" clearable>
            <el-option label="启用" value="active" />
            <el-option label="禁用" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">查询</el-button>
          <el-button @click="handleReset">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table v-loading="loading" :data="customers" stripe>
        <el-table-column prop="customer_code" label="客户编码" width="120" fixed />
        <el-table-column prop="customer_name" label="客户名称" min-width="180" fixed />
        <el-table-column prop="contact_person" label="联系人" width="100" />
        <el-table-column prop="contact_phone" label="电话" width="130" />
        <el-table-column prop="contact_email" label="邮箱" width="180" show-overflow-tooltip />
        <el-table-column prop="customer_type" label="类型" width="100">
          <template #default="{ row }">
            <el-tag :type="getCustomerTypeTag(row.customer_type)" size="small">
              {{ getCustomerTypeLabel(row.customer_type) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="province" label="省份" width="100" />
        <el-table-column prop="credit_limit" label="信用额度" width="120" align="right">
          <template #default="{ row }">
            {{ row.credit_limit ? formatCurrency(row.credit_limit) : '-' }}
          </template>
        </el-table-column>
        <el-table-column prop="payment_terms" label="账期(天)" width="90" align="center" />
        <el-table-column prop="status" label="状态" width="80">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ row.status === 'active' ? '启用' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="openEditDialog(row)"
              >编辑</el-button
            >
            <el-button type="danger" link size="small" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="queryParams.page"
          v-model:page-size="queryParams.page_size"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="handleQuery"
          @current-change="handleQuery"
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
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Download, Printer } from '@element-plus/icons-vue'
import { customerApi, type Customer } from '@/api/customer'
import { exportData } from '@/utils/export'
import { printData } from '@/utils/print'
import { logger } from '@/utils/logger'
import CustomerFormTab from './tabs/CustomerFormTab.vue'

const loading = ref(false)
const customers = ref<Customer[]>([])
const total = ref(0)

const formDialogVisible = ref(false)
const formDialogTitle = ref('新建客户')
const currentRow = ref<Customer | null>(null)

const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  customer_type: '',
  status: '',
})

const formatCurrency = (amount: number) => `¥${(amount || 0).toFixed(2)}`

const getCustomerTypeLabel = (type: string) => {
  const labelMap: Record<string, string> = {
    retail: '零售',
    vip: 'VIP',
    wholesale: '批发',
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

const fetchData = async () => {
  loading.value = true
  try {
    const res = await customerApi.list(queryParams)
    customers.value = res.data?.list || []
    total.value = res.data?.total || 0
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '获取客户列表失败')
    customers.value = []
    total.value = 0
  } finally {
    loading.value = false
  }
}

const handleQuery = () => {
  queryParams.page = 1
  fetchData()
}

const handleReset = () => {
  queryParams.keyword = ''
  queryParams.customer_type = ''
  queryParams.status = ''
  handleQuery()
}

const openCreateDialog = () => {
  currentRow.value = null
  formDialogTitle.value = '新建客户'
  formDialogVisible.value = true
}

const openEditDialog = (row: Customer) => {
  currentRow.value = row
  formDialogTitle.value = '编辑客户'
  formDialogVisible.value = true
}

const handleFormSubmitted = () => {
  formDialogVisible.value = false
  fetchData()
}

const handleDelete = async (row: Customer) => {
  try {
    await ElMessageBox.confirm(`确定删除客户 "${row.customer_name}" 吗？`, '删除确认', {
      type: 'warning',
    })
    await customerApi.delete(row.id)
    ElMessage.success('删除成功')
    fetchData()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || '删除失败')
    }
  }
}

const handleExport = () => {
  exportData({
    filename: '客户列表',
    columns: [
      { key: 'customer_code', title: '客户编码' },
      { key: 'customer_name', title: '客户名称' },
      { key: 'contact_person', title: '联系人' },
      { key: 'contact_phone', title: '电话' },
      { key: 'contact_email', title: '邮箱' },
      { key: 'customer_type', title: '类型', formatter: v => getCustomerTypeLabel(String(v)) },
      { key: 'province', title: '省份' },
      {
        key: 'credit_limit',
        title: '信用额度',
        formatter: v => (v ? formatCurrency(Number(v)) : '-'),
      },
      { key: 'status', title: '状态', formatter: v => (v === 'active' ? '启用' : '禁用') },
    ],
    data: customers.value,
  })
}

const handlePrint = () => {
  printData({
    title: '客户列表',
    columns: [
      { key: 'customer_code', title: '客户编码', width: '100px' },
      { key: 'customer_name', title: '客户名称' },
      { key: 'contact_person', title: '联系人', width: '80px' },
      { key: 'contact_phone', title: '电话', width: '120px' },
      {
        key: 'customer_type',
        title: '类型',
        width: '80px',
        formatter: v => getCustomerTypeLabel(String(v)),
      },
      {
        key: 'status',
        title: '状态',
        width: '60px',
        formatter: v => (v === 'active' ? '启用' : '禁用'),
      },
    ],
    data: customers.value,
  })
  logger.info('客户列表打印任务已生成')
}

onMounted(() => {
  fetchData()
})
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

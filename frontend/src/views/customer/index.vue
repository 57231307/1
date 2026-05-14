<template>
  <div class="customer-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">客户管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>客户管理</el-breadcrumb-item>
          <el-breadcrumb-item>客户列表</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          新建客户
        </el-button>
      </div>
    </div>

    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon total-icon">
              <el-icon><User /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">客户总数</div>
              <div class="stat-value">{{ stats.totalCustomers }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card highlight">
          <div class="stat-content">
            <div class="stat-icon active-icon">
              <el-icon><UserFilled /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">活跃客户</div>
              <div class="stat-value">{{ stats.activeCustomers }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon amount-icon">
              <el-icon><Money /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">本月销售额</div>
              <div class="stat-value">{{ formatCurrency(stats.monthSales) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card warning">
          <div class="stat-content">
            <div class="stat-icon credit-icon">
              <el-icon><CreditCard /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">应收余额</div>
              <div class="stat-value">{{ formatCurrency(stats.arBalance) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form">
        <el-form-item label="关键词">
          <el-input v-model="queryParams.keyword" placeholder="客户编码/名称/联系人" clearable @clear="handleQuery" />
        </el-form-item>
        <el-form-item label="客户类型">
          <el-select v-model="queryParams.customer_type" placeholder="选择类型" clearable @change="handleQuery">
            <el-option label="终端客户" value="end_user" />
            <el-option label="经销商" value="dealer" />
            <el-option label="代理商" value="agent" />
          </el-select>
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="queryParams.status" placeholder="选择状态" clearable @change="handleQuery">
            <el-option label="正常" value="active" />
            <el-option label="禁用" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">
            <el-icon><Search /></el-icon>
            查询
          </el-button>
          <el-button @click="handleReset">
            <el-icon><Refresh /></el-icon>
            重置
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table v-loading="loading" :data="customers" stripe>
        <el-table-column prop="customer_code" label="客户编码" width="140" fixed />
        <el-table-column prop="customer_name" label="客户名称" min-width="180" fixed />
        <el-table-column prop="customer_type" label="客户类型" width="100">
          <template #default="{ row }">
            <el-tag :type="getCustomerTypeColor(row.customer_type)" size="small">
              {{ getCustomerTypeText(row.customer_type) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="contact_person" label="联系人" width="100" />
        <el-table-column prop="phone" label="联系电话" width="120" />
        <el-table-column prop="credit_limit" label="信用额度" width="120" align="right">
          <template #default="{ row }">
            <span>¥{{ (row.credit_limit || 0).toLocaleString() }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="current_balance" label="当前余额" width="120" align="right">
          <template #default="{ row }">
            <span :class="{ 'debt': (row.current_balance || 0) > 0 }">
              ¥{{ (row.current_balance || 0).toLocaleString() }}
            </span>
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="80">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ row.status === 'active' ? '正常' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleView(row)">详情</el-button>
            <el-button type="primary" link size="small" @click="handleEdit(row)">编辑</el-button>
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
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Search, Refresh, User, UserFilled, Money, CreditCard } from '@element-plus/icons-vue'

const loading = ref(false)
const customers = ref<any[]>([])
const total = ref(0)

const stats = ref({
  totalCustomers: 45,
  activeCustomers: 38,
  monthSales: 1250000,
  arBalance: 580000
})

const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  customer_type: '',
  status: ''
})

const formatCurrency = (amount: number) => {
  return new Intl.NumberFormat('zh-CN', { style: 'currency', currency: 'CNY', minimumFractionDigits: 0 }).format(amount)
}

const getCustomerTypeColor = (type: string) => {
  const typeMap: Record<string, any> = { end_user: 'success', dealer: 'warning', agent: 'primary' }
  return typeMap[type] || 'info'
}

const getCustomerTypeText = (type: string) => {
  const textMap: Record<string, string> = { end_user: '终端客户', dealer: '经销商', agent: '代理商' }
  return textMap[type] || type
}

const fetchData = async () => {
  loading.value = true
  try {
    customers.value = [
      {
        id: 1,
        customer_code: 'C001',
        customer_name: '纺织公司A',
        customer_type: 'end_user',
        contact_person: '张经理',
        phone: '13800138000',
        credit_limit: 500000,
        current_balance: 80000,
        status: 'active'
      },
      {
        id: 2,
        customer_code: 'C002',
        customer_name: '服装厂B',
        customer_type: 'dealer',
        contact_person: '李总',
        phone: '13900139000',
        credit_limit: 1000000,
        current_balance: 150000,
        status: 'active'
      },
      {
        id: 3,
        customer_code: 'C003',
        customer_name: '面料贸易商C',
        customer_type: 'agent',
        contact_person: '王经理',
        phone: '13700137000',
        credit_limit: 300000,
        current_balance: 0,
        status: 'active'
      }
    ]
    total.value = 3
    ElMessage.info('使用演示数据')
  } finally {
    loading.value = false
  }
}

const handleQuery = () => { queryParams.page = 1; fetchData() }
const handleReset = () => { queryParams.keyword = ''; queryParams.customer_type = ''; queryParams.status = ''; handleQuery() }
const handleCreate = () => { ElMessage.info('新建客户功能开发中') }
const handleView = (row: any) => { ElMessage.info(`查看客户 ${row.customer_name}`) }
const handleEdit = (row: any) => { ElMessage.info(`编辑客户 ${row.customer_name}`) }
const handleDelete = async (row: any) => {
  try {
    await ElMessageBox.confirm(`确定删除客户 "${row.customer_name}" 吗？`, '删除确认', { type: 'warning' })
    ElMessage.success('删除成功')
    fetchData()
  } catch {}
}

onMounted(() => { fetchData() })
</script>

<style scoped>
.customer-page { padding: 24px; background-color: #f5f7fa; min-height: 100%; }
.page-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 24px; }
.header-left .page-title { font-size: 28px; font-weight: 600; color: #303133; margin: 0 0 12px 0; }
.header-actions { display: flex; gap: 12px; }
.stats-row { margin-bottom: 20px; }
.stat-card { border-radius: 12px; transition: all 0.3s ease; }
.stat-card:hover { transform: translateY(-4px); box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12); }
.stat-card.highlight { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); }
.stat-card.highlight .stat-icon { background: rgba(255, 255, 255, 0.2); color: white; }
.stat-card.highlight .stat-label, .stat-card.highlight .stat-value { color: white; }
.stat-card.warning { background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%); }
.stat-card.warning .stat-icon { background: rgba(255, 255, 255, 0.2); color: white; }
.stat-card.warning .stat-label, .stat-card.warning .stat-value { color: white; }
.stat-content { display: flex; align-items: center; gap: 16px; }
.stat-icon { width: 56px; height: 56px; border-radius: 12px; display: flex; align-items: center; justify-content: center; font-size: 28px; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; }
.stat-icon.total-icon { background: linear-gradient(135deg, #43e97b 0%, #38f9d7 100%); }
.stat-icon.active-icon { background: rgba(255, 255, 255, 0.2); color: white; }
.stat-icon.amount-icon { background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%); }
.stat-icon.credit-icon { background: rgba(255, 255, 255, 0.2); color: white; }
.stat-info { flex: 1; }
.stat-label { font-size: 14px; color: #909399; margin-bottom: 4px; }
.stat-value { font-size: 28px; font-weight: 700; color: #303133; line-height: 1.2; }
.filter-card { margin-bottom: 20px; }
.table-card { margin-bottom: 20px; }
.pagination-wrapper { margin-top: 20px; display: flex; justify-content: flex-end; }
.debt { color: #f56c6c; font-weight: 600; }
:deep(.el-card__header) { padding: 16px 20px; border-bottom: 1px solid #ebeef5; }
:deep(.el-card__body) { padding: 20px; }
</style>

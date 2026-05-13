<template>
  <div class="supplier-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">供应商管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>供应商管理</el-breadcrumb-item>
          <el-breadcrumb-item>供应商列表</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          新建供应商
        </el-button>
      </div>
    </div>

    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon total-icon">
              <el-icon><OfficeBuilding /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">供应商总数</div>
              <div class="stat-value">{{ stats.totalSuppliers }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon active-icon">
              <el-icon><CircleCheck /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">合作中</div>
              <div class="stat-value">{{ stats.activeSuppliers }}</div>
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
              <div class="stat-label">本月采购额</div>
              <div class="stat-value">{{ formatCurrency(stats.monthPurchase) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card warning">
          <div class="stat-content">
            <div class="stat-icon grade-icon">
              <el-icon><Star /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">A级供应商</div>
              <div class="stat-value">{{ stats.gradeASuppliers }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form">
        <el-form-item label="关键词">
          <el-input v-model="queryParams.keyword" placeholder="供应商编码/名称" clearable @clear="handleQuery" />
        </el-form-item>
        <el-form-item label="供应商分类">
          <el-select v-model="queryParams.category" placeholder="选择分类" clearable @change="handleQuery">
            <el-option label="原材料" value="raw_material" />
            <el-option label="辅料" value="accessory" />
            <el-option label="设备" value="equipment" />
            <el-option label="服务" value="service" />
          </el-select>
        </el-form-item>
        <el-form-item label="等级">
          <el-select v-model="queryParams.grade" placeholder="选择等级" clearable @change="handleQuery">
            <el-option label="A级" value="A" />
            <el-option label="B级" value="B" />
            <el-option label="C级" value="C" />
          </el-select>
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="queryParams.status" placeholder="选择状态" clearable @change="handleQuery">
            <el-option label="合作中" value="active" />
            <el-option label="暂停" value="suspended" />
            <el-option label="终止" value="terminated" />
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
      <el-table v-loading="loading" :data="suppliers" stripe>
        <el-table-column prop="supplier_code" label="供应商编码" width="140" fixed />
        <el-table-column prop="supplier_name" label="供应商名称" min-width="180" fixed />
        <el-table-column prop="category" label="分类" width="100">
          <template #default="{ row }">
            <el-tag size="small">{{ getCategoryText(row.category) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="grade" label="等级" width="80">
          <template #default="{ row }">
            <el-tag :type="getGradeType(row.grade)" size="small">{{ row.grade }}级</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="contact_person" label="联系人" width="100" />
        <el-table-column prop="phone" label="联系电话" width="120" />
        <el-table-column prop="lead_time" label="交期(天)" width="80" align="right" />
        <el-table-column prop="payment_terms" label="付款条件" width="100" />
        <el-table-column prop="status" label="状态" width="80">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)" size="small">{{ getStatusText(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleView(row)">详情</el-button>
            <el-button type="primary" link size="small" @click="handleEvaluate(row)">评价</el-button>
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
import { Plus, Search, Refresh, OfficeBuilding, CircleCheck, Money, Star } from '@element-plus/icons-vue'

const loading = ref(false)
const suppliers = ref<any[]>([])
const total = ref(0)

const stats = ref({
  totalSuppliers: 28,
  activeSuppliers: 22,
  monthPurchase: 850000,
  gradeASuppliers: 8
})

const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  category: '',
  grade: '',
  status: ''
})

const formatCurrency = (amount: number) => {
  return new Intl.NumberFormat('zh-CN', { style: 'currency', currency: 'CNY', minimumFractionDigits: 0 }).format(amount)
}

const getCategoryText = (category: string) => {
  const map: Record<string, string> = { raw_material: '原材料', accessory: '辅料', equipment: '设备', service: '服务' }
  return map[category] || category
}

const getGradeType = (grade: string) => {
  const map: Record<string, any> = { A: 'success', B: 'warning', C: 'info' }
  return map[grade] || 'info'
}

const getStatusType = (status: string) => {
  const map: Record<string, any> = { active: 'success', suspended: 'warning', terminated: 'danger' }
  return map[status] || 'info'
}

const getStatusText = (status: string) => {
  const map: Record<string, string> = { active: '合作中', suspended: '暂停', terminated: '终止' }
  return map[status] || status
}

const fetchData = async () => {
  loading.value = true
  try {
    suppliers.value = [
      { id: 1, supplier_code: 'S001', supplier_name: '纺织原料供应商A', category: 'raw_material', grade: 'A', contact_person: '张经理', phone: '13800138000', lead_time: 7, payment_terms: '月结30天', status: 'active' },
      { id: 2, supplier_code: 'S002', supplier_name: '染料供应商B', category: 'raw_material', grade: 'B', contact_person: '李经理', phone: '13900139000', lead_time: 5, payment_terms: '月结30天', status: 'active' },
      { id: 3, supplier_code: 'S003', supplier_name: '包装材料供应商C', category: 'accessory', grade: 'C', contact_person: '王经理', phone: '13700137000', lead_time: 3, payment_terms: '预付', status: 'active' }
    ]
    total.value = 3
    ElMessage.info('使用演示数据')
  } finally {
    loading.value = false
  }
}

const handleQuery = () => { queryParams.page = 1; fetchData() }
const handleReset = () => { queryParams.keyword = ''; queryParams.category = ''; queryParams.grade = ''; queryParams.status = ''; handleQuery() }
const handleCreate = () => { ElMessage.info('新建供应商功能开发中') }
const handleView = (row: any) => { ElMessage.info(`查看供应商 ${row.supplier_name}`) }
const handleEvaluate = (row: any) => { ElMessage.info(`评价供应商 ${row.supplier_name}`) }
const handleDelete = async (row: any) => {
  try {
    await ElMessageBox.confirm(`确定删除供应商 "${row.supplier_name}" 吗？`, '删除确认', { type: 'warning' })
    ElMessage.success('删除成功')
    fetchData()
  } catch {}
}

onMounted(() => { fetchData() })
</script>

<style scoped>
.supplier-page { padding: 24px; background-color: #f5f7fa; min-height: 100%; }
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
.stat-icon.active-icon { background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%); }
.stat-icon.amount-icon { background: rgba(255, 255, 255, 0.2); color: white; }
.stat-icon.grade-icon { background: rgba(255, 255, 255, 0.2); color: white; }
.stat-info { flex: 1; }
.stat-label { font-size: 14px; color: #909399; margin-bottom: 4px; }
.stat-value { font-size: 28px; font-weight: 700; color: #303133; line-height: 1.2; }
.filter-card { margin-bottom: 20px; }
.table-card { margin-bottom: 20px; }
.pagination-wrapper { margin-top: 20px; display: flex; justify-content: flex-end; }
:deep(.el-card__header) { padding: 16px 20px; border-bottom: 1px solid #ebeef5; }
:deep(.el-card__body) { padding: 20px; }
</style>

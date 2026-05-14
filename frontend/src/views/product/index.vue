<template>
  <div class="product-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">产品管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>基础数据</el-breadcrumb-item>
          <el-breadcrumb-item>产品管理</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          新建产品
        </el-button>
        <el-button @click="handleImport">
          <el-icon><Upload /></el-icon>
          导入
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          导出
        </el-button>
      </div>
    </div>

    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon total-icon">
              <el-icon><Goods /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">产品总数</div>
              <div class="stat-value">{{ stats.totalProducts }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card highlight">
          <div class="stat-content">
            <div class="stat-icon active-icon">
              <el-icon><CircleCheck /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">启用产品</div>
              <div class="stat-value">{{ stats.activeProducts }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card warning">
          <div class="stat-content">
            <div class="stat-icon category-icon">
              <el-icon><Collection /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">产品分类</div>
              <div class="stat-value">{{ stats.totalCategories }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon price-icon">
              <el-icon><Money /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">平均价格</div>
              <div class="stat-value">{{ formatCurrency(stats.avgPrice) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form">
        <el-form-item label="关键词">
          <el-input v-model="queryParams.keyword" placeholder="产品编码/名称" clearable />
        </el-form-item>
        <el-form-item label="分类">
          <el-cascader
            v-model="queryParams.category_id"
            :options="categoryTree"
            :props="{ checkStrictly: true, emitPath: false }"
            placeholder="选择分类"
            clearable
          />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="queryParams.is_active" placeholder="选择状态" clearable>
            <el-option label="启用" :value="true" />
            <el-option label="禁用" :value="false" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">查询</el-button>
          <el-button @click="handleReset">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table v-loading="loading" :data="products" stripe>
        <el-table-column prop="product_code" label="产品编码" width="140" fixed />
        <el-table-column prop="product_name" label="产品名称" min-width="180" fixed />
        <el-table-column prop="category_name" label="分类" width="120">
          <template #default="{ row }">
            <el-tag v-if="row.category_name" type="info" size="small">{{ row.category_name }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="specification" label="规格" width="120" show-overflow-tooltip />
        <el-table-column prop="unit" label="单位" width="80" />
        <el-table-column prop="price" label="售价" width="100" align="right">
          <template #default="{ row }">
            <span v-if="row.price">{{ formatCurrency(row.price) }}</span>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column prop="cost_price" label="成本" width="100" align="right">
          <template #default="{ row }">
            <span v-if="row.cost_price">{{ formatCurrency(row.cost_price) }}</span>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column prop="barcode" label="条形码" width="140" />
        <el-table-column prop="is_active" label="状态" width="80">
          <template #default="{ row }">
            <el-tag :type="row.is_active ? 'success' : 'info'" size="small">
              {{ row.is_active ? '启用' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="200" fixed="right">
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
import { Plus, Upload, Download, Goods, CircleCheck, Collection, Money } from '@element-plus/icons-vue'

const loading = ref(false)
const products = ref<any[]>([])
const categories = ref<any[]>([])
const categoryTree = ref<any[]>([])
const total = ref(0)

const stats = ref({
  totalProducts: 256,
  activeProducts: 230,
  totalCategories: 18,
  avgPrice: 88.5
})

const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  category_id: undefined as number | undefined,
  is_active: undefined as boolean | undefined
})

const formatCurrency = (amount: number) => `¥${amount.toFixed(2)}`

const fetchData = async () => {
  loading.value = true
  try {
    products.value = [
      { id: 1, product_code: 'P001', product_name: '纯棉斜纹布', category_name: '棉布', specification: '150cm/200g', unit: '米', price: 25.5, cost_price: 18.0, barcode: '6901234567890', is_active: true },
      { id: 2, product_code: 'P002', product_name: '涤纶平纹布', category_name: '化纤', specification: '145cm/150g', unit: '米', price: 18.0, cost_price: 12.0, barcode: '6901234567891', is_active: true },
      { id: 3, product_code: 'P003', product_name: '真丝缎面', category_name: '丝绸', specification: '110cm/80g', unit: '米', price: 180.0, cost_price: 120.0, barcode: '6901234567892', is_active: true }
    ]
    categories.value = [
      { id: 1, name: '棉布', children: [] },
      { id: 2, name: '化纤', children: [] },
      { id: 3, name: '丝绸', children: [] }
    ]
    categoryTree.value = categories.value
    total.value = 3
    ElMessage.info('使用演示数据')
  } finally {
    loading.value = false
  }
}

const handleQuery = () => { fetchData() }
const handleReset = () => { queryParams.keyword = ''; queryParams.category_id = undefined; queryParams.is_active = undefined; fetchData() }
const handleCreate = () => { ElMessage.info('新建产品功能开发中') }
const handleImport = () => { ElMessage.info('导入功能开发中') }
const handleExport = () => { ElMessage.info('导出功能开发中') }
const handleView = (row: any) => { ElMessage.info(`查看产品 ${row.product_name}`) }
const handleEdit = (row: any) => { ElMessage.info(`编辑产品 ${row.product_name}`) }
const handleDelete = async (row: any) => {
  try {
    await ElMessageBox.confirm(`确定删除产品 "${row.product_name}" 吗？`, '删除确认', { type: 'warning' })
    ElMessage.success('删除成功')
    fetchData()
  } catch {}
}

onMounted(() => { fetchData() })
</script>

<style scoped>
.product-page { padding: 24px; background-color: #f5f7fa; min-height: 100%; }
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
.stat-icon.category-icon { background: rgba(255, 255, 255, 0.2); color: white; }
.stat-icon.price-icon { background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%); }
.stat-info { flex: 1; }
.stat-label { font-size: 14px; color: #909399; margin-bottom: 4px; }
.stat-value { font-size: 28px; font-weight: 700; color: #303133; line-height: 1.2; }
.filter-card { margin-bottom: 20px; }
.table-card { margin-bottom: 20px; }
.pagination-wrapper { margin-top: 20px; display: flex; justify-content: flex-end; }
:deep(.el-card__header) { padding: 16px 20px; border-bottom: 1px solid #ebeef5; }
:deep(.el-card__body) { padding: 20px; }
</style>

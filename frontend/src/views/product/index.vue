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

    <!-- 新增/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="dialogTitle"
      width="700px"
      :close-on-click-modal="false"
      @close="resetForm"
    >
      <el-form
        ref="formRef"
        :model="formData"
        :rules="formRules"
        label-width="100px"
        :disabled="dialogMode === 'view'"
      >
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="产品编码" prop="product_code">
              <el-input v-model="formData.product_code" placeholder="请输入产品编码" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="产品名称" prop="product_name">
              <el-input v-model="formData.product_name" placeholder="请输入产品名称" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="产品分类" prop="category_id">
              <el-select v-model="formData.category_id" placeholder="请选择分类" style="width: 100%">
                <el-option
                  v-for="item in categories"
                  :key="item.id"
                  :label="item.name"
                  :value="item.id"
                />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="规格" prop="specification">
              <el-input v-model="formData.specification" placeholder="请输入规格" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="单位" prop="unit">
              <el-input v-model="formData.unit" placeholder="请输入单位" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="条形码" prop="barcode">
              <el-input v-model="formData.barcode" placeholder="请输入条形码" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="售价" prop="price">
              <el-input-number v-model="formData.price" :min="0" :precision="2" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="成本价" prop="cost_price">
              <el-input-number v-model="formData.cost_price" :min="0" :precision="2" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="描述" prop="description">
          <el-input v-model="formData.description" type="textarea" :rows="3" placeholder="请输入描述" />
        </el-form-item>
        <el-form-item label="状态" prop="is_active">
          <el-switch v-model="formData.is_active" active-text="启用" inactive-text="禁用" />
        </el-form-item>
      </el-form>
      <template #footer v-if="dialogMode !== 'view'">
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { Plus, Upload, Download, Goods, CircleCheck, Collection, Money } from '@element-plus/icons-vue'
import { productApi, type Product, type ProductCategory } from '@/api/product'

const loading = ref(false)
const submitLoading = ref(false)
const products = ref<Product[]>([])
const categories = ref<ProductCategory[]>([])
const categoryTree = ref<ProductCategory[]>([])
const total = ref(0)
const dialogVisible = ref(false)
const dialogMode = ref<'create' | 'edit' | 'view'>('create')
const formRef = ref<FormInstance>()

const stats = ref({
  totalProducts: 0,
  activeProducts: 0,
  totalCategories: 0,
  avgPrice: 0
})

const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  category_id: undefined as number | undefined,
  is_active: undefined as boolean | undefined
})

const formData = reactive({
  id: undefined as number | undefined,
  product_code: '',
  product_name: '',
  category_id: undefined as number | undefined,
  specification: '',
  unit: '',
  barcode: '',
  price: 0,
  cost_price: 0,
  description: '',
  is_active: true
})

const formRules: FormRules = {
  product_code: [
    { required: true, message: '请输入产品编码', trigger: 'blur' },
    { max: 50, message: '长度不能超过50个字符', trigger: 'blur' }
  ],
  product_name: [
    { required: true, message: '请输入产品名称', trigger: 'blur' },
    { max: 200, message: '长度不能超过200个字符', trigger: 'blur' }
  ],
  category_id: [
    { required: true, message: '请选择产品分类', trigger: 'change' }
  ],
  unit: [
    { required: true, message: '请输入单位', trigger: 'blur' }
  ]
}

const dialogTitle = computed(() => {
  const titles = {
    create: '新建产品',
    edit: '编辑产品',
    view: '查看产品'
  }
  return titles[dialogMode.value]
})

const formatCurrency = (amount: number) => `¥${(amount || 0).toFixed(2)}`

const fetchData = async () => {
  loading.value = true
  try {
    const res = await productApi.list(queryParams)
    products.value = res.data?.list || []
    total.value = res.data?.total || 0
    
    // 计算统计数据
    stats.value.totalProducts = total.value
    stats.value.activeProducts = products.value.filter(p => p.is_active).length
    stats.value.avgPrice = products.value.length > 0
      ? products.value.reduce((sum, p) => sum + (p.price || 0), 0) / products.value.length
      : 0
  } catch (error: any) {
    ElMessage.error(error.message || '获取产品列表失败')
    products.value = []
    total.value = 0
  } finally {
    loading.value = false
  }
}

const fetchCategories = async () => {
  try {
    const res = await productApi.getCategories()
    categories.value = res.data || []
    categoryTree.value = buildTree(res.data || [])
    stats.value.totalCategories = categories.value.length
  } catch (error: any) {
    console.error('获取分类失败:', error)
  }
}

const buildTree = (items: ProductCategory[]): ProductCategory[] => {
  const map = new Map<number, ProductCategory>()
  const tree: ProductCategory[] = []
  
  items.forEach(item => {
    map.set(item.id, { ...item, children: [] })
  })
  
  items.forEach(item => {
    const node = map.get(item.id)!
    if (item.parent_id && map.has(item.parent_id)) {
      map.get(item.parent_id)!.children!.push(node)
    } else {
      tree.push(node)
    }
  })
  
  return tree
}

const handleQuery = () => {
  queryParams.page = 1
  fetchData()
}

const handleReset = () => {
  queryParams.keyword = ''
  queryParams.category_id = undefined
  queryParams.is_active = undefined
  handleQuery()
}

const resetForm = () => {
  formData.id = undefined
  formData.product_code = ''
  formData.product_name = ''
  formData.category_id = undefined
  formData.specification = ''
  formData.unit = ''
  formData.barcode = ''
  formData.price = 0
  formData.cost_price = 0
  formData.description = ''
  formData.is_active = true
  formRef.value?.clearValidate()
}

const handleCreate = () => {
  resetForm()
  dialogMode.value = 'create'
  dialogVisible.value = true
}

const handleView = (row: Product) => {
  resetForm()
  Object.assign(formData, row)
  dialogMode.value = 'view'
  dialogVisible.value = true
}

const handleEdit = (row: Product) => {
  resetForm()
  Object.assign(formData, row)
  dialogMode.value = 'edit'
  dialogVisible.value = true
}

const handleDelete = async (row: Product) => {
  try {
    await ElMessageBox.confirm(`确定删除产品 "${row.product_name}" 吗？`, '删除确认', { type: 'warning' })
    await productApi.delete(row.id)
    ElMessage.success('删除成功')
    fetchData()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '删除失败')
    }
  }
}

const handleSubmit = async () => {
  if (!formRef.value) return
  
  await formRef.value.validate(async (valid) => {
    if (!valid) return
    
    submitLoading.value = true
    try {
      if (dialogMode.value === 'create') {
        await productApi.create(formData)
        ElMessage.success('创建成功')
      } else {
        await productApi.update(formData.id!, formData)
        ElMessage.success('更新成功')
      }
      dialogVisible.value = false
      fetchData()
    } catch (error: any) {
      ElMessage.error(error.message || '操作失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const handleImport = () => {
  ElMessage.info('导入功能开发中')
}

const handleExport = async () => {
  try {
    await productApi.export(queryParams)
    ElMessage.success('导出成功')
  } catch (error: any) {
    ElMessage.error(error.message || '导出失败')
  }
}

onMounted(() => {
  fetchData()
  fetchCategories()
})
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

<!--
  ProductListTab.vue - 产品列表 Tab
  来源：原 product/index.vue 中 列表/统计/过滤内容
  拆分日期：2026-06-15 B3-4
-->
<template>
  <div class="product-list">
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
        <el-card
          shadow="hover"
          class="stat-card warning"
          style="cursor: pointer"
          @click="emit('openCategory')"
        >
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
          <!-- P2-10 修复（批次 82 v1 复审）：补齐 v-permission 按钮权限 -->
          <el-button v-permission="'products:create'" type="primary" @click="emit('openForm', 'create', null)">
            <el-icon><Plus /></el-icon>新建
          </el-button>
          <el-button @click="emit('openImport')">
            <el-icon><Upload /></el-icon>导入
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table v-loading="loading" :data="products" stripe>
        <el-table-column prop="product_code" label="产品编码" width="140" fixed />
        <el-table-column prop="product_name" label="产品名称" min-width="180" fixed />
        <el-table-column prop="category_name" label="分类" width="120">
          <template #default="{ row }">
            <el-tag v-if="row.category_name" type="info" size="small">{{
              row.category_name
            }}</el-tag>
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
            <el-button type="primary" link size="small" @click="emit('openForm', 'view', row)"
              >详情</el-button
            >
            <el-button type="primary" link size="small" @click="emit('openForm', 'edit', row)"
              >编辑</el-button
            >
            <el-button type="danger" link size="small" @click="handleDelete(row)">删除</el-button>
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
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
        />
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
// 批次 277：迁移到 useTableApi composable，移除手写分页逻辑
import { ref, reactive, watch, onMounted, defineEmits, defineExpose } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Upload, Goods, CircleCheck, Collection, Money } from '@element-plus/icons-vue'
import { productApi, type Product, type ProductCategory } from '@/api/product'
import { useTableApi } from '@/composables/useTableApi'

const emit = defineEmits<{
  openForm: [mode: 'create' | 'edit' | 'view', row: Product | null]
  openImport: []
  openCategory: []
}>()

// 批次 277：使用 useTableApi 管理列表分页/筛选/loading/total 状态，自动 watch 分页变化并初始加载
const {
  data: products,
  total,
  loading,
  page,
  pageSize,
  queryParams,
  refresh: fetchData,
  setQueryParam,
} = useTableApi<Product>({
  url: '/products',
  defaultParams: {
    keyword: '',
    category_id: undefined as number | undefined,
    is_active: undefined as boolean | undefined,
  },
  onError: (err: unknown) => {
    // 批次 277：类型守卫处理错误，避免直接 as Error 强转
    const message = err instanceof Error ? err.message : '获取产品列表失败'
    ElMessage.error(message)
  },
})

// 分类树与列表（fetchCategories 仍手写，与列表分页无关）
const categories = ref<ProductCategory[]>([])
const categoryTree = ref<ProductCategory[]>([])

const stats = reactive({
  totalProducts: 0,
  activeProducts: 0,
  totalCategories: 0,
  avgPrice: 0,
})

const formatCurrency = (amount: number) => `¥${(amount || 0).toFixed(2)}`

const buildTree = (items: ProductCategory[]): ProductCategory[] => {
  const map = new Map<number, ProductCategory>()
  const tree: ProductCategory[] = []
  items.forEach(item => {
    map.set(item.id, { ...item, children: [] })
  })
  items.forEach(item => {
    const node = map.get(item.id)
    if (!node) return
    if (item.parent_id && map.has(item.parent_id)) {
      const parent = map.get(item.parent_id)
      if (parent?.children) parent.children.push(node)
    } else {
      tree.push(node)
    }
  })
  return tree
}

// 批次 277：watch data 自动更新统计指标（原 fetchData 内联逻辑迁移至此）
watch(products, () => {
  stats.totalProducts = total.value
  stats.activeProducts = products.value.filter(p => p.is_active).length
  stats.avgPrice =
    products.value.length > 0
      ? products.value.reduce((sum, p) => sum + (p.price || 0), 0) / products.value.length
      : 0
})

const fetchCategories = async () => {
  try {
    const res = await productApi.getCategories()
    categories.value = (res.data as ProductCategory[] | undefined) || []
    categoryTree.value = buildTree(categories.value)
    stats.totalCategories = categories.value.length
  } catch (error) {
    // 主入口已记录日志
  }
}

// 批次 277：将 queryParams 筛选字段同步到 setQueryParam，确保请求参数生效
const syncQueryParams = () => {
  setQueryParam('keyword', queryParams.value.keyword)
  setQueryParam('category_id', queryParams.value.category_id)
  setQueryParam('is_active', queryParams.value.is_active)
}

// 批次 277：分页页码变化处理（由 useTableApi watch 自动触发重载）
const handlePageChange = (p: number) => {
  page.value = p
}

// 批次 277：分页每页条数变化处理（由 useTableApi watch 自动触发重载）
const handleSizeChange = (s: number) => {
  pageSize.value = s
}

const handleQuery = () => {
  // 批次 277：同步筛选参数并回到首页重载
  syncQueryParams()
  page.value = 1
  fetchData()
}
const handleReset = () => {
  queryParams.value.keyword = ''
  queryParams.value.category_id = undefined
  queryParams.value.is_active = undefined
  handleQuery()
}

const handleDelete = async (row: Product) => {
  try {
    await ElMessageBox.confirm(`确定删除产品 "${row.product_name}" 吗？`, '删除确认', {
      type: 'warning',
    })
    await productApi.delete(row.id)
    ElMessage.success('删除成功')
    fetchData()
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error((error as Error).message || '删除失败')
    }
  }
}

defineExpose({ fetchData, fetchCategories })
// 批次 277：useTableApi 自动初始加载列表，onMounted 仅调用 fetchCategories 获取分类树
onMounted(() => {
  fetchCategories()
})
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
.stat-card.highlight {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}
.stat-card.highlight :deep(.stat-icon) {
  background: rgba(255, 255, 255, 0.2);
}
.stat-card.warning {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
}
.stat-card.warning :deep(.stat-icon) {
  background: rgba(255, 255, 255, 0.2);
}
.stat-card.highlight :deep(.stat-label),
.stat-card.highlight :deep(.stat-value),
.stat-card.warning :deep(.stat-label),
.stat-card.warning :deep(.stat-value) {
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
  background: linear-gradient(135deg, #43e97b 0%, #38f9d7 100%);
}
:deep(.stat-icon.active-icon),
:deep(.stat-icon.category-icon) {
  background: rgba(255, 255, 255, 0.2);
}
:deep(.stat-icon.price-icon) {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
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

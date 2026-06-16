<!--
  product/index.vue - 产品管理主入口（容器组件）
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-4）：
  原 847 行"上帝组件"已拆分为以下结构：

  - tabs/ProductListTab.vue         （产品列表 + 过滤 + 统计）
  - tabs/ProductFormDialogTab.vue   （新建/编辑弹窗）
  - tabs/ImportDialogTab.vue        （导入弹窗）
  - tabs/CategoryDialogTab.vue      （分类管理弹窗）

  本主入口仅承担：Tab 切换与公共样式。
-->
<template>
  <div class="product-page">
    <div class="page-header">
      <h1 class="page-title">产品管理</h1>
      <el-breadcrumb separator="/">
        <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
        <el-breadcrumb-item>基础数据</el-breadcrumb-item>
        <el-breadcrumb-item>产品管理</el-breadcrumb-item>
      </el-breadcrumb>
    </div>

    <ProductListTab @open-form="openForm" @open-import="openImport" @open-category="openCategory" />

    <ProductFormDialogTab
      v-model="formDialogVisible"
      :title="formDialogTitle"
      :row-data="currentRow"
      :categories="categories"
      :mode="dialogMode"
      @submitted="handleSubmitted"
    />

    <ImportDialogTab v-model="importDialogVisible" @submitted="handleSubmitted" />

    <CategoryDialogTab v-model="categoryDialogVisible" @changed="fetchCategories" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { productApi, type Product, type ProductCategory } from '@/api/product'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'
import ProductListTab from './tabs/ProductListTab.vue'
import ProductFormDialogTab from './tabs/ProductFormDialogTab.vue'
import ImportDialogTab from './tabs/ImportDialogTab.vue'
import CategoryDialogTab from './tabs/CategoryDialogTab.vue'

const formDialogVisible = ref(false)
const formDialogTitle = ref('新建产品')
const dialogMode = ref<'create' | 'edit' | 'view'>('create')
const currentRow = ref<Product | null>(null)
const importDialogVisible = ref(false)
const categoryDialogVisible = ref(false)

const categories = ref<ProductCategory[]>([])

const openForm = (mode: 'create' | 'edit' | 'view', row: Product | null) => {
  currentRow.value = row
  dialogMode.value = mode
  formDialogTitle.value = mode === 'create' ? '新建产品' : mode === 'edit' ? '编辑产品' : '查看产品'
  formDialogVisible.value = true
}

const openImport = () => {
  importDialogVisible.value = true
}

const openCategory = () => {
  categoryDialogVisible.value = true
}

const handleSubmitted = () => {
  // 子组件已通过 emit 触发刷新
}

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

const fetchCategories = async () => {
  try {
    const res = await productApi.getCategories()
    categories.value = (res.data as ProductCategory[] | undefined) || []
    void buildTree(categories.value)
  } catch (error) {
    logger.error('获取分类失败', (error as Error).message)
  }
}

const hasLoaded = createLazyLoader()
onMounted(() => {
  loadIfNot('productCategories', fetchCategories, hasLoaded)
})
</script>

<style scoped>
.product-page {
  padding: 24px;
  background: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}
.page-title {
  font-size: 28px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 12px 0;
}
</style>

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
  D05 Batch 8 Group B：接入 useI18n
-->
<template>
  <div class="product-page">
    <div class="page-header">
      <h1 class="page-title">{{ t('product.index.pageTitle') }}</h1>
      <el-breadcrumb separator="/">
        <el-breadcrumb-item :to="{ path: '/' }">{{
          t('product.index.breadcrumbHome')
        }}</el-breadcrumb-item>
        <el-breadcrumb-item>{{ t('product.index.breadcrumbMasterData') }}</el-breadcrumb-item>
        <el-breadcrumb-item>{{ t('product.index.breadcrumbProduct') }}</el-breadcrumb-item>
      </el-breadcrumb>
    </div>

    <ProductListTab
      ref="listTabRef"
      @open-form="openForm"
      @open-import="openImport"
      @open-category="openCategory"
    />

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
import { ref, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { getProductCategoryList, type Product, type ProductCategory } from '@/api/product';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import { logAuxLoadFailure } from '@/utils/logger';
import ProductListTab from './tabs/ProductListTab.vue';
import ProductFormDialogTab from './tabs/ProductFormDialogTab.vue';
import ImportDialogTab from './tabs/ImportDialogTab.vue';
import CategoryDialogTab from './tabs/CategoryDialogTab.vue';

const { t } = useI18n({ useScope: 'global' });

const formDialogVisible = ref(false);
const formDialogTitle = ref(t('product.index.titleCreate'));
const dialogMode = ref<'create' | 'edit' | 'view'>('create');
const currentRow = ref<Product | null>(null);
const importDialogVisible = ref(false);
const categoryDialogVisible = ref(false);

const categories = ref<ProductCategory[]>([]);

// 列表子组件句柄：ProductListTab 通过 defineExpose 交出 fetchData 与 fetchCategories，
// 本容器在保存/导入提交成功后调用 fetchData 重取列表，驱动编辑弹窗下次以库内新值回显。
const listTabRef = ref<InstanceType<typeof ProductListTab> | null>(null);

const openForm = (mode: 'create' | 'edit' | 'view', row: Product | null) => {
  currentRow.value = row;
  dialogMode.value = mode;
  formDialogTitle.value =
    mode === 'create'
      ? t('product.index.titleCreate')
      : mode === 'edit'
        ? t('product.index.titleEdit')
        : t('product.index.titleView');
  formDialogVisible.value = true;
};

const openImport = () => {
  importDialogVisible.value = true;
};

const openCategory = () => {
  categoryDialogVisible.value = true;
};

const handleSubmitted = () => {
  // 保存与导入落库成功后由本容器驱动列表重取：弹窗的提交结果只 emit 给这里，
  // 列表数据归 ProductListTab 自己管，本函数若不做任何事，新建的行不进列表、
  // 编辑过的行在下次打开弹窗时仍以保存前的旧行回显，用户即误判为「填了保存不了」。
  // 读：子组件暴露的 fetchData；写给谁：useTableApi 以当前分页与筛选参数重发列表请求；
  // 存什么存哪里：不落前端存储，页面真相始终以服务端 products 表为准。
  listTabRef.value?.fetchData();
};

const buildTree = (items: ProductCategory[]): ProductCategory[] => {
  const map = new Map<number, ProductCategory>();
  const tree: ProductCategory[] = [];
  items.forEach(item => {
    map.set(item.id, { ...item, children: [] });
  });
  items.forEach(item => {
    const node = map.get(item.id);
    if (!node) return;
    if (item.parent_id && map.has(item.parent_id)) {
      const parent = map.get(item.parent_id);
      if (parent?.children) parent.children.push(node);
    } else {
      tree.push(node);
    }
  });
  return tree;
};

const fetchCategories = async () => {
  try {
    const res = await getProductCategoryList();
    categories.value = res.data.items;
    void buildTree(categories.value);
  } catch (error) {
    logAuxLoadFailure(t('product.index.messageFetchCategoriesFailed'), error);
  }
};

const hasLoaded = createLazyLoader();
onMounted(() => {
  loadIfNot('productCategories', fetchCategories, hasLoaded);
});
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

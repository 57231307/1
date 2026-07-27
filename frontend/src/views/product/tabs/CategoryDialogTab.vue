<!--
  CategoryDialogTab.vue - 产品分类管理对话框
  来源：原 product/index.vue 中 分类管理弹窗
  拆分日期：2026-06-15 B3-4
  D05 Batch 8 Group B：接入 useI18n
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="t('product.categoryDialogTab.title')"
    width="600px"
    :aria-label="t('product.categoryDialogTab.ariaLabel')"
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <div class="category-dialog-content">
      <div class="category-add-form">
        <el-input
          v-model="newCategoryName"
          :placeholder="t('product.categoryDialogTab.placeholderNewCategory')"
          style="width: 300px; margin-right: 10px"
        />
        <el-button type="primary" @click="handleAdd">
          <el-icon><Plus /></el-icon>
          {{ t('product.categoryDialogTab.buttonAddCategory') }}
        </el-button>
      </div>
      <el-table
        v-loading="loading"
        :data="categories"
        stripe
        :aria-label="t('product.categoryDialogTab.tableAriaLabel')"
        style="margin-top: 15px"
      >
        <el-table-column prop="id" :label="t('product.categoryDialogTab.colId')" width="80" />
        <el-table-column prop="name" :label="t('product.categoryDialogTab.colName')" />
        <el-table-column
          prop="description"
          :label="t('product.categoryDialogTab.colDescription')"
        />
        <el-table-column :label="t('product.categoryDialogTab.colActions')" width="120">
          <template #default="{ row }">
            <el-button type="danger" link size="small" @click="handleDelete(row)">{{
              t('product.categoryDialogTab.buttonDelete')
            }}</el-button>
          </template>
        </el-table-column>
      </el-table>
    </div>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import {
  getProductCategoryList,
  createProductCategory,
  deleteProductCategory,
  type ProductCategory,
} from '@/api/product';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

interface Props {
  modelValue: boolean;
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void;
  (e: 'changed'): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();

const categories = ref<ProductCategory[]>([]);
const newCategoryName = ref('');
const loading = ref(false);

const fetchCategories = async () => {
  loading.value = true;
  try {
    const res = await getProductCategoryList();
    categories.value = (res.data as ProductCategory[] | undefined) || [];
  } catch (error) {
    const err = error as Error;
    logger.error(t('product.categoryDialogTab.messageFetchFailed'), err.message);
  } finally {
    loading.value = false;
  }
};

const handleAdd = async () => {
  if (!newCategoryName.value.trim()) {
    ElMessage.warning(t('product.categoryDialogTab.messageCategoryNameRequired'));
    return;
  }
  try {
    await createProductCategory({ name: newCategoryName.value.trim() });
    ElMessage.success(t('product.categoryDialogTab.messageAddSuccess'));
    newCategoryName.value = '';
    fetchCategories();
    emit('changed');
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('product.categoryDialogTab.messageAddFailed'));
  }
};

const handleDelete = async (row: ProductCategory) => {
  try {
    await ElMessageBox.confirm(
      t('product.categoryDialogTab.messageDeleteConfirm', { name: row.name }),
      t('product.categoryDialogTab.messageDeleteTitle'),
      { type: 'warning' }
    );
    await deleteProductCategory(row.id);
    ElMessage.success(t('product.categoryDialogTab.messageDeleteSuccess'));
    fetchCategories();
    emit('changed');
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error;
      ElMessage.error(err.message || t('product.categoryDialogTab.messageDeleteFailed'));
    }
  }
};

watch(
  () => props.modelValue,
  val => {
    if (val) {
      fetchCategories();
    }
  }
);
</script>

<style scoped>
.category-dialog-content {
  padding: 10px 0;
}
.category-add-form {
  display: flex;
  align-items: center;
}
</style>

<!--
  CategoryDialogTab.vue - 产品分类管理对话框
  来源：原 product/index.vue 中 分类管理弹窗
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    title="产品分类管理"
    width="600px"
    aria-label="产品分类对话框"
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <div class="category-dialog-content">
      <div class="category-add-form">
        <el-input
          v-model="newCategoryName"
          placeholder="输入新分类名称"
          style="width: 300px; margin-right: 10px"
        />
        <el-button type="primary" @click="handleAdd">
          <el-icon><Plus /></el-icon>
          添加分类
        </el-button>
      </div>
      <el-table
        v-loading="loading"
        :data="categories"
        stripe
        aria-label="产品分类列表"
        style="margin-top: 15px"
      >
        <el-table-column prop="id" label="ID" width="80" />
        <el-table-column prop="name" label="分类名称" />
        <el-table-column prop="description" label="描述" />
        <el-table-column label="操作" width="120">
          <template #default="{ row }">
            <el-button type="danger" link size="small" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </div>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  getProductCategoryList,
  createProductCategory,
  deleteProductCategory,
  type ProductCategory,
} from '@/api/product'
import { logger } from '@/utils/logger'

interface Props {
  modelValue: boolean
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void
  (e: 'changed'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const categories = ref<ProductCategory[]>([])
const newCategoryName = ref('')
const loading = ref(false)

const fetchCategories = async () => {
  loading.value = true
  try {
    const res = await getProductCategoryList()
    categories.value = (res.data as ProductCategory[] | undefined) || []
  } catch (error) {
    const err = error as Error
    logger.error('获取分类失败', err.message)
  } finally {
    loading.value = false
  }
}

const handleAdd = async () => {
  if (!newCategoryName.value.trim()) {
    ElMessage.warning('请输入分类名称')
    return
  }
  try {
    await createProductCategory({ name: newCategoryName.value.trim() })
    ElMessage.success('添加成功')
    newCategoryName.value = ''
    fetchCategories()
    emit('changed')
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '添加失败')
  }
}

const handleDelete = async (row: ProductCategory) => {
  try {
    await ElMessageBox.confirm(`确定删除分类 "${row.name}" 吗？`, '删除确认', {
      type: 'warning',
    })
    await deleteProductCategory(row.id)
    ElMessage.success('删除成功')
    fetchCategories()
    emit('changed')
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || '删除失败')
    }
  }
}

watch(
  () => props.modelValue,
  val => {
    if (val) {
      fetchCategories()
    }
  }
)
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

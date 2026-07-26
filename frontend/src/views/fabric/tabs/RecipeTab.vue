<!--
  RecipeTab.vue - 染色配方 Tab
  来源：原 fabric/index.vue 中 染色配方 tab 内容
  拆分日期：2026-06-15 B3-4
-->
<template>
  <div class="recipe-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('fabric.recipeTab.title') }}</h2>
      <el-button type="primary" @click="openCreate">
        <el-icon><Plus /></el-icon>
        {{ t('fabric.recipeTab.buttonCreate') }}
      </el-button>
    </div>

    <el-card shadow="hover">
      <el-table v-loading="loading" :data="recipes" stripe :aria-label="t('fabric.recipeTab.tableAriaLabel')">
        <el-table-column prop="recipe_no" :label="t('fabric.recipeTab.columnRecipeNo')" width="120" />
        <el-table-column prop="recipe_name" :label="t('fabric.recipeTab.columnName')" width="150" />
        <el-table-column prop="color_name" :label="t('fabric.recipeTab.columnColor')" width="120" />
        <el-table-column prop="fabric_type" :label="t('fabric.recipeTab.columnFabricType')" width="120" />
        <el-table-column prop="version" :label="t('fabric.recipeTab.columnVersion')" width="80" />
        <el-table-column prop="status" :label="t('fabric.recipeTab.columnStatus')" width="100" align="center">
          <template #default="{ row }">
            <el-tag
              :type="
                row.status === 'approved' ? 'success' : row.status === 'draft' ? 'info' : 'danger'
              "
              size="small"
            >
              {{ getStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_at" :label="t('fabric.recipeTab.columnCreatedAt')" width="160" />
        <el-table-column :label="t('fabric.recipeTab.columnAction')" width="240" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleView(row)">{{ t('fabric.recipeTab.buttonView') }}</el-button>
            <el-button
              v-if="row.status === 'draft'"
              type="success"
              link
              size="small"
              @click="handleApprove(row)"
              >{{ t('fabric.recipeTab.buttonApprove') }}</el-button
            >
            <el-button
              v-if="row.status === 'approved'"
              type="warning"
              link
              size="small"
              @click="handleNewVersion(row)"
              >{{ t('fabric.recipeTab.buttonNewVersion') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, defineEmits } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  getDyeRecipe,
  approveDyeRecipe,
  createNewVersion as createNewVersionApi,
  type DyeRecipe,
} from '@/api/dye-recipe'
import { logger } from '@/utils/logger'

const { t } = useI18n({ useScope: 'global' })

const emit = defineEmits<{ openDialog: [row: DyeRecipe | null] }>()

const recipes = ref<DyeRecipe[]>([])
const loading = ref(false)

const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    draft: t('fabric.recipeTab.statusDraft'),
    approved: t('fabric.recipeTab.statusApproved'),
    obsolete: t('fabric.recipeTab.statusObsolete'),
  }
  return map[status] || status
}

const fetchRecipes = async () => {
  loading.value = true
  try {
    const { getDyeRecipeList } = await import('@/api/dye-recipe')
    const res = await getDyeRecipeList()
    recipes.value = (res.data as DyeRecipe[] | undefined) || []
  } catch (error) {
    const err = error as Error
    logger.error('获取配方列表失败', err.message)
  } finally {
    loading.value = false
  }
}

const openCreate = () => emit('openDialog', null)

const handleView = async (row: DyeRecipe) => {
  try {
    const res = await getDyeRecipe(row.id)
    emit('openDialog', (res.data as DyeRecipe | undefined) || null)
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || t('fabric.recipeTab.messageGetDetailFailed'))
  }
}

const handleApprove = async (row: DyeRecipe) => {
  try {
    await ElMessageBox.confirm(t('fabric.recipeTab.confirmApproveContent'), t('fabric.common.confirmTitle'), { type: 'info' })
    await approveDyeRecipe(row.id)
    ElMessage.success(t('fabric.recipeTab.messageApproveSuccess'))
    fetchRecipes()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || t('fabric.common.failed'))
    }
  }
}

const handleNewVersion = async (row: DyeRecipe) => {
  try {
    await ElMessageBox.confirm(t('fabric.recipeTab.confirmNewVersionContent'), t('fabric.common.confirmTitle'), { type: 'info' })
    await createNewVersionApi(row.id)
    ElMessage.success(t('fabric.recipeTab.messageNewVersionSuccess'))
    fetchRecipes()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || t('fabric.common.failed'))
    }
  }
}

onMounted(() => fetchRecipes())

defineExpose({ fetchRecipes })
</script>

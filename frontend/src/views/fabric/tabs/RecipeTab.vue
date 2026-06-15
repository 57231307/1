<!--
  RecipeTab.vue - 染色配方 Tab
  来源：原 fabric/index.vue 中 染色配方 tab 内容
  拆分日期：2026-06-15 B3-4
-->
<template>
  <div class="recipe-tab">
    <div class="page-header">
      <h2 class="page-title">染色配方管理</h2>
      <el-button type="primary" @click="openCreate">
        <el-icon><Plus /></el-icon>
        新建配方
      </el-button>
    </div>

    <el-card shadow="hover">
      <el-table v-loading="loading" :data="recipes" stripe>
        <el-table-column prop="recipe_no" label="配方号" width="120" />
        <el-table-column prop="recipe_name" label="名称" width="150" />
        <el-table-column prop="color_name" label="颜色" width="120" />
        <el-table-column prop="fabric_type" label="面料类型" width="120" />
        <el-table-column prop="version" label="版本" width="80" />
        <el-table-column prop="status" label="状态" width="100" align="center">
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
        <el-table-column prop="created_at" label="创建时间" width="160" />
        <el-table-column label="操作" width="240" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleView(row)">查看</el-button>
            <el-button
              v-if="row.status === 'draft'"
              type="success"
              link
              size="small"
              @click="handleApprove(row)"
              >审批</el-button
            >
            <el-button
              v-if="row.status === 'approved'"
              type="warning"
              link
              size="small"
              @click="handleNewVersion(row)"
              >新版本</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, defineEmits } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  getDyeRecipe,
  approveDyeRecipe,
  createNewVersion as createNewVersionApi,
  type DyeRecipe,
} from '@/api/dye-recipe'
import { logger } from '@/utils/logger'

const emit = defineEmits<{ openDialog: [row: DyeRecipe | null] }>()

const recipes = ref<DyeRecipe[]>([])
const loading = ref(false)

const getStatusLabel = (status: string) => {
  const map: Record<string, string> = { draft: '草稿', approved: '已审批', obsolete: '已作废' }
  return map[status] || status
}

const fetchRecipes = async () => {
  loading.value = true
  try {
    const { listDyeRecipes } = await import('@/api/dye-recipe')
    const res = await listDyeRecipes()
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
    ElMessage.error(err.message || '获取配方详情失败')
  }
}

const handleApprove = async (row: DyeRecipe) => {
  try {
    await ElMessageBox.confirm('确定审批此配方吗？', '确认', { type: 'info' })
    await approveDyeRecipe(row.id)
    ElMessage.success('审批成功')
    fetchRecipes()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || '操作失败')
    }
  }
}

const handleNewVersion = async (row: DyeRecipe) => {
  try {
    await ElMessageBox.confirm('确定创建新版本吗？', '确认', { type: 'info' })
    await createNewVersionApi(row.id)
    ElMessage.success('创建成功')
    fetchRecipes()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || '操作失败')
    }
  }
}

onMounted(() => fetchRecipes())

defineExpose({ fetchRecipes })
</script>

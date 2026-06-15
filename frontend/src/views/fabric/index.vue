<!--
  fabric/index.vue - 面料管理主入口（容器组件）
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-4）：
  原 729 行"上帝组件"已拆分为以下 3 个 Tab 子组件 + 3 个对话框，
  位于 views/fabric/tabs/ 目录：

  | Tab         | 子组件                              |
  | ----------- | ----------------------------------- |
  | 染色批次    | tabs/DyeTab.vue                     |
  | 坯布管理    | tabs/GreigeTab.vue                  |
  | 染色配方    | tabs/RecipeTab.vue                  |
  | 染色编辑    | tabs/DyeFormDialogTab.vue           |
  | 坯布编辑    | tabs/GreigeFormDialogTab.vue        |
  | 配方编辑    | tabs/RecipeFormDialogTab.vue        |

  本主入口仅承担：Tab 切换 + 公共样式。
-->
<template>
  <div class="fabric-page">
    <el-tabs v-model="activeTab">
      <el-tab-pane label="染色批次" name="dye">
        <DyeTab @open-dialog="openDyeDialog" />
      </el-tab-pane>
      <el-tab-pane label="坯布管理" name="greige">
        <GreigeTab @open-dialog="openGreigeDialog" @open-stock="handleStock" />
      </el-tab-pane>
      <el-tab-pane label="染色配方" name="recipe">
        <RecipeTab @open-dialog="openRecipeDialog" />
      </el-tab-pane>
    </el-tabs>

    <DyeFormDialogTab
      v-model="dyeDialogVisible"
      :current-row="currentDyeRow"
      :greige-fabrics="greigeFabrics"
      @submitted="handleSubmitted"
    />

    <GreigeFormDialogTab
      v-model="greigeDialogVisible"
      :current-row="currentGreigeRow"
      :suppliers="suppliers"
      @submitted="handleSubmitted"
    />

    <RecipeFormDialogTab
      v-model="recipeDialogVisible"
      :current-row="currentRecipeRow"
      @submitted="handleSubmitted"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, provide } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { stockInGreigeFabric, stockOutGreigeFabric, type GreigeFabric } from '@/api/greige-fabric'
import { listSuppliers, type Supplier } from '@/api/supplier'
import type { DyeBatch } from '@/api/dye-batch'
import type { DyeRecipe as ApiDyeRecipe } from '@/api/dye-recipe'
import { logger } from '@/utils/logger'
import DyeTab from './tabs/DyeTab.vue'
import GreigeTab from './tabs/GreigeTab.vue'
import RecipeTab from './tabs/RecipeTab.vue'
import DyeFormDialogTab from './tabs/DyeFormDialogTab.vue'
import GreigeFormDialogTab from './tabs/GreigeFormDialogTab.vue'
import RecipeFormDialogTab from './tabs/RecipeFormDialogTab.vue'

const activeTab = ref('dye')

const dyeDialogVisible = ref(false)
const currentDyeRow = ref<DyeBatch | null>(null)
const greigeFabrics = ref<GreigeFabric[]>([])

const greigeDialogVisible = ref(false)
const currentGreigeRow = ref<GreigeFabric | null>(null)
const suppliers = ref<Supplier[]>([])

const recipeDialogVisible = ref(false)
const currentRecipeRow = ref<ApiDyeRecipe | null>(null)

const fetchGreigeFabrics = async () => {
  try {
    const { listGreigeFabrics } = await import('@/api/greige-fabric')
    const res = await listGreigeFabrics()
    greigeFabrics.value = (res.data as GreigeFabric[] | undefined) || []
  } catch (error) {
    const err = error as Error
    logger.error('获取坯布列表失败', err.message)
  }
}

const fetchSuppliers = async () => {
  try {
    const res = await listSuppliers()
    suppliers.value = (res.data?.list as Supplier[] | undefined) || []
  } catch (error) {
    const err = error as Error
    logger.error('获取供应商失败', err.message)
  }
}

const openDyeDialog = (row: DyeBatch | null) => {
  currentDyeRow.value = row
  dyeDialogVisible.value = true
  fetchGreigeFabrics()
}

const openGreigeDialog = (row: GreigeFabric | null) => {
  currentGreigeRow.value = row
  greigeDialogVisible.value = true
  fetchSuppliers()
}

const openRecipeDialog = (row: ApiDyeRecipe | null) => {
  currentRecipeRow.value = row
  recipeDialogVisible.value = true
}

const handleStock = async (type: 'in' | 'out', row: GreigeFabric) => {
  const title = type === 'in' ? '坯布入库' : '坯布出库'
  try {
    const { value } = await ElMessageBox.prompt(
      `请输入${type === 'in' ? '入库' : '出库'}数量`,
      title
    )
    const qty = parseFloat(value)
    if (isNaN(qty) || qty <= 0) {
      ElMessage.error('数量必须为正数')
      return
    }
    if (type === 'in') {
      await stockInGreigeFabric(row.id, { quantity: qty })
    } else {
      await stockOutGreigeFabric(row.id, { quantity: qty })
    }
    ElMessage.success(type === 'in' ? '入库成功' : '出库成功')
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || '操作失败')
    }
  }
}

const handleSubmitted = () => {
  // 各 Tab 内部已通过 emit 触发刷新
}

provide('fabricActions', {
  openDyeDialog,
  openGreigeDialog,
  openRecipeDialog,
})
</script>

<style scoped>
.fabric-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
:deep(.page-header) {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
:deep(.page-title) {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
</style>

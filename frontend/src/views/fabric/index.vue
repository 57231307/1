<template>
  <div class="fabric-page">
    <el-tabs v-model="activeTab" @tab-change="(tab) => loadTab(tab, hasLoaded)">
      <el-tab-pane label="染色批次" name="dye">
        <div class="page-header">
          <h2 class="page-title">染色批次管理</h2>
          <el-button type="primary" @click="openDyeDialog">
            <el-icon><Plus /></el-icon>
            新建批次
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table v-loading="dyeLoading" :data="dyeBatches" stripe>
            <el-table-column prop="batch_no" label="批次号" width="140" />
            <el-table-column prop="color_name" label="颜色" width="120" />
            <el-table-column prop="greige_fabric_name" label="坯布" width="150" />
            <el-table-column prop="planned_quantity" label="计划数量" width="100" align="right" />
            <el-table-column prop="actual_quantity" label="实际数量" width="100" align="right" />
            <el-table-column prop="status" label="状态" width="100" align="center">
              <template #default="{ row }">
                <el-tag :type="getStatusType(row.status)" size="small">
                  {{ getStatusLabel(row.status) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="start_date" label="开始日期" width="120" />
            <el-table-column label="操作" width="200" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="openDyeDialog(row)"
                  >编辑</el-button
                >
                <el-button
                  v-if="row.status === 'in_progress'"
                  type="success"
                  link
                  size="small"
                  @click="completeDye(row)"
                  >完成</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="坯布管理" name="greige">
        <div class="page-header">
          <h2 class="page-title">坯布管理</h2>
          <el-button type="primary" @click="openGreigeDialog">
            <el-icon><Plus /></el-icon>
            新建坯布
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table v-loading="greigeLoading" :data="greigeFabrics" stripe>
            <el-table-column prop="fabric_code" label="编号" width="120" />
            <el-table-column prop="fabric_name" label="名称" min-width="150" />
            <el-table-column prop="supplier_name" label="供应商" width="150" />
            <el-table-column prop="width" label="幅宽" width="80" />
            <el-table-column prop="weight" label="克重" width="80" />
            <el-table-column prop="composition" label="成分" width="120" />
            <el-table-column prop="quantity" label="库存" width="100" align="right" />
            <el-table-column prop="status" label="状态" width="80" align="center">
              <template #default="{ row }">
                <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
                  {{ row.status === 'active' ? '正常' : '停用' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="180" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="openGreigeDialog(row)"
                  >编辑</el-button
                >
                <el-button type="success" link size="small" @click="openStockInDialog(row)"
                  >入库</el-button
                >
                <el-button type="warning" link size="small" @click="openStockOutDialog(row)"
                  >出库</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="染色配方" name="recipe">
        <div class="page-header">
          <h2 class="page-title">染色配方管理</h2>
          <el-button type="primary" @click="openRecipeDialog">
            <el-icon><Plus /></el-icon>
            新建配方
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table v-loading="recipeLoading" :data="dyeRecipes" stripe>
            <el-table-column prop="recipe_no" label="配方号" width="120" />
            <el-table-column prop="recipe_name" label="名称" width="150" />
            <el-table-column prop="color_name" label="颜色" width="120" />
            <el-table-column prop="fabric_type" label="面料类型" width="120" />
            <el-table-column prop="version" label="版本" width="80" />
            <el-table-column prop="status" label="状态" width="100" align="center">
              <template #default="{ row }">
                <el-tag
                  :type="
                    row.status === 'approved'
                      ? 'success'
                      : row.status === 'draft'
                        ? 'info'
                        : 'danger'
                  "
                  size="small"
                >
                  {{ getRecipeStatusLabel(row.status) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="created_at" label="创建时间" width="160" />
            <el-table-column label="操作" width="200" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="viewRecipe(row)"
                  >查看</el-button
                >
                <el-button
                  v-if="row.status === 'draft'"
                  type="success"
                  link
                  size="small"
                  @click="approveRecipe(row)"
                  >审批</el-button
                >
                <el-button
                  v-if="row.status === 'approved'"
                  type="warning"
                  link
                  size="small"
                  @click="createNewVersion(row)"
                  >新版本</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>
    </el-tabs>

    <el-dialog
      v-model="dyeDialogVisible"
      :title="dyeForm.id ? '编辑染色批次' : '新建染色批次'"
      width="600px"
    >
      <el-form ref="dyeFormRef" :model="dyeForm" label-width="100px">
        <el-form-item label="批次号" prop="batch_no">
          <el-input v-model="dyeForm.batch_no" :disabled="!!dyeForm.id" />
        </el-form-item>
        <el-form-item label="颜色" prop="color_name">
          <el-input v-model="dyeForm.color_name" />
        </el-form-item>
        <el-form-item label="坯布" prop="greige_fabric_id">
          <el-select v-model="dyeForm.greige_fabric_id" placeholder="选择坯布" style="width: 100%">
            <el-option
              v-for="g in greigeFabrics"
              :key="g.id"
              :label="g.fabric_name"
              :value="g.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="计划数量" prop="planned_quantity">
          <el-input-number v-model="dyeForm.planned_quantity" style="width: 100%" />
        </el-form-item>
        <el-form-item label="开始日期" prop="start_date">
          <el-date-picker
            v-model="dyeForm.start_date"
            type="date"
            style="width: 100%"
            value-format="YYYY-MM-DD"
          />
        </el-form-item>
        <el-form-item label="染色机台" prop="machine_code">
          <el-input v-model="dyeForm.machine_code" />
        </el-form-item>
        <el-form-item label="备注" prop="remark">
          <el-input v-model="dyeForm.remark" type="textarea" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dyeDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="dyeSubmitLoading" @click="submitDye">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="greigeDialogVisible"
      :title="greigeForm.id ? '编辑坯布' : '新建坯布'"
      width="600px"
    >
      <el-form ref="greigeFormRef" :model="greigeForm" label-width="100px">
        <el-form-item label="编号" prop="fabric_code">
          <el-input v-model="greigeForm.fabric_code" :disabled="!!greigeForm.id" />
        </el-form-item>
        <el-form-item label="名称" prop="fabric_name">
          <el-input v-model="greigeForm.fabric_name" />
        </el-form-item>
        <el-form-item label="供应商" prop="supplier_id">
          <el-select v-model="greigeForm.supplier_id" placeholder="选择供应商" style="width: 100%">
            <el-option v-for="s in suppliers" :key="s.id" :label="s.supplier_name" :value="s.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="幅宽" prop="width">
          <el-input-number v-model="greigeForm.width" style="width: 100%" />
        </el-form-item>
        <el-form-item label="克重" prop="weight">
          <el-input-number v-model="greigeForm.weight" style="width: 100%" />
        </el-form-item>
        <el-form-item label="成分" prop="composition">
          <el-input v-model="greigeForm.composition" />
        </el-form-item>
        <el-form-item label="描述" prop="description">
          <el-input v-model="greigeForm.description" type="textarea" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="greigeDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="greigeSubmitLoading" @click="submitGreige"
          >确定</el-button
        >
      </template>
    </el-dialog>

    <el-dialog
      v-model="recipeDialogVisible"
      :title="recipeForm.id ? '编辑配方' : '新建配方'"
      width="700px"
    >
      <el-form ref="recipeFormRef" :model="recipeForm" label-width="100px">
        <el-form-item label="配方号" prop="recipe_no">
          <el-input v-model="recipeForm.recipe_no" :disabled="!!recipeForm.id" />
        </el-form-item>
        <el-form-item label="名称" prop="recipe_name">
          <el-input v-model="recipeForm.recipe_name" />
        </el-form-item>
        <el-form-item label="颜色" prop="color_name">
          <el-input v-model="recipeForm.color_name" />
        </el-form-item>
        <el-form-item label="面料类型" prop="fabric_type">
          <el-input v-model="recipeForm.fabric_type" />
        </el-form-item>
        <el-form-item label="配方明细">
          <el-table :data="recipeForm.recipe_items" border style="width: 100%">
            <el-table-column prop="chemical_name" label="化学品" width="150">
              <template #default="{ row }">
                <el-input v-model="row.chemical_name" />
              </template>
            </el-table-column>
            <el-table-column prop="dosage" label="用量" width="120">
              <template #default="{ row }">
                <el-input-number v-model="row.dosage" style="width: 100%" />
              </template>
            </el-table-column>
            <el-table-column prop="dosage_unit" label="单位" width="80">
              <template #default="{ row }">
                <el-input v-model="row.dosage_unit" />
              </template>
            </el-table-column>
            <el-table-column label="操作" width="80">
              <template #default="{ $index }">
                <el-button type="danger" link size="small" @click="removeRecipeItem($index)"
                  >删除</el-button
                >
              </template>
            </el-table-column>
          </el-table>
          <el-button type="primary" link style="margin-top: 8px" @click="addRecipeItem"
            >添加化学品</el-button
          >
        </el-form-item>
        <el-form-item label="工艺参数" prop="process_parameters">
          <el-input v-model="processParamsText" type="textarea" placeholder="JSON格式" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="recipeDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="recipeSubmitLoading" @click="submitRecipe"
          >确定</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import type { FormInstance } from 'element-plus'
import {
  listDyeBatches,
  createDyeBatch,
  updateDyeBatch,
  completeDyeBatch,
  type DyeBatch,
} from '@/api/dye-batch'
import {
  listGreigeFabrics,
  createGreigeFabric,
  updateGreigeFabric,
  stockInGreigeFabric,
  stockOutGreigeFabric,
  type GreigeFabric,
} from '@/api/greige-fabric'
import {
  listDyeRecipes,
  getDyeRecipe,
  createDyeRecipe,
  updateDyeRecipe,
  approveDyeRecipe,
  createNewVersion as createNewVersionApi,
  type DyeRecipe,
} from '@/api/dye-recipe'
import { listSuppliers, type Supplier } from '@/api/supplier'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'

const activeTab = ref('dye')
const hasLoaded = createLazyLoader()
const dyeBatches = ref<DyeBatch[]>([])
const greigeFabrics = ref<GreigeFabric[]>([])
const dyeRecipes = ref<DyeRecipe[]>([])
const suppliers = ref<Supplier[]>([])
const dyeLoading = ref(false)
const greigeLoading = ref(false)
const recipeLoading = ref(false)

const fetchDyeBatches = async () => {
  dyeLoading.value = true
  try {
    const res = await listDyeBatches()
    dyeBatches.value = res.data! || []
  } finally {
    dyeLoading.value = false
  }
}

const fetchGreigeFabrics = async () => {
  greigeLoading.value = true
  try {
    const res = await listGreigeFabrics()
    greigeFabrics.value = res.data! || []
  } finally {
    greigeLoading.value = false
  }
}

const fetchDyeRecipes = async () => {
  recipeLoading.value = true
  try {
    const res = await listDyeRecipes()
    dyeRecipes.value = res.data! || []
  } finally {
    recipeLoading.value = false
  }
}

const fetchSuppliers = async () => {
  try {
    const res = await listSuppliers()
    suppliers.value = res.data!.list || []
  } catch (e) {
    console.error(e)
  }
}

const getStatusType = (status: string) => {
  const map: Record<string, any> = {
    pending: 'info',
    in_progress: 'warning',
    completed: 'success',
    cancelled: 'danger',
  }
  return map[status] || 'info'
}

const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    pending: '待处理',
    in_progress: '进行中',
    completed: '已完成',
    cancelled: '已取消',
  }
  return map[status] || status
}

const getRecipeStatusLabel = (status: string) => {
  const map: Record<string, string> = { draft: '草稿', approved: '已审批', obsolete: '已作废' }
  return map[status] || status
}

const dyeDialogVisible = ref(false)
const dyeFormRef = ref<FormInstance>()
const dyeSubmitLoading = ref(false)
const dyeForm = reactive({
  id: 0,
  batch_no: '',
  color_name: '',
  greige_fabric_id: undefined as number | undefined,
  planned_quantity: 0,
  actual_quantity: 0,
  unit: '米',
  status: 'pending',
  start_date: '',
  end_date: '',
  machine_code: '',
  operator: '',
  remark: '',
})

const openDyeDialog = async (row?: DyeBatch) => {
  if (row) {
    Object.assign(dyeForm, row)
  } else {
    Object.assign(dyeForm, {
      id: 0,
      batch_no: '',
      color_name: '',
      greige_fabric_id: undefined,
      planned_quantity: 0,
      actual_quantity: 0,
      unit: '米',
      status: 'pending',
      start_date: '',
      end_date: '',
      machine_code: '',
      operator: '',
      remark: '',
    })
  }
  dyeDialogVisible.value = true
}

const submitDye = async () => {
  dyeSubmitLoading.value = true
  try {
    if (dyeForm.id) {
      await updateDyeBatch(dyeForm.id, dyeForm as Partial<DyeBatch>)
    } else {
      await createDyeBatch(dyeForm as Partial<DyeBatch>)
    }
    ElMessage.success('操作成功')
    dyeDialogVisible.value = false
    fetchDyeBatches()
  } catch (e: any) {
    ElMessage.error(e.message || '操作失败')
  } finally {
    dyeSubmitLoading.value = false
  }
}

const completeDye = async (row: DyeBatch) => {
  try {
    await ElMessageBox.confirm('确定完成此批次吗？', '确认', { type: 'info' })
    await completeDyeBatch(row.id)
    ElMessage.success('操作成功')
    fetchDyeBatches()
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(e.message || '操作失败')
  }
}

const greigeDialogVisible = ref(false)
const greigeFormRef = ref<FormInstance>()
const greigeSubmitLoading = ref(false)
const greigeForm = reactive({
  id: 0,
  fabric_code: '',
  fabric_name: '',
  fabric_type: '',
  supplier_id: undefined as number | undefined,
  width: 0,
  weight: 0,
  unit: '米',
  composition: '',
  quantity: 0,
  min_order_quantity: 0,
  status: 'active',
  description: '',
})

const openGreigeDialog = async (row?: GreigeFabric) => {
  if (row) {
    Object.assign(greigeForm, row)
  } else {
    Object.assign(greigeForm, {
      id: 0,
      fabric_code: '',
      fabric_name: '',
      fabric_type: '',
      supplier_id: undefined,
      width: 0,
      weight: 0,
      unit: '米',
      composition: '',
      quantity: 0,
      min_order_quantity: 0,
      status: 'active',
      description: '',
    })
  }
  greigeDialogVisible.value = true
}

const submitGreige = async () => {
  greigeSubmitLoading.value = true
  try {
    if (greigeForm.id) {
      await updateGreigeFabric(greigeForm.id, greigeForm as Partial<GreigeFabric>)
    } else {
      await createGreigeFabric(greigeForm as Partial<GreigeFabric>)
    }
    ElMessage.success('操作成功')
    greigeDialogVisible.value = false
    fetchGreigeFabrics()
  } catch (e: any) {
    ElMessage.error(e.message || '操作失败')
  } finally {
    greigeSubmitLoading.value = false
  }
}

const openStockInDialog = (row: GreigeFabric) => {
  ElMessageBox.prompt('请输入入库数量', '坯布入库')
    .then(async ({ value }) => {
      const qty = parseFloat(value)
      if (!isNaN(qty) && qty > 0) {
        await stockInGreigeFabric(row.id, { quantity: qty })
        ElMessage.success('入库成功')
        fetchGreigeFabrics()
      } else {
        ElMessage.error('入库数量必须为正数')
      }
    })
    .catch((e: any) => {
      if (e !== 'cancel') ElMessage.error(e?.message || '坯布入库失败')
    })
}

const openStockOutDialog = (row: GreigeFabric) => {
  ElMessageBox.prompt('请输入出库数量', '坯布出库')
    .then(async ({ value }) => {
      const qty = parseFloat(value)
      if (!isNaN(qty) && qty > 0) {
        await stockOutGreigeFabric(row.id, { quantity: qty })
        ElMessage.success('出库成功')
        fetchGreigeFabrics()
      } else {
        ElMessage.error('出库数量必须为正数')
      }
    })
    .catch((e: any) => {
      if (e !== 'cancel') ElMessage.error(e?.message || '坯布出库失败')
    })
}

const recipeDialogVisible = ref(false)
const recipeFormRef = ref<FormInstance>()
const recipeSubmitLoading = ref(false)
const processParamsText = ref('')
const recipeForm = reactive({
  id: 0,
  recipe_no: '',
  recipe_name: '',
  color_code: '',
  color_name: '',
  fabric_type: '',
  version: 1 as number,
  status: 'draft' as const,
  recipe_items: [] as any[],
  process_parameters: {} as Record<string, any>,
})

const openRecipeDialog = async (row?: DyeRecipe) => {
  if (row) {
    Object.assign(recipeForm, { ...row, version: Number(row.version) || 1 })
    processParamsText.value = JSON.stringify(row.process_parameters, null, 2)
  } else {
    Object.assign(recipeForm, {
      id: 0,
      recipe_no: '',
      recipe_name: '',
      color_code: '',
      color_name: '',
      fabric_type: '',
      version: 1,
      status: 'draft' as const,
      recipe_items: [
        {
          id: 0,
          chemical_name: '',
          chemical_code: '',
          dosage: 0,
          dosage_unit: 'g/l',
          sequence: 1,
          remark: '',
        },
      ],
      process_parameters: {},
    })
    processParamsText.value = ''
  }
  recipeDialogVisible.value = true
}

const viewRecipe = async (row: DyeRecipe) => {
  const res = await getDyeRecipe(row.id)
  openRecipeDialog(res.data!)
}

const submitRecipe = async () => {
  recipeSubmitLoading.value = true
  try {
    try {
      recipeForm.process_parameters = processParamsText.value
        ? JSON.parse(processParamsText.value)
        : {}
    } catch (e) {
      ElMessage.error('工艺参数格式错误')
      return
    }
    if (recipeForm.id) {
      await updateDyeRecipe(recipeForm.id, recipeForm as Partial<DyeRecipe>)
    } else {
      await createDyeRecipe(recipeForm as Partial<DyeRecipe>)
    }
    ElMessage.success('操作成功')
    recipeDialogVisible.value = false
    fetchDyeRecipes()
  } catch (e: any) {
    ElMessage.error(e.message || '操作失败')
  } finally {
    recipeSubmitLoading.value = false
  }
}

const approveRecipe = async (row: DyeRecipe) => {
  try {
    await ElMessageBox.confirm('确定审批此配方吗？', '确认', { type: 'info' })
    await approveDyeRecipe(row.id)
    ElMessage.success('审批成功')
    fetchDyeRecipes()
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(e.message || '操作失败')
  }
}

const createNewVersion = async (row: DyeRecipe) => {
  try {
    await ElMessageBox.confirm('确定创建新版本吗？', '确认', { type: 'info' })
    await createNewVersionApi(row.id)
    ElMessage.success('创建成功')
    fetchDyeRecipes()
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(e.message || '操作失败')
  }
}

const addRecipeItem = () => {
  const len = recipeForm.recipe_items.length
  recipeForm.recipe_items.push({
    id: 0,
    chemical_name: '',
    chemical_code: '',
    dosage: 0,
    dosage_unit: 'g/l',
    sequence: len + 1,
    remark: '',
  })
}

const removeRecipeItem = (index: number) => {
  if (recipeForm.recipe_items.length > 1) {
    recipeForm.recipe_items.splice(index, 1)
  }
}

const loadTab = (tabName: string, loader: Record<string, () => void>) => {
  loadIfNot(tabName, loader[tabName], hasLoaded)
}

const initPage = () => {
  loadTab(activeTab.value, {
    dye: fetchDyeBatches,
    greige: fetchGreigeFabrics,
    recipe: fetchDyeRecipes,
  })
}

onMounted(() => {
  initPage()
})
</script>

<style scoped>
.fabric-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.page-title {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
</style>

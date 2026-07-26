<template>
  <div class="greige-fabrics-page">
    <div class="header">
      <h2>{{ t('greigeFabrics.index.pageTitle') }}</h2>
      <el-button type="primary" @click="handleCreate">{{
        t('greigeFabrics.index.buttonCreate')
      }}</el-button>
    </div>

    <el-table
      v-loading="loading"
      :data="greigeList"
      border
      :aria-label="t('greigeFabrics.index.ariaTable')"
    >
      <el-table-column
        prop="fabric_code"
        :label="t('greigeFabrics.index.colFabricCode')"
        min-width="120"
      />
      <el-table-column
        prop="fabric_name"
        :label="t('greigeFabrics.index.colFabricName')"
        min-width="120"
      />
      <el-table-column
        prop="fabric_type"
        :label="t('greigeFabrics.index.colFabricType')"
        width="100"
      />
      <el-table-column
        prop="supplier_name"
        :label="t('greigeFabrics.index.colSupplier')"
        width="120"
      />
      <el-table-column
        prop="quantity"
        :label="t('greigeFabrics.index.colQuantity')"
        width="100"
        align="right"
      />
      <el-table-column prop="unit" :label="t('greigeFabrics.index.colUnit')" width="80" />
      <el-table-column prop="status" :label="t('greigeFabrics.index.colStatus')" width="100">
        <template #default="{ row }">
          <el-tag :type="row.status === 'active' ? 'success' : 'info'">
            {{
              row.status === 'active'
                ? t('greigeFabrics.index.optionActive')
                : t('greigeFabrics.index.optionInactive')
            }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('greigeFabrics.index.colOperation')" width="200">
        <template #default="{ row }">
          <el-button size="small" @click="handleEdit(row)">{{
            t('greigeFabrics.index.buttonEdit')
          }}</el-button>
          <el-button size="small" type="danger" @click="handleDelete(row)">{{
            t('greigeFabrics.index.buttonDelete')
          }}</el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- 新建/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="
        dialogMode === 'create'
          ? t('greigeFabrics.index.titleCreate')
          : t('greigeFabrics.index.titleEdit')
      "
      width="600px"
      :aria-label="t('greigeFabrics.index.ariaEditDialog')"
      @close="handleDialogClose"
    >
      <el-form
        ref="formRef"
        :model="formData"
        :rules="formRules"
        label-width="100px"
        :aria-label="t('greigeFabrics.index.ariaForm')"
      >
        <el-form-item :label="t('greigeFabrics.index.colFabricCode')" prop="fabric_code">
          <el-input
            v-model="formData.fabric_code"
            :placeholder="t('greigeFabrics.index.placeholderFabricCode')"
          />
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.index.colFabricName')" prop="fabric_name">
          <el-input
            v-model="formData.fabric_name"
            :placeholder="t('greigeFabrics.index.placeholderFabricName')"
          />
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.index.colFabricType')" prop="fabric_type">
          <el-input
            v-model="formData.fabric_type"
            :placeholder="t('greigeFabrics.index.placeholderFabricType')"
          />
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.index.colSupplier')" prop="supplier_id">
          <el-select
            v-model="formData.supplier_id"
            :placeholder="t('greigeFabrics.index.placeholderSelectSupplier')"
            filterable
          >
            <el-option
              v-for="s in supplierList"
              :key="s.id"
              :label="s.supplier_name"
              :value="s.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.index.labelWidth')" prop="width">
          <el-input-number
            v-model="formData.width"
            :min="0"
            :precision="2"
            :placeholder="t('greigeFabrics.index.placeholderWidth')"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.index.labelWeight')" prop="weight">
          <el-input-number
            v-model="formData.weight"
            :min="0"
            :precision="2"
            :placeholder="t('greigeFabrics.index.placeholderWeight')"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.index.colUnit')" prop="unit">
          <el-select
            v-model="formData.unit"
            :placeholder="t('greigeFabrics.index.placeholderSelectUnit')"
          >
            <el-option :label="t('greigeFabrics.index.optionMeter')" value="m" />
            <el-option :label="t('greigeFabrics.index.optionYard')" value="yd" />
            <el-option :label="t('greigeFabrics.index.optionKg')" value="kg" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.index.labelComposition')" prop="composition">
          <el-input
            v-model="formData.composition"
            :placeholder="t('greigeFabrics.index.placeholderComposition')"
          />
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.index.colQuantity')" prop="quantity">
          <el-input-number
            v-model="formData.quantity"
            :min="0"
            :precision="2"
            :placeholder="t('greigeFabrics.index.placeholderQuantity')"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item
          :label="t('greigeFabrics.index.labelMinOrderQuantity')"
          prop="min_order_quantity"
        >
          <el-input-number
            v-model="formData.min_order_quantity"
            :min="0"
            :precision="2"
            :placeholder="t('greigeFabrics.index.placeholderMinOrderQuantity')"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.index.colStatus')" prop="status">
          <el-select
            v-model="formData.status"
            :placeholder="t('greigeFabrics.index.placeholderSelectStatus')"
          >
            <el-option :label="t('greigeFabrics.index.optionActive')" value="active" />
            <el-option :label="t('greigeFabrics.index.optionInactive')" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('greigeFabrics.index.labelDescription')" prop="description">
          <el-input v-model="formData.description" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="dialogVisible = false">{{
          t('greigeFabrics.index.buttonCancel')
        }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">
          {{ t('greigeFabrics.index.buttonConfirm') }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import {
  getGreigeFabricList,
  createGreigeFabric,
  updateGreigeFabric,
  deleteGreigeFabric,
  type GreigeFabric,
} from '@/api/greige-fabric'
import { getSupplierList, type Supplier } from '@/api/supplier'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'

const { t } = useI18n({ useScope: 'global' })

const loading = ref(false)
const submitLoading = ref(false)
const dialogVisible = ref(false)
const dialogMode = ref<'create' | 'edit'>('create')
const formRef = ref<FormInstance>()
// v11 批次 181 P2-1 修复：使用 API 的 GreigeFabric 类型，替代本地 GreigeFabricRow
const greigeList = ref<GreigeFabric[]>([])
// v11 批次 181 P2-1 修复：使用 Supplier 类型，替代本地 WarehouseOption
const supplierList = ref<Supplier[]>([])

// 表单数据类型，与 GreigeFabric 对齐（id 创建时为空，编辑时由后端返回）
interface GreigeFabricForm {
  id?: number
  fabric_code: string
  fabric_name: string
  fabric_type: string
  supplier_id: number
  supplier_name: string
  width: number
  weight: number
  unit: string
  composition: string
  quantity: number
  min_order_quantity: number
  status: 'active' | 'inactive'
  description: string
}

const formData = reactive<GreigeFabricForm>({
  fabric_code: '',
  fabric_name: '',
  fabric_type: '',
  supplier_id: 0,
  supplier_name: '',
  width: 0,
  weight: 0,
  unit: 'm',
  composition: '',
  quantity: 0,
  min_order_quantity: 0,
  status: 'active',
  description: '',
})

const formRules: FormRules = {
  fabric_code: [
    { required: true, message: t('greigeFabrics.index.ruleFabricCodeRequired'), trigger: 'blur' },
  ],
  fabric_name: [
    { required: true, message: t('greigeFabrics.index.ruleFabricNameRequired'), trigger: 'blur' },
  ],
  fabric_type: [
    { required: true, message: t('greigeFabrics.index.ruleFabricTypeRequired'), trigger: 'blur' },
  ],
  supplier_id: [
    { required: true, message: t('greigeFabrics.index.ruleSupplierRequired'), trigger: 'change' },
  ],
  unit: [{ required: true, message: t('greigeFabrics.index.ruleUnitRequired'), trigger: 'change' }],
  status: [
    { required: true, message: t('greigeFabrics.index.ruleStatusRequired'), trigger: 'change' },
  ],
}

const loadGreigeFabrics = async () => {
  loading.value = true
  try {
    const res = await getGreigeFabricList()
    // v11 批次 181 P2-1 修复：API 返回 GreigeFabric[]，前端直接使用，无需类型转换
    greigeList.value = res.data || []
  } catch (error) {
    ElMessage.error(t('greigeFabrics.index.messageLoadListFailed'))
  } finally {
    loading.value = false
  }
}

const loadSuppliers = async () => {
  try {
    // v11 批次 181 P2-1 修复：使用 supplier API 替代 warehouse API
    // getSupplierList 返回 { list: Supplier[]; total: number }，提取 list 字段
    const res = await getSupplierList()
    const d = res.data
    if (d && typeof d === 'object' && 'list' in d) {
      supplierList.value = d.list || []
    } else {
      supplierList.value = []
    }
  } catch (error) {
    ElMessage.error(t('greigeFabrics.index.messageLoadSuppliersFailed'))
  }
}

const resetForm = () => {
  Object.assign(formData, {
    id: undefined,
    fabric_code: '',
    fabric_name: '',
    fabric_type: '',
    supplier_id: 0,
    supplier_name: '',
    width: 0,
    weight: 0,
    unit: 'm',
    composition: '',
    quantity: 0,
    min_order_quantity: 0,
    status: 'active',
    description: '',
  })
}

const handleCreate = () => {
  dialogMode.value = 'create'
  resetForm()
  dialogVisible.value = true
}

const handleEdit = (row: GreigeFabric) => {
  dialogMode.value = 'edit'
  // 同步供应商名称展示
  const supplier = supplierList.value.find(s => s.id === row.supplier_id)
  Object.assign(formData, {
    id: row.id,
    fabric_code: row.fabric_code,
    fabric_name: row.fabric_name,
    fabric_type: row.fabric_type,
    supplier_id: row.supplier_id,
    supplier_name: supplier?.supplier_name || row.supplier_name || '',
    width: row.width,
    weight: row.weight,
    unit: row.unit,
    composition: row.composition,
    quantity: row.quantity,
    min_order_quantity: row.min_order_quantity,
    status: row.status,
    description: row.description,
  })
  dialogVisible.value = true
}

const handleDelete = async (row: GreigeFabric) => {
  if (!row.id) return

  try {
    await deleteGreigeFabric(row.id)
    ElMessage.success(t('greigeFabrics.index.messageDeleteSuccess'))
    await loadGreigeFabrics()
  } catch (error) {
    ElMessage.error(t('greigeFabrics.index.messageDeleteFailed'))
  }
}

const handleSubmit = async () => {
  if (!formRef.value) return

  await formRef.value.validate(async (valid: boolean) => {
    if (!valid) return

    submitLoading.value = true
    try {
      // 同步 supplier_name 到表单数据
      const supplier = supplierList.value.find(s => s.id === formData.supplier_id)
      if (supplier) {
        formData.supplier_name = supplier.supplier_name
      }

      if (dialogMode.value === 'create') {
        // v11 批次 181 P2-1 修复：GreigeFabricForm 与 Partial<GreigeFabric> 字段一致，直接传入
        await createGreigeFabric(formData)
        ElMessage.success(t('greigeFabrics.index.messageCreateSuccess'))
      } else {
        // edit 模式下 formData.id 由 handleEdit 从 row.id 赋值
        await updateGreigeFabric(formData.id!, formData)
        ElMessage.success(t('greigeFabrics.index.messageUpdateSuccess'))
      }
      dialogVisible.value = false
      await loadGreigeFabrics()
    } catch (error) {
      ElMessage.error(
        dialogMode.value === 'create'
          ? t('greigeFabrics.index.messageCreateFailed')
          : t('greigeFabrics.index.messageUpdateFailed')
      )
    } finally {
      submitLoading.value = false
    }
  })
}

const handleDialogClose = () => {
  formRef.value?.resetFields()
}

const hasLoaded = createLazyLoader()

onMounted(() => {
  loadGreigeFabrics()
  loadIfNot('suppliers', loadSuppliers, hasLoaded)
})
</script>

<style scoped>
.greige-fabrics-page {
  padding: 20px;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
</style>

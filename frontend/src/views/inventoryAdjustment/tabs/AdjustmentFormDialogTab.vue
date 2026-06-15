<!--
  AdjustmentFormDialogTab.vue - 库存调整编辑对话框
  来源：原 inventoryAdjustment/index.vue 中 调整单编辑对话框
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="formData.id ? '编辑调整单' : '新建调整单'"
    width="800px"
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form ref="formRef" :model="formData" label-width="100px" :disabled="mode === 'view'">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="调整单号" prop="adjust_no">
            <el-input v-model="formData.adjust_no" :disabled="!!formData.id" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="调整日期" prop="adjust_date">
            <el-date-picker
              v-model="formData.adjust_date"
              type="date"
              value-format="YYYY-MM-DD"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="仓库" prop="warehouse_id">
        <el-select v-model="formData.warehouse_id" style="width: 100%">
          <el-option
            v-for="wh in warehouses"
            :key="wh.id"
            :label="wh.warehouse_name"
            :value="wh.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item label="调整原因" prop="reason">
        <el-input
          v-model="formData.reason"
          type="textarea"
          :rows="2"
          placeholder="请输入调整原因"
        />
      </el-form-item>
      <el-divider content-position="left">调整明细</el-divider>
      <div
        v-for="(item, index) in formData.items"
        :key="index"
        style="display: flex; gap: 8px; margin-bottom: 8px; align-items: center"
      >
        <el-select v-model="item.product_id" placeholder="产品" style="flex: 2" filterable>
          <el-option
            v-for="p in products"
            :key="p.id"
            :label="`${p.product_code} - ${p.product_name}`"
            :value="p.id"
          />
        </el-select>
        <el-input-number v-model="item.quantity" :min="1" placeholder="数量" style="flex: 1" />
        <el-input-number
          v-model="item.cost_price"
          :min="0"
          :precision="2"
          placeholder="单价"
          style="flex: 1"
        />
        <el-input v-model="item.remark" placeholder="备注" style="flex: 1.5" />
        <el-button
          type="danger"
          :icon="Delete"
          circle
          :disabled="formData.items.length <= 1"
          @click="removeItem(index)"
        />
      </div>
      <el-button type="primary" link @click="addItem">
        <el-icon><Plus /></el-icon>添加明细
      </el-button>
    </el-form>
    <template v-if="mode !== 'view'" #footer>
      <el-button @click="emit('update:modelValue', false)">取消</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">保存</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance } from 'element-plus'
import { Plus, Delete } from '@element-plus/icons-vue'
import {
  createInventoryAdjustment,
  updateInventoryAdjustment,
  generateInventoryAdjustmentNo,
  type InventoryAdjustmentEntity,
} from '@/api/inventoryAdjustment'
import type { Warehouse } from '@/api/warehouse'
import type { Product } from '@/api/product'
import { logger } from '@/utils/logger'

interface Props {
  modelValue: boolean
  currentRow: InventoryAdjustmentEntity | null
  warehouses: Warehouse[]
  products: Product[]
  mode: 'create' | 'edit' | 'view'
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void
  (e: 'submitted'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const formRef = ref<FormInstance>()
const submitLoading = ref(false)

const formData = reactive({
  id: 0,
  adjust_no: '',
  adjust_date: new Date().toISOString().split('T')[0],
  warehouse_id: undefined as number | undefined,
  reason: '',
  status: 'pending' as 'pending' | 'approved' | 'rejected',
  total_amount: 0,
  items: [{ product_id: 0, quantity: 1, cost_price: 0, amount: 0, remark: '' }] as {
    product_id: number
    quantity: number
    cost_price: number
    amount: number
    remark: string
  }[],
})

const resetForm = () => {
  formData.id = 0
  formData.adjust_no = ''
  formData.adjust_date = new Date().toISOString().split('T')[0]
  formData.warehouse_id = undefined
  formData.reason = ''
  formData.status = 'pending'
  formData.total_amount = 0
  formData.items = [{ product_id: 0, quantity: 1, cost_price: 0, amount: 0, remark: '' }]
}

const addItem = () => {
  formData.items.push({ product_id: 0, quantity: 1, cost_price: 0, amount: 0, remark: '' })
}
const removeItem = (index: number) => {
  if (formData.items.length > 1) formData.items.splice(index, 1)
}

const generateNo = async () => {
  try {
    const res = await generateInventoryAdjustmentNo()
    formData.adjust_no = res.data?.adjustment_no || ''
  } catch (error) {
    logger.error('生成调整单号失败', (error as Error).message)
  }
}

watch(
  () => props.modelValue,
  async val => {
    if (val) {
      if (props.currentRow) {
        Object.assign(formData, props.currentRow)
        if (!formData.items || formData.items.length === 0) {
          formData.items = [{ product_id: 0, quantity: 1, cost_price: 0, amount: 0, remark: '' }]
        }
      } else {
        resetForm()
        await generateNo()
      }
    }
  }
)

onMounted(() => {
  if (props.modelValue && !props.currentRow) {
    generateNo()
  }
})

const handleSubmit = async () => {
  submitLoading.value = true
  try {
    formData.total_amount = formData.items.reduce(
      (sum, item) => sum + item.cost_price * item.quantity,
      0
    )
    if (formData.id) {
      await updateInventoryAdjustment(formData.id, formData as Partial<InventoryAdjustmentEntity>)
    } else {
      await createInventoryAdjustment(formData as Partial<InventoryAdjustmentEntity>)
    }
    ElMessage.success('操作成功')
    emit('update:modelValue', false)
    emit('submitted')
  } catch (error) {
    ElMessage.error((error as Error).message || '操作失败')
    logger.error('调整单保存失败', (error as Error).message)
  } finally {
    submitLoading.value = false
  }
}
</script>

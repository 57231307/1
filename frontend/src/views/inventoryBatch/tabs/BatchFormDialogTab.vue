<!--
  BatchFormDialogTab.vue - 批次编辑对话框
  来源：原 inventoryBatch/index.vue 中 批次编辑对话框
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="formData.id ? '编辑批次' : '新建批次'"
    width="600px"
    :aria-label="formData.id ? '编辑批次' : '新建批次'"
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px" aria-label="库存批次表单">
      <el-form-item label="批次号" prop="batchNo">
        <el-input v-model="formData.batchNo" :disabled="!!formData.id" />
      </el-form-item>
      <el-form-item label="产品名称" prop="productName">
        <el-input v-model="formData.productName" />
      </el-form-item>
      <el-form-item label="色号" prop="colorNo">
        <el-input v-model="formData.colorNo" />
      </el-form-item>
      <el-form-item label="缸号" prop="dyeLotNo">
        <el-input v-model="formData.dyeLotNo" />
      </el-form-item>
      <el-form-item label="等级" prop="grade">
        <el-select v-model="formData.grade" style="width: 100%">
          <el-option label="一等品" value="一等品" />
          <el-option label="二等品" value="二等品" />
          <el-option label="三等品" value="三等品" />
        </el-select>
      </el-form-item>
      <el-form-item label="数量(米)" prop="quantityMeters">
        <el-input-number v-model="formData.quantityMeters" :min="0" style="width: 100%" />
      </el-form-item>
      <el-form-item label="数量(kg)" prop="quantityKg">
        <el-input-number
          v-model="formData.quantityKg"
          :min="0"
          :precision="2"
          style="width: 100%"
        />
      </el-form-item>
      <el-form-item label="克重" prop="gramWeight">
        <el-input-number v-model="formData.gramWeight" :min="0" style="width: 100%" />
      </el-form-item>
      <el-form-item label="幅宽" prop="width">
        <el-input-number v-model="formData.width" :min="0" style="width: 100%" />
      </el-form-item>
      <el-form-item label="生产日期" prop="productionDate">
        <el-date-picker
          v-model="formData.productionDate"
          type="date"
          value-format="YYYY-MM-DD"
          style="width: 100%"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:modelValue', false)">取消</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">保存</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import {
  createBatch,
  updateBatch,
  type CreateBatchRequest,
  type UpdateBatchRequest,
  type InventoryBatch,
} from '@/api/inventoryBatch'
// CreateBatchRequest / UpdateBatchRequest 用于 submit 时的强类型断言
import { logger } from '@/utils/logger'

interface Props {
  modelValue: boolean
  currentRow: InventoryBatch | null
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
  batchNo: '',
  productName: '',
  colorNo: '',
  dyeLotNo: '',
  grade: '一等品',
  quantityMeters: 0,
  quantityKg: 0,
  gramWeight: 0,
  width: 0,
  warehouseId: 0,
  warehouseName: '',
  productionDate: '',
})

const formRules: FormRules = {
  batchNo: [{ required: true, message: '请输入批次号', trigger: 'blur' }],
  productName: [{ required: true, message: '请输入产品名称', trigger: 'blur' }],
}

const resetForm = () => {
  formData.id = 0
  formData.batchNo = ''
  formData.productName = ''
  formData.colorNo = ''
  formData.dyeLotNo = ''
  formData.grade = '一等品'
  formData.quantityMeters = 0
  formData.quantityKg = 0
  formData.gramWeight = 0
  formData.width = 0
  formData.warehouseId = 0
  formData.warehouseName = ''
  formData.productionDate = ''
}

watch(
  () => props.modelValue,
  val => {
    if (val) {
      if (props.currentRow) {
        Object.assign(formData, props.currentRow)
      } else {
        resetForm()
      }
    }
  }
)

const handleSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async valid => {
    if (!valid) return
    submitLoading.value = true
    try {
      if (formData.id) {
        await updateBatch(formData.id, formData as unknown as UpdateBatchRequest)
      } else {
        await createBatch(formData as unknown as CreateBatchRequest)
      }
      ElMessage.success('操作成功')
      emit('update:modelValue', false)
      emit('submitted')
    } catch (error) {
      ElMessage.error((error as Error).message || '操作失败')
      logger.error('批次保存失败', (error as Error).message)
    } finally {
      submitLoading.value = false
    }
  })
}
</script>

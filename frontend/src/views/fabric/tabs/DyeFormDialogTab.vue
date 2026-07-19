<!--
  DyeFormDialogTab.vue - 染色批次编辑对话框
  来源：原 fabric/index.vue 中 染色批次编辑对话框
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="formData.id ? '编辑批次' : '新建批次'"
    width="700px"
    :aria-label="formData.id ? '编辑批次' : '新建批次'"
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form ref="formRef" :model="formData" label-width="100px" aria-label="染色配方表单">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="批次号" prop="batch_no">
            <el-input v-model="formData.batch_no" :disabled="!!formData.id" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="颜色" prop="color_name">
            <el-input v-model="formData.color_name" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="坯布" prop="greige_fabric_id">
        <el-select v-model="formData.greige_fabric_id" style="width: 100%">
          <el-option
            v-for="item in greigeFabrics"
            :key="item.id"
            :label="item.fabric_name"
            :value="item.id"
          />
        </el-select>
      </el-form-item>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="计划数量" prop="planned_quantity">
            <el-input-number v-model="formData.planned_quantity" :min="0" style="width: 100%" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="实际数量" prop="actual_quantity">
            <el-input-number v-model="formData.actual_quantity" :min="0" style="width: 100%" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="开始日期" prop="start_date">
        <el-date-picker
          v-model="formData.start_date"
          type="date"
          value-format="YYYY-MM-DD"
          style="width: 100%"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:modelValue', false)">取消</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">确定</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance } from 'element-plus'
import { createDyeBatch, updateDyeBatch, type DyeBatch } from '@/api/dye-batch'
import type { GreigeFabric } from '@/api/greige-fabric'
import { logger } from '@/utils/logger'

interface Props {
  modelValue: boolean
  currentRow: DyeBatch | null
  greigeFabrics: GreigeFabric[]
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
  batch_no: '',
  color_name: '',
  greige_fabric_id: undefined as number | undefined,
  planned_quantity: 0,
  actual_quantity: 0,
  start_date: '',
  status: 'pending' as 'pending' | 'in_progress' | 'completed' | 'cancelled',
})

const resetForm = () => {
  formData.id = 0
  formData.batch_no = ''
  formData.color_name = ''
  formData.greige_fabric_id = undefined
  formData.planned_quantity = 0
  formData.actual_quantity = 0
  formData.start_date = ''
  formData.status = 'pending'
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
  submitLoading.value = true
  try {
    if (formData.id) {
      await updateDyeBatch(formData.id, formData as Partial<DyeBatch>)
    } else {
      await createDyeBatch(formData as Partial<DyeBatch>)
    }
    ElMessage.success('操作成功')
    emit('update:modelValue', false)
    emit('submitted')
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '操作失败')
    logger.error('染色批次保存失败', err.message)
  } finally {
    submitLoading.value = false
  }
}
</script>

<!--
  ApproveDialogTab.vue - 库存调整审批对话框
  来源：原 inventoryAdjustment/index.vue 中 审批弹窗
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    title="审批调整单"
    width="500px"
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form ref="formRef" :model="formData" label-width="80px">
      <el-form-item label="调整单号">
        <el-input :model-value="currentRow?.adjust_no" disabled />
      </el-form-item>
      <el-form-item label="调整日期">
        <el-input :model-value="currentRow?.adjust_date" disabled />
      </el-form-item>
      <el-form-item label="仓库">
        <el-input :model-value="currentRow?.warehouse_name" disabled />
      </el-form-item>
      <el-form-item label="调整原因">
        <el-input :model-value="currentRow?.reason" disabled type="textarea" :rows="2" />
      </el-form-item>
      <el-form-item label="审批意见" prop="approval_comment">
        <el-input
          v-model="formData.approval_comment"
          type="textarea"
          :rows="3"
          placeholder="请输入审批意见"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:modelValue', false)">取消</el-button>
      <el-button type="warning" :loading="submitLoading" @click="handleReject">驳回</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handlePass">通过</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance } from 'element-plus'
import {
  approveInventoryAdjustment,
  rejectInventoryAdjustment,
  type InventoryAdjustmentEntity,
} from '@/api/inventoryAdjustment'
import { logger } from '@/utils/logger'

interface Props {
  modelValue: boolean
  currentRow: InventoryAdjustmentEntity | null
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void
  (e: 'submitted'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const formRef = ref<FormInstance>()
const submitLoading = ref(false)

const formData = reactive({ approval_comment: '' })

const resetForm = () => {
  formData.approval_comment = ''
}

watch(
  () => props.modelValue,
  val => {
    if (val) resetForm()
  }
)

const handlePass = async () => {
  if (!formRef.value || !props.currentRow) return
  submitLoading.value = true
  try {
    await approveInventoryAdjustment(props.currentRow.id as number)
    ElMessage.success('审批通过')
    emit('update:modelValue', false)
    emit('submitted')
  } catch (error) {
    ElMessage.error((error as Error).message || '操作失败')
    logger.error('审批失败', (error as Error).message)
  } finally {
    submitLoading.value = false
  }
}

const handleReject = async () => {
  if (!props.currentRow) return
  try {
    await ElMessageBox.confirm('确定要驳回此调整单吗？', '确认驳回', { type: 'warning' })
    submitLoading.value = true
    await rejectInventoryAdjustment(props.currentRow.id as number)
    ElMessage.success('已驳回')
    emit('update:modelValue', false)
    emit('submitted')
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error((error as Error).message || '操作失败')
    }
  } finally {
    submitLoading.value = false
  }
}
</script>

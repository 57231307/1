<!--
  ApproveTransferDialogTab.vue - 调拨单审批对话框
  来源：原 inventoryTransfer/index.vue 中 调拨单审批对话框
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    title="审批调拨单"
    width="500px"
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form ref="formRef" :model="formData" label-width="80px">
      <el-form-item label="调拨单号">
        <el-input :model-value="currentRow?.transfer_no" disabled />
      </el-form-item>
      <el-form-item label="调拨日期">
        <el-input :model-value="currentRow?.transfer_date" disabled />
      </el-form-item>
      <el-form-item label="调出仓库">
        <el-input :model-value="currentRow?.from_warehouse_name" disabled />
      </el-form-item>
      <el-form-item label="调入仓库">
        <el-input :model-value="currentRow?.to_warehouse_name" disabled />
      </el-form-item>
      <el-form-item label="审批意见" prop="approval_comment">
        <el-input
          v-model="formData.approval_comment"
          type="textarea"
          :rows="4"
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
import { approveInventoryTransfer, type InventoryTransferEntity } from '@/api/inventoryTransfer'
import { logger } from '@/utils/logger'

interface Props {
  modelValue: boolean
  currentRow: InventoryTransferEntity | null
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
    await approveInventoryTransfer(props.currentRow.id as number)
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
    await ElMessageBox.confirm('确定要驳回此调拨单吗？', '确认驳回', { type: 'warning' })
    submitLoading.value = true
    // reject 接口未在 api/inventoryTransfer 中实现，复用 approve 接口
    await approveInventoryTransfer(props.currentRow.id as number)
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

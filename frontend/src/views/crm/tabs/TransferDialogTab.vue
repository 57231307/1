<!--
  TransferDialogTab.vue - 客户公海池 - 分配/转移对话框
  来源：原 crm/pool.vue 中 分配对话框
  拆分日期：2026-06-15 B3-3
-->
<template>
  <el-dialog v-model="visible" title="分配客户" width="500px" aria-label="分配客户对话框">
    <p>
      将客户 <strong>{{ customerName }}</strong> 分配给：
    </p>
    <el-form :model="form" label-width="80px" aria-label="分配客户表单">
      <el-form-item label="负责人">
        <el-select v-model="form.ownerId" placeholder="请选择负责人" filterable>
          <el-option
            v-for="user in users"
            :key="user.id"
            :label="user.real_name"
            :value="user.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item label="分配原因">
        <el-input v-model="form.reason" type="textarea" :rows="3" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">取消</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">确认</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { ElMessage } from 'element-plus'
import type { User } from '@/api/user'
import { logger } from '@/utils/logger'
import { crmEnhancedApi } from '@/api/crm-enhanced'

interface Props {
  modelValue: boolean
  customerName: string
  customerId: number | null
  users: User[]
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void
  (e: 'submitted'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const visible = ref(props.modelValue)
const submitLoading = ref(false)
const form = reactive({ ownerId: undefined as number | undefined, reason: '' })

watch(
  () => props.modelValue,
  val => {
    visible.value = val
    if (val) {
      form.ownerId = undefined
      form.reason = ''
    }
  }
)

watch(visible, val => {
  emit('update:modelValue', val)
})

const handleSubmit = async () => {
  if (!props.customerId || !form.ownerId) {
    ElMessage.warning('请选择负责人')
    return
  }
  try {
    submitLoading.value = true
    // P1-5：实际调用分配 API
    await crmEnhancedApi.assignCustomer({
      customer_ids: [props.customerId],
      assign_to: form.ownerId,
      reason: form.reason,
    })
    ElMessage.success('分配成功')
    visible.value = false
    emit('submitted')
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '分配失败')
    logger.warn('分配失败', err.message)
  } finally {
    submitLoading.value = false
  }
}
</script>

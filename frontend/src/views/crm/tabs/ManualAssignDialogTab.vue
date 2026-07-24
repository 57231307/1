<!--
  ManualAssignDialogTab.vue - 客户分配规则 - 手动分配对话框
  来源：原 crm/assignment.vue 中 手动分配对话框
  拆分日期：2026-06-15 B3-3
-->
<template>
  <el-dialog v-model="visible" title="手动分配客户" width="500px" aria-label="手动分配客户对话框">
    <p>
      为客户 <strong>{{ customerName }}</strong> 选择新负责人：
    </p>
    <el-form :model="form" label-width="80px" aria-label="手动分配客户表单">
      <el-form-item label="新负责人">
        <el-select v-model="form.newOwnerId" placeholder="请选择负责人" filterable>
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
// D14 Batch 5b：原 crmEnhancedApi 对象已转风格 B 函数
import { assignCustomer } from '@/api/crm-enhanced'

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
const form = reactive({
  newOwnerId: undefined as number | undefined,
  reason: '',
})

watch(
  () => props.modelValue,
  val => {
    visible.value = val
    if (val) {
      form.newOwnerId = undefined
      form.reason = ''
    }
  }
)

watch(visible, val => {
  emit('update:modelValue', val)
})

const handleSubmit = async () => {
  if (!props.customerId || !form.newOwnerId) {
    ElMessage.warning('请选择新负责人')
    return
  }
  try {
    submitLoading.value = true
    // P1-5：实际调用手动分配 API
    await assignCustomer({
      customer_ids: [props.customerId],
      assign_to: form.newOwnerId,
      reason: form.reason,
    })
    ElMessage.success('分配成功')
    visible.value = false
    emit('submitted')
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '分配失败')
    logger.warn('手动分配失败', err.message)
  } finally {
    submitLoading.value = false
  }
}
</script>

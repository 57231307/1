<!--
  TransferDialogTab.vue - 客户公海池 - 分配/转移对话框
  来源：原 crm/pool.vue 中 分配对话框
-->
<template>
  <el-dialog v-model="visible" :title="t('crmTransferDialog.title')" width="500px" :aria-label="t('crmTransferDialog.ariaLabel')">
    <p>
      {{ t('crmTransferDialog.description', { name: customerName }) }}
    </p>
    <el-form :model="form" label-width="80px" :aria-label="t('crmTransferDialog.formAriaLabel')">
      <el-form-item :label="t('crmTransferDialog.owner')">
        <el-select v-model="form.ownerId" :placeholder="t('crmTransferDialog.ownerPlaceholder')" filterable>
          <el-option
            v-for="user in users"
            :key="user.id"
            :label="user.real_name"
            :value="user.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('crmTransferDialog.reason')">
        <el-input v-model="form.reason" type="textarea" :rows="3" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">{{ t('crmTransferDialog.cancel') }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{ t('crmTransferDialog.confirm') }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import type { User } from '@/api/user'
import { logger } from '@/utils/logger'
// D14 Batch 5b：原 crmEnhancedApi 对象已转风格 B 函数
import { assignCustomer } from '@/api/crm-enhanced'

const { t } = useI18n({ useScope: 'global' })

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
    ElMessage.warning(t('crmTransferDialog.message.ownerRequired'))
    return
  }
  try {
    submitLoading.value = true
    // P1-5：实际调用分配 API
    await assignCustomer({
      customer_ids: [props.customerId],
      assign_to: form.ownerId,
      reason: form.reason,
    })
    ElMessage.success(t('crmTransferDialog.message.success'))
    visible.value = false
    emit('submitted')
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || t('crmTransferDialog.message.failed'))
    logger.warn(t('crmTransferDialog.message.failed'), err.message)
  } finally {
    submitLoading.value = false
  }
}
</script>

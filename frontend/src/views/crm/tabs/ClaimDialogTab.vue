<!--
  ClaimDialogTab.vue - 客户公海池 - 领取对话框
  来源：原 crm/pool.vue 中 领取对话框
-->
<template>
  <el-dialog v-model="visible" :title="t('crmClaimDialog.title')" width="500px" :aria-label="t('crmClaimDialog.ariaLabel')">
    <p>
      {{ t('crmClaimDialog.description', { name: customerName }) }}
    </p>
    <el-form :model="form" label-width="80px" :aria-label="t('crmClaimDialog.formAriaLabel')">
      <el-form-item :label="t('crmClaimDialog.remark')">
        <el-input v-model="form.remark" type="textarea" :rows="3" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">{{ t('crmClaimDialog.cancel') }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{ t('crmClaimDialog.confirm') }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { logger } from '@/utils/logger'
// D14 Batch 5b：原 crmEnhancedApi 对象已转风格 B 函数
import { claimCustomerFromPool } from '@/api/crm-enhanced'

const { t } = useI18n({ useScope: 'global' })

interface Props {
  modelValue: boolean
  customerName: string
  customerId: number | null
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void
  (e: 'submitted'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const visible = ref(props.modelValue)
const submitLoading = ref(false)
const form = reactive({ remark: '' })

watch(
  () => props.modelValue,
  val => {
    visible.value = val
    if (val) form.remark = ''
  }
)

watch(visible, val => {
  emit('update:modelValue', val)
})

const handleSubmit = async () => {
  if (!props.customerId) return
  try {
    submitLoading.value = true
    // P1-5：实际调用领取 API
    await claimCustomerFromPool(props.customerId)
    ElMessage.success(t('crmClaimDialog.message.success'))
    visible.value = false
    emit('submitted')
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || t('crmClaimDialog.message.failed'))
    logger.warn(t('crmClaimDialog.message.failed'), err.message)
  } finally {
    submitLoading.value = false
  }
}
</script>

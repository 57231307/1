<!--
  GreigeFormDialogTab.vue - 坯布编辑对话框
  来源：原 fabric/index.vue 中 坯布编辑对话框
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="formData.id ? t('fabric.greigeFormDialog.titleEdit') : t('fabric.greigeFormDialog.titleCreate')"
    width="600px"
    :aria-label="formData.id ? t('fabric.greigeFormDialog.titleEdit') : t('fabric.greigeFormDialog.titleCreate')"
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form ref="formRef" :model="formData" label-width="100px" :aria-label="t('fabric.greigeFormDialog.formAriaLabel')">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('fabric.greigeFormDialog.labelCode')" prop="fabric_code">
            <el-input v-model="formData.fabric_code" :disabled="!!formData.id" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('fabric.greigeFormDialog.labelName')" prop="fabric_name">
            <el-input v-model="formData.fabric_name" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('fabric.greigeFormDialog.labelSupplier')" prop="supplier_id">
        <el-select v-model="formData.supplier_id" style="width: 100%">
          <el-option v-for="s in suppliers" :key="s.id" :label="s.supplier_name" :value="s.id" />
        </el-select>
      </el-form-item>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('fabric.greigeFormDialog.labelWidth')" prop="width">
            <el-input-number v-model="formData.width" :min="0" style="width: 100%" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('fabric.greigeFormDialog.labelWeight')" prop="weight">
            <el-input-number v-model="formData.weight" :min="0" style="width: 100%" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('fabric.greigeFormDialog.labelComposition')" prop="composition">
        <el-input v-model="formData.composition" :placeholder="t('fabric.greigeFormDialog.placeholderComposition')" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:modelValue', false)">{{ t('fabric.common.cancel') }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{ t('fabric.common.confirm') }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import type { FormInstance } from 'element-plus'
import { createGreigeFabric, updateGreigeFabric, type GreigeFabric } from '@/api/greige-fabric'
import type { Supplier } from '@/api/supplier'
import { logger } from '@/utils/logger'

const { t } = useI18n({ useScope: 'global' })

interface Props {
  modelValue: boolean
  currentRow: GreigeFabric | null
  suppliers: Supplier[]
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
  fabric_code: '',
  fabric_name: '',
  supplier_id: undefined as number | undefined,
  width: 0,
  weight: 0,
  composition: '',
  status: 'active' as 'active' | 'inactive',
})

const resetForm = () => {
  formData.id = 0
  formData.fabric_code = ''
  formData.fabric_name = ''
  formData.supplier_id = undefined
  formData.width = 0
  formData.weight = 0
  formData.composition = ''
  formData.status = 'active'
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
      await updateGreigeFabric(formData.id, formData as Partial<GreigeFabric>)
    } else {
      await createGreigeFabric(formData as Partial<GreigeFabric>)
    }
    ElMessage.success(t('fabric.common.success'))
    emit('update:modelValue', false)
    emit('submitted')
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || t('fabric.common.failed'))
    logger.error('坯布保存失败', err.message)
  } finally {
    submitLoading.value = false
  }
}
</script>

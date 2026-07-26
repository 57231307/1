<!--
  BatchFormDialogTab.vue - 批次编辑对话框
  来源：原 inventoryBatch/index.vue 中 批次编辑对话框
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="
      formData.id
        ? t('inventoryBatch.batchFormDialog.titleEdit')
        : t('inventoryBatch.batchFormDialog.titleCreate')
    "
    width="600px"
    :aria-label="
      formData.id
        ? t('inventoryBatch.batchFormDialog.ariaLabelEdit')
        : t('inventoryBatch.batchFormDialog.ariaLabelCreate')
    "
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form
      ref="formRef"
      :model="formData"
      :rules="formRules"
      label-width="100px"
      :aria-label="t('inventoryBatch.batchFormDialog.ariaLabelForm')"
    >
      <el-form-item :label="t('inventoryBatch.batchFormDialog.labelBatchNo')" prop="batchNo">
        <el-input v-model="formData.batchNo" :disabled="!!formData.id" />
      </el-form-item>
      <el-form-item
        :label="t('inventoryBatch.batchFormDialog.labelProductName')"
        prop="productName"
      >
        <el-input v-model="formData.productName" />
      </el-form-item>
      <el-form-item :label="t('inventoryBatch.batchFormDialog.labelColorNo')" prop="colorNo">
        <el-input v-model="formData.colorNo" />
      </el-form-item>
      <el-form-item :label="t('inventoryBatch.batchFormDialog.labelDyeLotNo')" prop="dyeLotNo">
        <el-input v-model="formData.dyeLotNo" />
      </el-form-item>
      <el-form-item :label="t('inventoryBatch.batchFormDialog.labelGrade')" prop="grade">
        <el-select v-model="formData.grade" style="width: 100%">
          <el-option
            :label="t('inventoryBatch.batchFormDialog.optionGradeFirst')"
            :value="t('inventoryBatch.batchFormDialog.optionGradeFirst')"
          />
          <el-option
            :label="t('inventoryBatch.batchFormDialog.optionGradeSecond')"
            :value="t('inventoryBatch.batchFormDialog.optionGradeSecond')"
          />
          <el-option
            :label="t('inventoryBatch.batchFormDialog.optionGradeThird')"
            :value="t('inventoryBatch.batchFormDialog.optionGradeThird')"
          />
        </el-select>
      </el-form-item>
      <el-form-item
        :label="t('inventoryBatch.batchFormDialog.labelQuantityMeters')"
        prop="quantityMeters"
      >
        <el-input-number v-model="formData.quantityMeters" :min="0" style="width: 100%" />
      </el-form-item>
      <el-form-item :label="t('inventoryBatch.batchFormDialog.labelQuantityKg')" prop="quantityKg">
        <el-input-number
          v-model="formData.quantityKg"
          :min="0"
          :precision="2"
          style="width: 100%"
        />
      </el-form-item>
      <el-form-item :label="t('inventoryBatch.batchFormDialog.labelGramWeight')" prop="gramWeight">
        <el-input-number v-model="formData.gramWeight" :min="0" style="width: 100%" />
      </el-form-item>
      <el-form-item :label="t('inventoryBatch.batchFormDialog.labelWidth')" prop="width">
        <el-input-number v-model="formData.width" :min="0" style="width: 100%" />
      </el-form-item>
      <el-form-item
        :label="t('inventoryBatch.batchFormDialog.labelProductionDate')"
        prop="productionDate"
      >
        <el-date-picker
          v-model="formData.productionDate"
          type="date"
          value-format="YYYY-MM-DD"
          style="width: 100%"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:modelValue', false)">{{
        t('inventoryBatch.batchFormDialog.buttonCancel')
      }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
        t('inventoryBatch.batchFormDialog.buttonSave')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import {
  createBatch,
  updateBatch,
  type CreateBatchRequest,
  type UpdateBatchRequest,
  type InventoryBatch,
} from '@/api/inventoryBatch'
import { logger } from '@/utils/logger'

const { t } = useI18n({ useScope: 'global' })

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

/** 校验规则：computed 确保语言切换后规则消息响应式更新 */
const formRules = computed<FormRules>(() => ({
  batchNo: [
    {
      required: true,
      message: t('inventoryBatch.batchFormDialog.ruleBatchNoRequired'),
      trigger: 'blur',
    },
  ],
  productName: [
    {
      required: true,
      message: t('inventoryBatch.batchFormDialog.ruleProductNameRequired'),
      trigger: 'blur',
    },
  ],
}))

const resetForm = () => {
  formData.id = 0
  formData.batchNo = ''
  formData.productName = ''
  formData.colorNo = ''
  formData.dyeLotNo = ''
  formData.grade = t('inventoryBatch.batchFormDialog.optionGradeFirst')
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
      ElMessage.success(t('inventoryBatch.batchFormDialog.messageSuccess'))
      emit('update:modelValue', false)
      emit('submitted')
    } catch (error) {
      ElMessage.error((error as Error).message || t('inventoryBatch.batchFormDialog.messageFailed'))
      logger.error('批次保存失败', (error as Error).message)
    } finally {
      submitLoading.value = false
    }
  })
}
</script>

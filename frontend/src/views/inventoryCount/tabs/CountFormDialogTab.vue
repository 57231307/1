<!--
  CountFormDialogTab.vue - 盘点单编辑对话框
  来源：原 inventoryCount/index.vue 中 盘点单编辑对话框
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="
      formData.id
        ? t('inventoryCount.formDialogTab.titleEdit')
        : t('inventoryCount.formDialogTab.titleCreate')
    "
    width="600px"
    :aria-label="
      mode === 'view'
        ? t('inventoryCount.formDialogTab.ariaLabelDetail')
        : formData.id
          ? t('inventoryCount.formDialogTab.ariaLabelEdit')
          : t('inventoryCount.formDialogTab.ariaLabelCreate')
    "
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form
      ref="formRef"
      :model="formData"
      label-width="100px"
      :disabled="mode === 'view'"
      :aria-label="t('inventoryCount.formDialogTab.ariaLabelForm')"
    >
      <el-form-item :label="t('inventoryCount.formDialogTab.labelCountNo')" prop="count_no">
        <el-input v-model="formData.count_no" :disabled="!!formData.id" />
      </el-form-item>
      <el-form-item :label="t('inventoryCount.formDialogTab.labelCountDate')" prop="count_date">
        <el-date-picker
          v-model="formData.count_date"
          type="date"
          value-format="YYYY-MM-DD"
          style="width: 100%"
        />
      </el-form-item>
      <el-form-item :label="t('inventoryCount.formDialogTab.labelWarehouse')" prop="warehouse_id">
        <el-select v-model="formData.warehouse_id" style="width: 100%">
          <el-option
            v-for="wh in warehouses"
            :key="wh.id"
            :label="wh.warehouse_name"
            :value="wh.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('inventoryCount.formDialogTab.labelRemark')" prop="remark">
        <el-input
          v-model="formData.remark"
          type="textarea"
          :rows="3"
          :placeholder="t('inventoryCount.formDialogTab.placeholderRemark')"
        />
      </el-form-item>
    </el-form>
    <template v-if="mode !== 'view'" #footer>
      <el-button @click="emit('update:modelValue', false)">{{
        t('inventoryCount.formDialogTab.buttonCancel')
      }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
        t('inventoryCount.formDialogTab.buttonSave')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import type { FormInstance } from 'element-plus'
import {
  createInventoryCount,
  updateInventoryCount,
  generateInventoryCountNo,
  type InventoryCountEntity,
} from '@/api/inventoryCount'
import type { Warehouse } from '@/api/warehouse'
import { logger } from '@/utils/logger'

const { t } = useI18n({ useScope: 'global' })

interface Props {
  modelValue: boolean
  currentRow: InventoryCountEntity | null
  warehouses: Warehouse[]
  mode: 'create' | 'edit' | 'view'
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
  count_no: '',
  count_date: new Date().toISOString().split('T')[0],
  warehouse_id: undefined as number | undefined,
  status: 'in_progress' as 'in_progress' | 'completed',
  remark: '',
})

const resetForm = () => {
  formData.id = 0
  formData.count_no = ''
  formData.count_date = new Date().toISOString().split('T')[0]
  formData.warehouse_id = undefined
  formData.status = 'in_progress'
  formData.remark = ''
}

const generateNo = async () => {
  try {
    const res = await generateInventoryCountNo()
    formData.count_no = res.data?.count_no || ''
  } catch (error) {
    logger.error(
      t('inventoryCount.formDialogTab.messageGenerateNoFailure'),
      (error as Error).message
    )
  }
}

watch(
  () => props.modelValue,
  async val => {
    if (val) {
      if (props.currentRow) {
        Object.assign(formData, props.currentRow)
      } else {
        resetForm()
        await generateNo()
      }
    }
  }
)

onMounted(() => {
  if (props.modelValue && !props.currentRow) {
    generateNo()
  }
})

const handleSubmit = async () => {
  submitLoading.value = true
  try {
    if (formData.id) {
      await updateInventoryCount(formData.id, formData as Partial<InventoryCountEntity>)
    } else {
      await createInventoryCount(formData as Partial<InventoryCountEntity>)
    }
    ElMessage.success(t('inventoryCount.formDialogTab.messageSuccess'))
    emit('update:modelValue', false)
    emit('submitted')
  } catch (error) {
    ElMessage.error((error as Error).message || t('inventoryCount.formDialogTab.messageFailure'))
    logger.error(t('inventoryCount.formDialogTab.messageSaveFailure'), (error as Error).message)
  } finally {
    submitLoading.value = false
  }
}
</script>

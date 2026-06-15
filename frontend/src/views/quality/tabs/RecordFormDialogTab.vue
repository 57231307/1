<!--
  RecordFormDialogTab.vue - 检验记录编辑对话框
  来源：原 quality/index.vue 中 检验记录编辑对话框
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="formData.id ? '编辑检验' : '新建检验'"
    width="700px"
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form ref="formRef" :model="formData" label-width="100px">
      <el-form-item label="记录编号" prop="record_no">
        <el-input v-model="formData.record_no" :disabled="!!formData.id" />
      </el-form-item>
      <el-form-item label="检验类型" prop="inspection_type">
        <el-select v-model="formData.inspection_type" style="width: 100%">
          <el-option label="进货检验" value="incoming" />
          <el-option label="过程检验" value="process" />
          <el-option label="成品检验" value="finished" />
          <el-option label="出厂检验" value="outgoing" />
        </el-select>
      </el-form-item>
      <el-form-item label="产品" prop="product_name">
        <el-input v-model="formData.product_name" placeholder="产品名称" />
      </el-form-item>
      <el-form-item label="批次号" prop="batch_no">
        <el-input v-model="formData.batch_no" />
      </el-form-item>
      <el-form-item label="检验日期" prop="inspection_date">
        <el-date-picker
          v-model="formData.inspection_date"
          type="date"
          value-format="YYYY-MM-DD"
          style="width: 100%"
        />
      </el-form-item>
      <el-form-item label="检验员" prop="inspector">
        <el-input v-model="formData.inspector" />
      </el-form-item>
      <el-form-item label="检验结果" prop="result">
        <el-radio-group v-model="formData.result">
          <el-radio label="pass">合格</el-radio>
          <el-radio label="fail">不合格</el-radio>
          <el-radio label="pending">待检</el-radio>
        </el-radio-group>
      </el-form-item>
      <el-form-item label="备注" prop="remark">
        <el-input v-model="formData.remark" type="textarea" />
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
import { createQualityRecord, type QualityRecord, type Defect } from '@/api/quality'
import { logger } from '@/utils/logger'

interface Props {
  modelValue: boolean
  currentRow: QualityRecord | null
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
  record_no: '',
  inspection_type: '',
  product_id: undefined as number | undefined,
  product_name: '',
  batch_no: '',
  inspection_date: '',
  inspector: '',
  result: 'pending' as 'pass' | 'fail' | 'pending',
  defects: [] as Defect[],
  remark: '',
})

const resetForm = () => {
  formData.id = 0
  formData.record_no = ''
  formData.inspection_type = ''
  formData.product_id = undefined
  formData.product_name = ''
  formData.batch_no = ''
  formData.inspection_date = ''
  formData.inspector = ''
  formData.result = 'pending'
  formData.defects = []
  formData.remark = ''
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
      ElMessage.info('更新功能待实现')
    } else {
      await createQualityRecord(formData as Partial<QualityRecord>)
    }
    ElMessage.success('操作成功')
    emit('update:modelValue', false)
    emit('submitted')
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '操作失败')
    logger.error('检验记录保存失败', err.message)
  } finally {
    submitLoading.value = false
  }
}
</script>

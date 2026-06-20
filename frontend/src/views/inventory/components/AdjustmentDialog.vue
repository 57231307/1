<!--
  AdjustmentDialog.vue - 库存调整对话框
  任务编号: P14 批 2 I-3 第 8 批
  拆分原 inventory/index.vue 的库存调整对话框
  内部维护 localForm，避免直接突变 form prop
-->
<template>
  <el-dialog
    :model-value="visible"
    title="库存调整"
    width="500px"
    :close-on-click-modal="false"
    @update:model-value="onClose"
  >
    <el-form :model="localForm" label-width="100px">
      <el-form-item v-if="localForm.product_name" label="产品">
        <el-input :value="localForm.product_name" disabled />
      </el-form-item>
      <el-form-item v-if="localForm.warehouse_name" label="仓库">
        <el-input :value="localForm.warehouse_name" disabled />
      </el-form-item>
      <el-form-item v-if="localForm.current_quantity" label="当前库存">
        <el-input :value="localForm.current_quantity" disabled />
      </el-form-item>
      <el-form-item label="调整类型">
        <el-radio-group v-model="localForm.adjustment_type">
          <el-radio value="increase">增加</el-radio>
          <el-radio value="decrease">减少</el-radio>
        </el-radio-group>
      </el-form-item>
      <el-form-item label="调整数量">
        <el-input-number
          v-model="localForm.adjustment_quantity"
          :min="1"
          style="width: 100%"
        />
      </el-form-item>
      <el-form-item label="调整原因">
        <el-input
          v-model="localForm.reason"
          type="textarea"
          :rows="3"
          placeholder="请输入调整原因"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="onClose(false)">取消</el-button>
      <el-button type="primary" @click="onSubmit">确定</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { reactive, watch } from 'vue'

const props = defineProps<{
  visible: boolean
  form: any
}>()

const emit = defineEmits<{
  (e: 'update:visible', val: boolean): void
  (e: 'submit', data: any): void
}>()

// 浅拷贝避免突变 prop
const localForm = reactive<Record<string, any>>({})
watch(
  () => props.form,
  newVal => {
    Object.keys(localForm).forEach(k => delete localForm[k])
    Object.assign(localForm, JSON.parse(JSON.stringify(newVal)))
  },
  { immediate: true, deep: true }
)

// 同步 localForm 回父组件 form prop
watch(
  localForm,
  (newVal: Record<string, any>) => {
    emit('update:form', JSON.parse(JSON.stringify(newVal)))
  },
  { deep: true }
)

const onClose = (val: boolean) => {
  emit('update:visible', val)
}

const onSubmit = () => {
  emit('submit')
}
</script>

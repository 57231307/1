<!--
  AdjustmentDialog.vue - 库存调整对话框
  任务编号: P14 批 2 I-3 第 8 批
  拆分原 inventory/index.vue 的库存调整对话框
  行为完全保持一致（仅结构重构）
  使用 props.initialForm 初始化 + 内部 localForm（不直接突变 prop）
  submit 时 emit submitWithForm(localForm) 把当前 form 回传
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

// 库存调整表单数据结构（字段与 inventory 父组件 initialForm 保持一致）
interface AdjustmentForm {
  product_name?: string
  warehouse_name?: string
  current_quantity?: number
  adjustment_type: 'increase' | 'decrease'
  adjustment_quantity: number
  reason: string
}

const props = defineProps<{
  visible: boolean
  initialForm: AdjustmentForm
}>()

const emit = defineEmits<{
  (e: 'update:visible', val: boolean): void
  (e: 'submit', data: AdjustmentForm): void
}>()

// 浅拷贝 initialForm 同步初始值（不直接突变 prop）
const localForm = reactive<AdjustmentForm>({} as AdjustmentForm)
watch(
  () => props.initialForm,
  newVal => {
    Object.keys(localForm).forEach(k => delete localForm[k])
    Object.assign(localForm, JSON.parse(JSON.stringify(newVal)))
  },
  { immediate: true, deep: true }
)

const onClose = (val: boolean) => {
  emit('update:visible', val)
}

const onSubmit = () => {
  emit('submit', JSON.parse(JSON.stringify(localForm)))
}
</script>

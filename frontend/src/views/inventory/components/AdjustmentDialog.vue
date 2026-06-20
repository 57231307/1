<!--
  AdjustmentDialog.vue - 库存调整对话框
  任务编号: P14 批 2 I-3 第 8 批
  拆分原 inventory/index.vue 的库存调整对话框
-->
<template>
  <el-dialog
    :model-value="visible"
    title="库存调整"
    width="500px"
    :close-on-click-modal="false"
    @update:model-value="onClose"
  >
    <el-form :model="form" label-width="100px">
      <el-form-item v-if="form.product_name" label="产品">
        <el-input :value="form.product_name" disabled />
      </el-form-item>
      <el-form-item v-if="form.warehouse_name" label="仓库">
        <el-input :value="form.warehouse_name" disabled />
      </el-form-item>
      <el-form-item v-if="form.current_quantity" label="当前库存">
        <el-input :value="form.current_quantity" disabled />
      </el-form-item>
      <el-form-item label="调整类型">
        <el-radio-group v-model="form.adjustment_type">
          <el-radio value="increase">增加</el-radio>
          <el-radio value="decrease">减少</el-radio>
        </el-radio-group>
      </el-form-item>
      <el-form-item label="调整数量">
        <el-input-number
          v-model="form.adjustment_quantity"
          :min="1"
          style="width: 100%"
        />
      </el-form-item>
      <el-form-item label="调整原因">
        <el-input
          v-model="form.reason"
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
defineProps<{
  visible: boolean
  form: any
}>()

const emit = defineEmits<{
  (e: 'update:visible', val: boolean): void
  (e: 'submit'): void
}>()

const onClose = (val: boolean) => {
  emit('update:visible', val)
}

const onSubmit = () => {
  emit('submit')
}
</script>

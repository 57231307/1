<!-- eslint-disable vue/no-mutating-props -->
<!--
  PrRtnApr.vue - 采购退货审批对话框
  任务编号: P14 批 2 I-3 第 2 批（拆分原 purchase-return/index.vue）
-->
<template>
  <el-dialog
    :model-value="visible"
    title="审批退货单"
    width="500px"
    @update:model-value="onVisibleChange"
  >
    <el-form :model="approveForm" label-width="80px">
      <el-form-item label="审批意见">
        <el-input
          v-model="approveForm.remark"
          type="textarea"
          :rows="3"
          placeholder="请输入审批意见"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="onCancel">取消</el-button>
      <el-button type="danger" @click="emit('reject')">拒绝</el-button>
      <el-button type="success" @click="emit('approve-confirm')">通过</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
// 审批表单类型
interface ApproveForm {
  id: number
  remark: string
}

// 采购退货审批对话框属性
const props = defineProps<{
  // 对话框可见性
  visible: boolean
  // 审批表单
  approveForm: ApproveForm
}>()

// 定义事件
const emit = defineEmits<{
  // 关闭
  (e: 'update:visible', value: boolean): void
  // 通过
  (e: 'approve-confirm'): void
  // 拒绝
  (e: 'reject'): void
}>()

/** 关闭对话框 */
const onVisibleChange = (v: boolean) => {
  emit('update:visible', v)
}

/** 取消 */
const onCancel = () => {
  emit('update:visible', false)
}
</script>

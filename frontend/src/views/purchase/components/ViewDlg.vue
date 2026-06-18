<script setup lang="ts">
/**
 * ViewDlg - 采购单详情对话框（纯展示）
 * 任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue 查看对话框）
 */
interface Props {
  modelValue: boolean
  data: any
  getStatusType: (s: string) => string
  getStatusText: (s: string) => string
  getPaymentStatusType: (s: string) => string
  getPaymentStatusText: (s: string) => string
}

defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
}>()
</script>

<template>
  <el-dialog
    :model-value="modelValue"
    title="采购单详情"
    width="800px"
    @update:model-value="(v: boolean) => emit('update:modelValue', v)"
  >
    <el-descriptions :column="2" border>
      <el-descriptions-item label="采购单号">{{ data.order_no }}</el-descriptions-item>
      <el-descriptions-item label="供应商">{{ data.supplier_name }}</el-descriptions-item>
      <el-descriptions-item label="订单日期">{{ data.order_date }}</el-descriptions-item>
      <el-descriptions-item label="要求交货日期">{{
        data.required_date
      }}</el-descriptions-item>
      <el-descriptions-item label="订单金额"
        >¥{{ data.total_amount?.toLocaleString() }}</el-descriptions-item
      >
      <el-descriptions-item label="已收货金额"
        >¥{{ (data.received_amount || 0).toLocaleString() }}</el-descriptions-item
      >
      <el-descriptions-item label="付款状态">
        <el-tag :type="getPaymentStatusType(data.payment_status)">{{
          getPaymentStatusText(data.payment_status)
        }}</el-tag>
      </el-descriptions-item>
      <el-descriptions-item label="订单状态">
        <el-tag :type="getStatusType(data.status)">{{
          getStatusText(data.status)
        }}</el-tag>
      </el-descriptions-item>
      <el-descriptions-item label="创建人">{{ data.creator_name }}</el-descriptions-item>
      <el-descriptions-item label="创建时间">{{ data.created_at }}</el-descriptions-item>
      <el-descriptions-item label="备注" :span="2">{{
        data.remarks || '无'
      }}</el-descriptions-item>
    </el-descriptions>
    <div style="margin-top: 20px">
      <h4>采购明细</h4>
      <el-table :data="data.items || []" border style="width: 100%">
        <el-table-column prop="product_name" label="产品" width="150" />
        <el-table-column prop="product_code" label="产品编码" width="120" />
        <el-table-column prop="quantity" label="数量" width="100" />
        <el-table-column prop="unit_price" label="单价" width="100" />
        <el-table-column prop="subtotal" label="金额" width="120" />
        <el-table-column prop="received_quantity" label="已收货" width="100" />
        <el-table-column prop="remarks" label="备注" />
      </el-table>
    </div>
  </el-dialog>
</template>

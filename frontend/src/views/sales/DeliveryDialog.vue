<!--
  DeliveryDialog.vue - 销售发货对话框
  来源：原 sales/index.vue 中 发货 dialog
  拆分日期：2026-06-15 B3-1
-->
<template>
  <el-dialog
    :model-value="visible"
    title="销售发货"
    width="800px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="销售单号">
            <el-input :model-value="form.order_no" readonly />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="客户">
            <el-input :model-value="form.customer_name" readonly />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="发货日期" required>
            <el-date-picker
              :model-value="form.delivery_date"
              type="date"
              placeholder="选择日期"
              style="width: 100%"
              @update:model-value="(v: string) => updateForm('delivery_date', v)"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="仓库" required>
            <el-select
              :model-value="form.warehouse_id"
              placeholder="选择仓库"
              style="width: 100%"
              @update:model-value="(v: number) => updateForm('warehouse_id', v)"
            >
              <el-option
                v-for="w in warehouses"
                :key="w.id"
                :label="w.warehouse_name || w.name"
                :value="w.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="发货明细">
        <el-table :data="form.items" border style="width: 100%">
          <el-table-column prop="product_name" label="产品" width="150" />
          <el-table-column prop="quantity" label="订单数量" width="100" />
          <el-table-column prop="delivered_quantity" label="已发货" width="100" />
          <el-table-column label="本次发货" width="120">
            <template #default="{ row }">
              <el-input-number
                :model-value="row.deliver_quantity"
                :min="0"
                :max="row.quantity - (row.delivered_quantity || 0)"
                size="small"
                @update:model-value="(v: number) => updateItem(row, 'deliver_quantity', v)"
              />
            </template>
          </el-table-column>
          <el-table-column prop="unit_price" label="单价" width="100" />
          <el-table-column label="备注" min-width="150">
            <template #default="{ row }">
              <el-input
                :model-value="row.remarks"
                size="small"
                placeholder="备注"
                @update:model-value="(v: string) => updateItem(row, 'remarks', v)"
              />
            </template>
          </el-table-column>
        </el-table>
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">取消</el-button>
      <el-button type="primary" :loading="submitting" @click="handleSubmit(form)"
        >确定发货</el-button
      >
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
// 注：form 由父组件通过 ref() 创建，prop 传递的 form 对象在父子组件间共享是设计意图；
// 子组件在用户交互时直接更新 form 字段，父组件通过 v-model 同步可见。
import { ElMessage } from 'element-plus'

interface DeliveryItem {
  product_id: number
  product_name: string
  quantity: number
  delivered_quantity: number
  deliver_quantity: number
  unit_price: number
  remarks: string
}

interface DeliveryForm {
  order_id: number
  order_no: string
  customer_name: string
  delivery_date: string
  warehouse_id: number | undefined
  items: DeliveryItem[]
}

const props = defineProps<{
  visible: boolean
  form: DeliveryForm
  warehouses: { id: number; warehouse_name?: string; name?: string }[]
  submitting?: boolean
}>()

const emit = defineEmits<{
  'update:visible': [value: boolean]
  'update:form': [value: DeliveryForm]
  submit: [data: DeliveryForm]
}>()

// 通过 emit 通知父组件更新 form 字段，避免直接修改 prop
const updateForm = <K extends keyof DeliveryForm>(key: K, value: DeliveryForm[K]) => {
  emit('update:form', { ...props.form, [key]: value })
}

const updateItem = <K extends keyof DeliveryItem>(
  row: DeliveryItem,
  key: K,
  value: DeliveryItem[K]
) => {
  // eslint-disable-next-line vue/no-mutating-props
  row[key] = value
}

const handleSubmit = (form: DeliveryForm) => {
  // 校验：确保必填项已填
  if (!form.warehouse_id) {
    ElMessage.warning('请选择仓库')
    return
  }
  if (!form.delivery_date) {
    ElMessage.warning('请选择发货日期')
    return
  }
  const hasDelivery = form.items.some(i => i.deliver_quantity > 0)
  if (!hasDelivery) {
    ElMessage.warning('请至少填写一项发货数量')
    return
  }
  emit('submit', form)
}
</script>

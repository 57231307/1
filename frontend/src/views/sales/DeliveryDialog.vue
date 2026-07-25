<!--
  DeliveryDialog.vue - 销售发货对话框
  来源：原 sales/index.vue 中 发货 dialog
  拆分日期：2026-06-15 B3-1
  P9-3 批次 F 重构：移除 vue/no-mutating-props 抑制，通过 emit 整体覆盖 + 局部 update
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('sales.delivery.title')"
    width="800px"
    :aria-label="t('sales.delivery.dialogAriaLabel')"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form :aria-label="t('sales.delivery.formAriaLabel')">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('sales.delivery.salesOrderNo')">
            <el-input :model-value="form.order_no" readonly />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('sales.delivery.customer')">
            <el-input :model-value="form.customer_name" readonly />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('sales.delivery.deliveryDate')" required>
            <el-date-picker
              :model-value="form.delivery_date"
              type="date"
              :placeholder="t('sales.delivery.datePlaceholder')"
              style="width: 100%"
              @update:model-value="(v: string) => updateForm('delivery_date', v)"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('sales.delivery.warehouse')" required>
            <el-select
              :model-value="form.warehouse_id"
              :placeholder="t('sales.delivery.warehousePlaceholder')"
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
      <el-form-item :label="t('sales.delivery.deliveryItems')">
        <el-table :data="form.items" border style="width: 100%" :aria-label="t('sales.delivery.itemsTableAriaLabel')">
          <el-table-column prop="product_name" :label="t('sales.delivery.product')" width="150" />
          <el-table-column prop="quantity" :label="t('sales.delivery.orderQuantity')" width="100" />
          <el-table-column prop="delivered_quantity" :label="t('sales.delivery.delivered')" width="100" />
          <el-table-column :label="t('sales.delivery.currentDelivery')" width="120">
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
          <el-table-column prop="unit_price" :label="t('sales.delivery.unitPrice')" width="100" />
          <el-table-column :label="t('sales.delivery.remark')" min-width="150">
            <template #default="{ row }">
              <el-input
                :model-value="row.remarks"
                size="small"
                :placeholder="t('sales.delivery.remarkPlaceholder')"
                @update:model-value="(v: string) => updateItem(row, 'remarks', v)"
              />
            </template>
          </el-table-column>
        </el-table>
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">{{ t('sales.delivery.cancel') }}</el-button>
      <el-button type="primary" :loading="submitting" @click="handleSubmit(form)"
        >{{ t('sales.delivery.confirmDelivery') }}</el-button
      >
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
// 注：form 由父组件通过 reactive() 创建并通过 prop 传入；
// 子组件在用户交互时通过 emit('update:form', newForm) 整体覆盖，避免直接修改 prop。
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'

const { t } = useI18n({ useScope: 'global' })

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

// 通过 emit 通知父组件更新 form 字段（顶层字段）
const updateForm = <K extends keyof DeliveryForm>(key: K, value: DeliveryForm[K]) => {
  emit('update:form', { ...props.form, [key]: value })
}

// 通过 emit 通知父组件更新 items 数组中的指定行
const updateItem = <K extends keyof DeliveryItem>(
  row: DeliveryItem,
  key: K,
  value: DeliveryItem[K]
) => {
  // 创建新的 items 数组（不可变更新），避免直接修改 prop.items
  const newItems = props.form.items.map(item =>
    item === row ? { ...item, [key]: value } : item
  )
  emit('update:form', { ...props.form, items: newItems })
}

const handleSubmit = (form: DeliveryForm) => {
  // 校验：确保必填项已填
  if (!form.warehouse_id) {
    ElMessage.warning(t('sales.delivery.warehouseRequired'))
    return
  }
  if (!form.delivery_date) {
    ElMessage.warning(t('sales.delivery.deliveryDateRequired'))
    return
  }
  const hasDelivery = form.items.some(i => i.deliver_quantity > 0)
  if (!hasDelivery) {
    ElMessage.warning(t('sales.delivery.atLeastOneDelivery'))
    return
  }
  emit('submit', form)
}
</script>

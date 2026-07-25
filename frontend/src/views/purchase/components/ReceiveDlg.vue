<!--
  ReceiveDlg - 采购收货对话框
  任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue 收货对话框）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="t('purchase.receiveDlg.title')"
    width="800px"
    :aria-label="t('purchase.receiveDlg.ariaLabel')"
    @update:model-value="(v: boolean) => emit('update:modelValue', v)"
  >
    <el-form :model="localForm" label-width="100px" :aria-label="t('purchase.receiveDlg.formAria')">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('purchase.receiveDlg.orderNo')">
            <el-input v-model="localForm.order_no" readonly />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('purchase.receiveDlg.supplier')">
            <el-input v-model="localForm.supplier_name" readonly />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('purchase.receiveDlg.receiveDate')" required>
            <el-date-picker
              v-model="localForm.receive_date"
              type="date"
              :placeholder="t('purchase.receiveDlg.datePlaceholder')"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('purchase.receiveDlg.warehouse')" required>
            <el-select
              v-model="localForm.warehouse_id"
              :placeholder="t('purchase.receiveDlg.warehousePlaceholder')"
              style="width: 100%"
            >
              <el-option
                v-for="w in warehouses"
                :key="w.id"
                :label="w.warehouse_name"
                :value="w.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('purchase.receiveDlg.detail')">
        <el-table :data="localForm.items" border style="width: 100%" :aria-label="t('purchase.receiveDlg.detailListAria')">
          <el-table-column prop="product_name" :label="t('purchase.receiveDlg.colProduct')" width="150" />
          <el-table-column prop="ordered_quantity" :label="t('purchase.receiveDlg.colOrderedQty')" width="100" />
          <el-table-column prop="received_quantity" :label="t('purchase.receiveDlg.colReceivedQty')" width="100" />
          <el-table-column :label="t('purchase.receiveDlg.colThisReceive')" width="120">
            <template #default="{ row }">
              <el-input-number
                v-model="row.receive_quantity"
                :min="0"
                :max="row.ordered_quantity - row.received_quantity"
                size="small"
              />
            </template>
          </el-table-column>
          <el-table-column prop="unit_price" :label="t('purchase.receiveDlg.colUnitPrice')" width="100" />
          <el-table-column :label="t('purchase.receiveDlg.colRemark')" min-width="150">
            <template #default="{ row }">
              <el-input v-model="row.remarks" size="small" :placeholder="t('purchase.receiveDlg.remarkPlaceholder')" />
            </template>
          </el-table-column>
        </el-table>
      </el-form-item>
    </el-form>
    <template #footer>
      <span class="dialog-footer">
        <el-button @click="onCancel">{{ t('purchase.receiveDlg.cancel') }}</el-button>
        <el-button type="primary" @click="onSubmit">{{ t('purchase.receiveDlg.confirm') }}</el-button>
      </span>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { deepClone } from '@/utils'
import { ref, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Warehouse } from '@/api/warehouse'
import type { ReceiveFormData } from '../composables/usePurchRcv'

const { t } = useI18n({ useScope: 'global' })

const props = defineProps<{
  // 对话框可见性
  modelValue: boolean
  // 表单数据（由父组件管理，子组件通过 emit('update:form') 回写）
  form: ReceiveFormData
  // 仓库列表
  warehouses: Warehouse[]
  // 提交
  onSubmit: () => void
  // 取消
  onCancel: () => void
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
  // 整体回写表单（父组件监听此事件并回写到自己的 form.value）
  (e: 'update:form', form: ReceiveFormData): void
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
// 注意：表单内有 items 数组，需要深拷贝以保证本地修改与父组件解耦
const localForm = ref<ReceiveFormData>(deepClone(props.form))

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local（如父组件打开对话框时填充数据）
watch(
  () => props.form,
  (newForm) => {
    if (syncing) return
    syncing = true
    localForm.value = deepClone(newForm)
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 本地变化时通知父组件（用户输入）
watch(
  localForm,
  (newForm) => {
    if (syncing) return
    syncing = true
    emit('update:form', deepClone(newForm))
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)
</script>

<style scoped>
.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>

<!--
  ReturnEditDialog.vue - 销售退货新建/编辑对话框
  任务编号: P14 批 2 I-3 第 7 批
  拆分原 sales-returns/index.vue 的新建/编辑对话框部分
  包含主表单 + 退货明细表 + 总金额计算
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="dialogMode === 'create' ? '新建退货单' : '编辑退货单'"
    width="900px"
    @update:model-value="onClose"
    @close="onDialogClose"
  >
    <el-form
      ref="formRef"
      :model="formData"
      :rules="formRules"
      label-width="120px"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="销售订单号" prop="salesOrderId">
            <el-select
              v-model="formData.salesOrderId"
              placeholder="请选择销售订单"
              style="width: 100%"
              filterable
              @change="onSalesOrderChange"
            >
              <el-option
                v-for="order in salesOrderList"
                :key="order.id"
                :label="order.order_no"
                :value="order.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="客户" prop="customerId">
            <el-select
              v-model="formData.customerId"
              placeholder="请选择客户"
              style="width: 100%"
              filterable
            >
              <el-option
                v-for="customer in customerList"
                :key="customer.id"
                :label="customer.customer_name"
                :value="customer.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="退货日期" prop="returnDate">
            <el-date-picker
              v-model="formData.returnDate"
              type="date"
              placeholder="请选择退货日期"
              style="width: 100%"
              format="YYYY-MM-DD"
              value-format="YYYY-MM-DD"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="退货原因" prop="reason">
            <el-select v-model="formData.reason" placeholder="请选择退货原因" style="width: 100%">
              <el-option label="质量问题" value="quality" />
              <el-option label="数量不符" value="quantity" />
              <el-option label="规格不符" value="specification" />
              <el-option label="包装破损" value="packaging" />
              <el-option label="其他" value="other" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :span="24">
          <el-form-item label="备注" prop="remarks">
            <el-input
              v-model="formData.remarks"
              type="textarea"
              :rows="3"
              placeholder="请输入备注"
            />
          </el-form-item>
        </el-col>
      </el-row>

      <el-divider />

      <el-form-item label="退货明细">
        <el-button type="primary" size="small" style="margin-bottom: 10px" @click="onAddItem">
          添加明细
        </el-button>
        <el-table :data="formData.items" border style="width: 100%">
          <el-table-column label="产品名称" width="200">
            <template #default="{ row }">
              <el-select
                v-model="row.productId"
                placeholder="选择产品"
                style="width: 100%"
                filterable
              >
                <el-option
                  v-for="product in productList"
                  :key="product.id"
                  :label="product.product_name"
                  :value="product.id"
                />
              </el-select>
            </template>
          </el-table-column>
          <el-table-column label="数量" width="120">
            <template #default="{ row }">
              <el-input-number
                v-model="row.quantity"
                :min="1"
                :precision="2"
                style="width: 100%"
                @change="onCalculate"
              />
            </template>
          </el-table-column>
          <el-table-column label="单价" width="120">
            <template #default="{ row }">
              <el-input-number
                v-model="row.unitPrice"
                :min="0"
                :precision="2"
                style="width: 100%"
                @change="onCalculate"
              />
            </template>
          </el-table-column>
          <el-table-column label="金额" width="120">
            <template #default="{ row }">
              {{ (row.quantity * row.unitPrice).toFixed(2) }}
            </template>
          </el-table-column>
          <el-table-column label="退货原因" width="150">
            <template #default="{ row }">
              <el-input v-model="row.reason" placeholder="原因" size="small" />
            </template>
          </el-table-column>
          <el-table-column label="操作" width="80">
            <template #default="{ $index }">
              <el-button type="danger" size="small" @click="onRemoveItem($index)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-form-item>

      <el-row :gutter="20">
        <el-col :span="12" :offset="12">
          <el-form-item label="退货总金额">
            <el-input-number
              v-model="formData.totalAmount"
              :precision="2"
              :disabled="true"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
    </el-form>

    <template #footer>
      <el-button @click="onClose(false)">取消</el-button>
      <el-button type="primary" :loading="submitLoading" @click="onSubmit">确定</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'

const props = defineProps<{
  visible: boolean
  dialogMode: 'create' | 'edit'
  formData: any
  salesOrderList: any[]
  customerList: any[]
  productList: any[]
  formRules: FormRules
  submitLoading: boolean
}>()

const emit = defineEmits<{
  (e: 'update:visible', val: boolean): void
  (e: 'submit'): void
  (e: 'salesOrderChange', orderId: number): void
  (e: 'addItem'): void
  (e: 'removeItem', index: number): void
  (e: 'calculate'): void
  (e: 'dialogClose'): void
}>()

const formRef = ref<FormInstance>()

const onClose = (val: boolean) => {
  emit('update:visible', val)
}

const onSubmit = () => {
  emit('submit')
}

const onSalesOrderChange = (orderId: number) => {
  emit('salesOrderChange', orderId)
}

const onAddItem = () => {
  emit('addItem')
}

const onRemoveItem = (index: number) => {
  emit('removeItem', index)
}

const onCalculate = () => {
  emit('calculate')
}

const onDialogClose = () => {
  emit('dialogClose')
}

defineExpose({ formRef })
</script>

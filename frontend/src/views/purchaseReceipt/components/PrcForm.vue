<!--
  PrcForm.vue - 采购入库新增/编辑对话框
  拆分自 purchaseReceipt/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-dialog
    :model-value="visible"
    :title="title"
    width="800px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form ref="formRef" :model="form" :rules="rules" label-width="100px">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="入库单号" prop="receipt_no">
            <el-input v-model="form.receipt_no" readonly />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="入库日期" prop="receipt_date">
            <el-date-picker v-model="form.receipt_date" type="date" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="供应商" prop="supplier_id">
            <el-select
              :model-value="form.supplier_id"
              placeholder="请选择供应商"
              @update:model-value="(v: number | undefined) => (form.supplier_id = v)"
            >
              <el-option
                v-for="s in suppliers"
                :key="s.value"
                :label="s.label"
                :value="s.value"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="仓库" prop="warehouse_id">
            <el-select
              :model-value="form.warehouse_id"
              placeholder="请选择仓库"
              @update:model-value="(v: number | undefined) => (form.warehouse_id = v)"
            >
              <el-option
                v-for="w in warehouses"
                :key="w.value"
                :label="w.label"
                :value="w.value"
              />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="入库明细">
        <div class="items-table">
          <div class="items-header">
            <span class="col-product">产品</span>
            <span class="col-qty">数量</span>
            <span class="col-price">单价</span>
            <span class="col-amount">金额</span>
            <span class="col-action">操作</span>
          </div>
          <div v-for="(item, index) in (form.items || [])" :key="index" class="items-row">
            <el-select
              :model-value="item.product_id"
              placeholder="选择产品"
              class="col-product"
              @update:model-value="(v: number) => (item.product_id = v)"
            >
              <el-option
                v-for="p in products"
                :key="p.value"
                :label="p.label"
                :value="p.value"
              />
            </el-select>
            <el-input-number
              :model-value="item.quantity"
              class="col-qty"
              @update:model-value="(v: number | undefined) => {
                item.quantity = v ?? 0
                emit('calc-amount', item)
              }"
            />
            <el-input-number
              :model-value="item.price"
              :precision="2"
              class="col-price"
              @update:model-value="(v: number | undefined) => {
                item.price = v ?? 0
                emit('calc-amount', item)
              }"
            />
            <el-input-number
              :model-value="item.amount"
              :precision="2"
              class="col-amount"
              readonly
            />
            <el-button
              v-if="(form.items || []).length > 1"
              size="small"
              type="danger"
              @click="emit('remove-item', index)"
              >删除</el-button
            >
          </div>
          <el-button type="text" @click="emit('add-item')">+ 添加明细</el-button>
        </div>
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">取消</el-button>
      <el-button type="primary" @click="onSubmit">确定</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import { ref } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'
import type { ReceiptItem } from '@/api/purchaseReceipt'

// 选项类型
interface OptItem {
  label: string
  value: number
}

// 表单类型
interface PrcFormModel {
  id?: number
  receipt_no?: string
  receipt_date?: string
  supplier_id?: number
  warehouse_id?: number
  status?: string
  items?: ReceiptItem[]
  [key: string]: unknown
}

const props = defineProps<{
  // 对话框可见性
  visible: boolean
  // 对话框标题
  title: string
  // 表单数据
  form: PrcFormModel
  // 校验规则
  rules: FormRules
  // 供应商选项
  suppliers: OptItem[]
  // 仓库选项
  warehouses: OptItem[]
  // 产品选项
  products: OptItem[]
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  'add-item': []
  'remove-item': [index: number]
  'calc-amount': [item: ReceiptItem]
  submit: []
}>()

// 表单 ref
const formRef = ref<FormInstance>()

/** 点击确定：先校验再发 submit */
const onSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async (valid: boolean) => {
    if (!valid) return
    emit('submit')
  })
}
</script>

<style scoped>
.items-table {
  border: 1px solid #ebeef5;
  border-radius: 4px;
}
.items-header {
  display: flex;
  background: #f5f7fa;
  padding: 10px;
  font-weight: bold;
}
.items-row {
  display: flex;
  padding: 10px;
  border-top: 1px solid #ebeef5;
}
.col-product {
  flex: 2;
  margin-right: 10px;
}
.col-qty,
.col-price,
.col-amount {
  width: 100px;
  margin-right: 10px;
}
.col-action {
  width: 60px;
}
</style>

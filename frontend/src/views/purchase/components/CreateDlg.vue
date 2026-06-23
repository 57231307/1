<!--
  CreateDlg - 新建采购单对话框
  任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue 新建采购单对话框）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
-->
<template>
  <el-dialog
    :model-value="modelValue"
    title="新建采购单"
    width="800px"
    @update:model-value="(v: boolean) => emit('update:modelValue', v)"
  >
    <el-form
      ref="formRef"
      :model="localForm"
      :rules="rules"
      label-width="100px"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="供应商" required>
            <el-select
              v-model="localForm.supplier_id"
              placeholder="选择供应商"
              style="width: 100%"
            >
              <el-option
                v-for="s in suppliers"
                :key="s.id"
                :label="s.supplier_name"
                :value="s.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="订单日期" required>
            <el-date-picker
              v-model="localForm.order_date"
              type="date"
              placeholder="选择日期"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="要求交货日期">
            <el-date-picker
              v-model="localForm.required_date"
              type="date"
              placeholder="选择日期"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="备注">
            <el-input v-model="localForm.remark" placeholder="请输入备注" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="采购明细">
        <div class="items-table">
          <div class="items-header">
            <span class="col-product">产品</span>
            <span class="col-qty">数量</span>
            <span class="col-price">单价</span>
            <span class="col-amount">金额</span>
            <span class="col-action">操作</span>
          </div>
          <div v-for="(item, index) in localForm.items" :key="index" class="items-row">
            <el-select
              v-model="item.product_id"
              placeholder="选择产品"
              class="col-product"
              @change="onProductSelect(index)"
            >
              <el-option
                v-for="p in products"
                :key="p.id"
                :label="p.product_name"
                :value="p.id"
              />
            </el-select>
            <el-input-number
              v-model="item.quantity"
              :min="1"
              class="col-qty"
              @change="onCalculateSubtotal(item)"
            />
            <el-input-number
              v-model="item.unit_price"
              :min="0"
              :precision="2"
              class="col-price"
              @change="onCalculateSubtotal(item)"
            />
            <el-input-number v-model="item.subtotal" :precision="2" class="col-amount" readonly />
            <el-button
              v-if="localForm.items.length > 1"
              size="small"
              type="danger"
              @click="onRemoveItem(index)"
              >删除</el-button
            >
          </div>
          <el-button type="text" @click="onAddItem">+ 添加明细</el-button>
        </div>
      </el-form-item>
      <el-form-item label="合计金额">
        <span class="total-amount">¥{{ calculateTotal().toLocaleString() }}</span>
      </el-form-item>
    </el-form>
    <template #footer>
      <span class="dialog-footer">
        <el-button @click="onCancel">取消</el-button>
        <el-button type="primary" @click="onSubmit">确定</el-button>
      </span>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import type { Supplier } from '@/api/supplier'
import type { Product } from '@/api/product'
import type { CreateFormData } from '../composables/useCreate'

const props = defineProps<{
  // 对话框可见性
  modelValue: boolean
  // 表单数据（由父组件管理，子组件通过 emit('update:form') 回写）
  form: CreateFormData
  // 校验规则
  rules: any
  // 供应商列表
  suppliers: Supplier[]
  // 产品列表
  products: Product[]
  // 表单 ref
  formRef: any
  // 提交
  onSubmit: () => void
  // 取消
  onCancel: () => void
  // 添加明细
  onAddItem: () => void
  // 删除明细
  onRemoveItem: (index: number) => void
  // 选择产品
  onProductSelect: (index: number) => void
  // 重算小计
  onCalculateSubtotal: (item: any) => void
  // 计算总金额
  calculateTotal: () => number
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
  // 整体回写表单（父组件监听此事件并回写到自己的 form.value）
  (e: 'update:form', form: CreateFormData): void
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
// 注意：表单内有 items 数组，需要深拷贝以保证本地修改与父组件解耦
const localForm = ref<CreateFormData>(JSON.parse(JSON.stringify(props.form)))

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local（如父组件点击"新建"重置表单）
watch(
  () => props.form,
  (newForm) => {
    if (syncing) return
    syncing = true
    localForm.value = JSON.parse(JSON.stringify(newForm))
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
    emit('update:form', JSON.parse(JSON.stringify(newForm)))
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)
</script>

<style scoped>
.items-table {
  width: 100%;
}

.items-header {
  display: flex;
  gap: 8px;
  padding: 8px 0;
  font-weight: 600;
  color: #303133;
  border-bottom: 1px solid #ebeef5;
}

.items-row {
  display: flex;
  gap: 8px;
  align-items: center;
  padding: 8px 0;
  border-bottom: 1px solid #f0f0f0;
}

.col-product {
  flex: 1;
  min-width: 200px;
}

.col-qty,
.col-price,
.col-amount {
  width: 110px;
}

.total-amount {
  font-size: 18px;
  font-weight: 700;
  color: #f56c6c;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>

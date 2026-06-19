<!--
  LgsForm.vue - 物流管理新建/编辑运单对话框
  拆分自 logistics/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-dialog
    :model-value="visible"
    :title="isEdit ? '编辑运单' : '新建运单'"
    width="600px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form ref="formRef" :model="form" :rules="rules" label-width="100px">
      <el-form-item label="关联订单" prop="order_id">
        <el-select
          :model-value="form.order_id"
          placeholder="选择关联订单"
          filterable
          @update:model-value="(v: number | undefined) => (form.order_id = v)"
        >
          <el-option
            v-for="order in orders"
            :key="order.id"
            :label="order.order_no"
            :value="order.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item label="物流公司" prop="logistics_company">
        <el-select
          :model-value="form.logistics_company"
          placeholder="选择物流公司"
          @update:model-value="(v: string) => (form.logistics_company = v)"
        >
          <el-option label="顺丰速运" value="顺丰速运" />
          <el-option label="中通快递" value="中通快递" />
          <el-option label="圆通速递" value="圆通速递" />
          <el-option label="韵达快递" value="韵达快递" />
          <el-option label="京东物流" value="京东物流" />
        </el-select>
      </el-form-item>
      <el-form-item label="快递单号" prop="tracking_number">
        <el-input
          :model-value="form.tracking_number"
          placeholder="请输入快递单号"
          @update:model-value="(v: string) => (form.tracking_number = v)"
        />
      </el-form-item>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="司机姓名">
            <el-input
              :model-value="form.driver_name"
              placeholder="请输入司机姓名"
              @update:model-value="(v: string) => (form.driver_name = v)"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="司机电话">
            <el-input
              :model-value="form.driver_phone"
              placeholder="请输入司机电话"
              @update:model-value="(v: string) => (form.driver_phone = v)"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="运费">
            <el-input-number
              :model-value="form.freight_fee"
              :min="0"
              :precision="2"
              @update:model-value="(v: number | undefined) => (form.freight_fee = v ?? 0)"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="预计到达">
            <el-date-picker
              :model-value="form.expected_arrival"
              type="date"
              placeholder="选择预计到达日期"
              value-format="YYYY-MM-DD"
              @update:model-value="(v: string) => (form.expected_arrival = v)"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="备注">
        <el-input
          :model-value="form.notes"
          type="textarea"
          :rows="3"
          placeholder="请输入备注"
          @update:model-value="(v: string) => (form.notes = v)"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">取消</el-button>
      <el-button type="primary" :loading="loading" @click="onSubmit">确定</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import { ref } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'

// 订单选项类型
interface OrderOption {
  id: number
  order_no: string
}

// 订单表单字段类型
interface LgsForm {
  id?: number | undefined
  order_id?: number | undefined
  logistics_company?: string
  tracking_number?: string
  driver_name?: string
  driver_phone?: string
  freight_fee?: number
  expected_arrival?: string
  notes?: string
}

const props = defineProps<{
  // 对话框可见性
  visible: boolean
  // 是否编辑模式
  isEdit: boolean
  // 提交 loading
  loading: boolean
  // 关联订单
  orders: OrderOption[]
  // 表单数据
  form: LgsForm
  // 校验规则
  rules: FormRules
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
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

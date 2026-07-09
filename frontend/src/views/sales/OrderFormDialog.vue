<!--
  OrderFormDialog.vue - 销售订单表单对话框
  来源：原 sales/index.vue 中 订单表单 dialog
  拆分日期：2026-06-15 B3-1
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="title"
    width="900px"
    destroy-on-close
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form ref="formRef" :model="localData" :rules="formRules" label-width="100px">
      <el-divider content-position="left">基本信息</el-divider>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="客户" prop="customer_id">
            <el-select
              v-model="localData.customer_id"
              placeholder="选择客户"
              style="width: 100%"
              @change="handleCustomerChange"
            >
              <el-option
                v-for="c in customers"
                :key="c.id"
                :label="c.customer_name"
                :value="c.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="订单日期" prop="order_date">
            <el-date-picker
              v-model="localData.order_date"
              type="date"
              placeholder="选择日期"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="要求交货日期" prop="required_date">
            <el-date-picker
              v-model="localData.required_date"
              type="date"
              placeholder="选择日期"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="联系人" prop="contact_person">
            <el-input v-model="localData.contact_person" placeholder="联系人姓名" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="联系电话" prop="contact_phone">
        <el-input v-model="localData.contact_phone" placeholder="联系电话" style="width: 50%" />
      </el-form-item>
      <el-form-item label="收货地址" prop="delivery_address">
        <el-input
          v-model="localData.delivery_address"
          type="textarea"
          :rows="2"
          placeholder="详细收货地址"
        />
      </el-form-item>

      <el-divider content-position="left">订单明细</el-divider>
      <div class="order-items">
        <el-table :data="localData.items" border style="width: 100%">
          <el-table-column label="产品" width="200">
            <template #default="{ row, $index }">
              <el-select
                v-model="row.product_id"
                placeholder="选择产品"
                @change="(v: number) => handleProductSelect($index, v)"
              >
                <el-option
                  v-for="p in products"
                  :key="p.id"
                  :label="p.product_name"
                  :value="p.id"
                />
              </el-select>
            </template>
          </el-table-column>
          <el-table-column prop="quantity" label="数量" width="120">
            <template #default="{ row }">
              <el-input-number
                v-model="row.quantity"
                :min="1"
                size="small"
                @change="() => calculateSubtotal(row)"
              />
            </template>
          </el-table-column>
          <el-table-column prop="unit" label="单位" width="80" />
          <el-table-column prop="unit_price" label="单价" width="120">
            <template #default="{ row }">
              <el-input-number
                v-model="row.unit_price"
                :min="0"
                :precision="2"
                size="small"
                @change="() => calculateSubtotal(row)"
              />
            </template>
          </el-table-column>
          <el-table-column prop="subtotal" label="小计" width="120">
            <template #default="{ row }">
              <span class="amount">¥{{ (row.subtotal || 0).toLocaleString() }}</span>
            </template>
          </el-table-column>
          <el-table-column label="操作" width="80">
            <template #default="{ $index }">
              <el-button type="danger" link size="small" @click="removeItem($index)"
                >删除</el-button
              >
            </template>
          </el-table-column>
        </el-table>
        <el-button type="primary" plain size="small" style="margin-top: 10px" @click="addItem">
          <el-icon><Plus /></el-icon> 添加明细
        </el-button>
      </div>

      <el-divider content-position="left">其他信息</el-divider>
      <el-form-item label="备注">
        <el-input v-model="localData.remark" type="textarea" :rows="3" placeholder="订单备注信息" />
      </el-form-item>
      <el-row :gutter="20">
        <el-col :span="8">
          <el-form-item label="订单总额">
            <div class="total-amount">¥{{ calculateTotal().toLocaleString() }}</div>
          </el-form-item>
        </el-col>
        <el-col :span="8">
          <el-form-item label="税额">
            <div class="total-amount">¥{{ (calculateTotal() * 0.13).toLocaleString() }}</div>
          </el-form-item>
        </el-col>
        <el-col :span="8">
          <el-form-item label="价税合计">
            <div class="total-amount highlight">
              ¥{{ (calculateTotal() * 1.13).toLocaleString() }}
            </div>
          </el-form-item>
        </el-col>
      </el-row>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">取消</el-button>
      <el-button type="primary" :loading="submitting" @click="handleSubmit">确定</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { deepClone } from '@/utils'
import { ref, reactive, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import type { FormInstance, FormRules } from 'element-plus'
import type { Customer } from '@/api/customer'
import type { Product } from '@/api/product'

interface OrderItemForm {
  id: number
  product_id: number | undefined
  product_name: string
  product_code: string
  quantity: number
  unit: string
  unit_price: number
  subtotal: number
}

interface OrderForm {
  id?: number
  customer_id: number | undefined
  customer_name: string
  order_date: Date | string
  required_date: string
  contact_person: string
  contact_phone: string
  delivery_address: string
  remark: string
  items: OrderItemForm[]
  total_amount?: number
}

const props = defineProps<{
  visible: boolean
  title: string
  formData: OrderForm
  customers: Customer[]
  products: Product[]
  submitting?: boolean
}>()

const emit = defineEmits<{
  'update:visible': [value: boolean]
  submit: [data: OrderForm]
}>()

const formRef = ref<FormInstance>()

// 本地数据副本：避免 vue/no-mutating-props 警告；
// watch 监听 props.formData 变化以同步父组件重置
const localData = reactive<OrderForm>({
  id: 0,
  customer_id: undefined,
  customer_name: '',
  order_date: new Date(),
  required_date: '',
  contact_person: '',
  contact_phone: '',
  delivery_address: '',
  remark: '',
  items: [],
  total_amount: 0,
})

watch(
  () => props.formData,
  newData => {
    Object.assign(localData, deepClone(newData))
  },
  { deep: true, immediate: true }
)

const formRules: FormRules = {
  customer_id: [{ required: true, message: '请选择客户', trigger: 'change' }],
  order_date: [{ required: true, message: '请选择订单日期', trigger: 'change' }],
  required_date: [{ required: true, message: '请选择要求交货日期', trigger: 'change' }],
  contact_person: [{ required: true, message: '请输入联系人', trigger: 'blur' }],
  contact_phone: [
    { required: true, message: '请输入联系电话', trigger: 'blur' },
    { pattern: /^1[3-9]\d{9}$/, message: '请输入正确的手机号码', trigger: 'blur' },
  ],
  delivery_address: [{ required: true, message: '请输入收货地址', trigger: 'blur' }],
}

const handleCustomerChange = (customerId: number) => {
  const customer = props.customers.find(c => c.id === customerId)
  if (customer) {
    localData.customer_name = customer.customer_name
  }
}

const handleProductSelect = (index: number, _v: number) => {
  const product = props.products.find(p => p.id === localData.items[index].product_id)
  if (product) {
    localData.items[index].product_name = product.product_name
    localData.items[index].product_code = product.product_code
    localData.items[index].unit_price = product.price || 0
    calculateSubtotal(localData.items[index])
  }
}

const calculateSubtotal = (item: OrderItemForm) => {
  item.subtotal = (item.quantity || 0) * (item.unit_price || 0)
}

const calculateTotal = () => {
  return localData.items.reduce((sum, item) => sum + (item.subtotal || 0), 0)
}

const addItem = () => {
  localData.items.push({
    id: Date.now(),
    product_id: undefined,
    product_name: '',
    product_code: '',
    quantity: 1,
    unit: '米',
    unit_price: 0,
    subtotal: 0,
  })
}

const removeItem = (index: number) => {
  if (localData.items.length > 1) {
    localData.items.splice(index, 1)
  } else {
    ElMessage.warning('至少保留一条明细')
  }
}

const handleSubmit = async () => {
  if (!formRef.value) return
  try {
    await formRef.value.validate()
    const validItems = localData.items.filter(
      item => item.product_id && item.quantity > 0 && item.unit_price > 0
    )
    if (validItems.length === 0) {
      ElMessage.warning('请至少添加一条有效的订单明细')
      return
    }
    localData.total_amount = calculateTotal()
    emit('submit', localData)
  } catch (error) {
    const err = error as { message?: string }
    if (err.message) {
      ElMessage.error(err.message || '操作失败')
    }
  }
}
</script>

<style scoped>
.order-items {
  margin-bottom: 20px;
}
.amount {
  font-weight: 600;
  color: #f56c6c;
}
.total-amount {
  font-size: 20px;
  font-weight: 700;
  color: #303133;
}
.total-amount.highlight {
  color: #f56c6c;
}
</style>

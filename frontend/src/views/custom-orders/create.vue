<!--
  定制订单创建页
  - 客户/产品/色号选择
  - 定制要求（JSONB）
  - 工艺路线
-->
<template>
  <div class="custom-order-create">
    <el-card>
      <template #header>
        <div class="card-header">
          <span class="title">新建定制订单</span>
          <el-button @click="$router.back()">返回</el-button>
        </div>
      </template>

      <el-form :model="form" :rules="rules" ref="formRef" label-width="120px">
        <el-form-item label="客户" prop="customer_id">
          <el-input-number v-model="form.customer_id" :min="1" placeholder="客户 ID" />
        </el-form-item>
        <el-form-item label="产品" prop="product_id">
          <el-input-number v-model="form.product_id" :min="1" placeholder="产品 ID" />
        </el-form-item>
        <el-form-item label="色号">
          <el-input-number v-model="form.color_id" :min="1" placeholder="色号 ID（可选）" />
        </el-form-item>
        <el-form-item label="规格" prop="spec">
          <el-input v-model="form.spec" placeholder="例如：100% 棉 200g/m² 幅宽 1.5m" />
        </el-form-item>
        <el-form-item label="数量" prop="quantity">
          <el-input-number v-model="form.quantity" :min="0.01" :precision="2" :step="1" />
          <el-select v-model="form.unit" style="width: 100px; margin-left: 8px">
            <el-option label="米 (m)" value="m" />
            <el-option label="千克 (kg)" value="kg" />
            <el-option label="件 (pcs)" value="pcs" />
          </el-select>
        </el-form-item>
        <el-form-item label="纱线规格">
          <el-input v-model="form.yarn_spec" placeholder="例如：32S 精梳" />
        </el-form-item>
        <el-form-item label="染色方法">
          <el-select v-model="form.dye_method" clearable placeholder="选择染色方法">
            <el-option label="活性染料" value="reactive" />
            <el-option label="分散染料" value="disperse" />
            <el-option label="还原染料" value="vat" />
            <el-option label="酸性染料" value="acid" />
          </el-select>
        </el-form-item>
        <el-form-item label="后整理">
          <el-select v-model="form.finishing_method" clearable placeholder="选择后整理方法">
            <el-option label="柔软整理" value="softening" />
            <el-option label="防水整理" value="waterproof" />
            <el-option label="阻燃整理" value="flame_retardant" />
            <el-option label="免烫整理" value="easy_care" />
          </el-select>
        </el-form-item>
        <el-form-item label="期望交付">
          <el-date-picker
            v-model="form.expected_delivery_date"
            type="date"
            value-format="YYYY-MM-DD"
            placeholder="选择期望交付日期"
          />
        </el-form-item>
        <el-form-item label="总金额">
          <el-input-number v-model="form.total_amount" :min="0" :precision="2" :step="100" />
          <el-select v-model="form.currency" style="width: 100px; margin-left: 8px">
            <el-option label="CNY" value="CNY" />
            <el-option label="USD" value="USD" />
            <el-option label="EUR" value="EUR" />
          </el-select>
        </el-form-item>
        <el-form-item label="关联销售订单">
          <el-input-number v-model="form.sales_order_id" :min="1" placeholder="销售订单 ID（可选）" />
        </el-form-item>
        <el-form-item label="定制要求">
          <el-input
            v-model="customReqText"
            type="textarea"
            :rows="3"
            placeholder="例如：特殊克重 220g/m²，幅宽 1.6m，要求 4 级色牢度"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :loading="submitting" @click="handleSubmit">保存草稿</el-button>
          <el-button @click="$router.back()">取消</el-button>
        </el-form-item>
      </el-form>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { createCustomOrder } from '@/api/custom-order'

const router = useRouter()
const formRef = ref()
const submitting = ref(false)
const customReqText = ref('')

const form = ref({
  customer_id: undefined as number | undefined,
  product_id: undefined as number | undefined,
  color_id: undefined as number | undefined,
  spec: '',
  quantity: 1,
  unit: 'm',
  yarn_spec: '',
  dye_method: '',
  finishing_method: '',
  expected_delivery_date: '',
  total_amount: undefined as number | undefined,
  currency: 'CNY',
  sales_order_id: undefined as number | undefined,
})

const rules = {
  customer_id: [{ required: true, message: '客户必填', trigger: 'blur' }],
  product_id: [{ required: true, message: '产品必填', trigger: 'blur' }],
  spec: [{ required: true, message: '规格必填', trigger: 'blur' }],
  quantity: [{ required: true, message: '数量必填', trigger: 'blur' }],
}

async function handleSubmit() {
  if (!formRef.value) return
  try {
    await formRef.value.validate()
  } catch {
    return
  }

  submitting.value = true
  try {
    const custom_requirements = customReqText.value
      ? { note: customReqText.value }
      : null

    // P2-9a 修复配套：表单验证通过后 narrowing 必填字段，满足 CustomOrderCreateDto 类型
    if (!form.value.customer_id || !form.value.product_id) {
      throw new Error('客户和产品为必填项')
    }
    const payload = {
      ...form.value,
      customer_id: form.value.customer_id,
      product_id: form.value.product_id,
      custom_requirements,
    }
    const res: any = await createCustomOrder(payload)
    const orderId = res.data?.id || res.id
    ElMessage.success('创建成功')
    router.push(`/custom-orders/${orderId}`)
  } catch (e: any) {
    ElMessage.error(e?.message || '创建失败')
  } finally {
    submitting.value = false
  }
}

onMounted(() => {
  // 可在此预加载客户/产品列表
})
</script>

<style scoped>
.custom-order-create {
  padding: 16px;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.title {
  font-size: 18px;
  font-weight: 600;
}
</style>

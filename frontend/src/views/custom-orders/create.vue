<!--
  定制订单创建页
  - 客户/产品/色号选择
  - 定制要求（JSONB）
  - 工艺路线
  D05 Batch 8 Group B：接入 useI18n
-->
<template>
  <div class="custom-order-create">
    <el-card>
      <template #header>
        <div class="card-header">
          <span class="title">{{ t('customOrders.create.title') }}</span>
          <el-button @click="$router.back()">{{ t('customOrders.create.buttonBack') }}</el-button>
        </div>
      </template>

      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        label-width="120px"
        :aria-label="t('customOrders.create.formAriaLabel')"
      >
        <el-form-item :label="t('customOrders.create.labelCustomer')" prop="customer_id">
          <el-input-number
            v-model="form.customer_id"
            :min="1"
            :placeholder="t('customOrders.create.placeholderCustomer')"
          />
        </el-form-item>
        <el-form-item :label="t('customOrders.create.labelProduct')" prop="product_id">
          <el-input-number
            v-model="form.product_id"
            :min="1"
            :placeholder="t('customOrders.create.placeholderProduct')"
          />
        </el-form-item>
        <el-form-item :label="t('customOrders.create.labelColor')">
          <el-input-number
            v-model="form.color_id"
            :min="1"
            :placeholder="t('customOrders.create.placeholderColor')"
          />
        </el-form-item>
        <el-form-item :label="t('customOrders.create.labelSpec')" prop="spec">
          <el-input v-model="form.spec" :placeholder="t('customOrders.create.placeholderSpec')" />
        </el-form-item>
        <el-form-item :label="t('customOrders.create.labelQuantity')" prop="quantity">
          <el-input-number v-model="form.quantity" :min="0.01" :precision="2" :step="1" />
          <el-select v-model="form.unit" style="width: 100px; margin-left: 8px">
            <el-option :label="t('customOrders.create.unitMeter')" value="m" />
            <el-option :label="t('customOrders.create.unitKilogram')" value="kg" />
            <el-option :label="t('customOrders.create.unitPiece')" value="pcs" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('customOrders.create.labelYarnSpec')">
          <el-input
            v-model="form.yarn_spec"
            :placeholder="t('customOrders.create.placeholderYarnSpec')"
          />
        </el-form-item>
        <el-form-item :label="t('customOrders.create.labelDyeMethod')">
          <el-select
            v-model="form.dye_method"
            clearable
            :placeholder="t('customOrders.create.placeholderDyeMethod')"
          >
            <el-option :label="t('customOrders.create.dyeMethodReactive')" value="reactive" />
            <el-option :label="t('customOrders.create.dyeMethodDisperse')" value="disperse" />
            <el-option :label="t('customOrders.create.dyeMethodVat')" value="vat" />
            <el-option :label="t('customOrders.create.dyeMethodAcid')" value="acid" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('customOrders.create.labelFinishingMethod')">
          <el-select
            v-model="form.finishing_method"
            clearable
            :placeholder="t('customOrders.create.placeholderFinishingMethod')"
          >
            <el-option :label="t('customOrders.create.finishingSoftening')" value="softening" />
            <el-option :label="t('customOrders.create.finishingWaterproof')" value="waterproof" />
            <el-option
              :label="t('customOrders.create.finishingFlameRetardant')"
              value="flame_retardant"
            />
            <el-option :label="t('customOrders.create.finishingEasyCare')" value="easy_care" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('customOrders.create.labelExpectedDelivery')">
          <el-date-picker
            v-model="form.expected_delivery_date"
            type="date"
            value-format="YYYY-MM-DD"
            :placeholder="t('customOrders.create.placeholderExpectedDelivery')"
          />
        </el-form-item>
        <el-form-item :label="t('customOrders.create.labelTotalAmount')">
          <el-input-number v-model="form.total_amount" :min="0" :precision="2" :step="100" />
          <el-select v-model="form.currency" style="width: 100px; margin-left: 8px">
            <el-option label="CNY" value="CNY" />
            <el-option label="USD" value="USD" />
            <el-option label="EUR" value="EUR" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('customOrders.create.labelSalesOrder')">
          <el-input-number
            v-model="form.sales_order_id"
            :min="1"
            :placeholder="t('customOrders.create.placeholderSalesOrder')"
          />
        </el-form-item>
        <el-form-item :label="t('customOrders.create.labelCustomReq')">
          <el-input
            v-model="customReqText"
            type="textarea"
            :rows="3"
            :placeholder="t('customOrders.create.placeholderCustomReq')"
          />
        </el-form-item>
        <!-- v3 复审 P1-4：新增订单备注输入控件 -->
        <el-form-item :label="t('customOrders.create.labelNotes')">
          <el-input
            v-model="form.notes"
            type="textarea"
            :rows="2"
            :placeholder="t('customOrders.create.placeholderNotes')"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :loading="submitting" @click="handleSubmit">{{
            t('customOrders.create.buttonSaveDraft')
          }}</el-button>
          <el-button @click="$router.back()">{{ t('customOrders.create.buttonCancel') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { createCustomOrder } from '@/api/custom-order'

const router = useRouter()
const { t } = useI18n({ useScope: 'global' })
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
  // v3 复审 P1-4：新增订单备注字段
  notes: '',
})

const rules = {
  customer_id: [
    {
      required: true,
      message: t('customOrders.create.validationCustomerRequired'),
      trigger: 'blur',
    },
  ],
  product_id: [
    {
      required: true,
      message: t('customOrders.create.validationProductRequired'),
      trigger: 'blur',
    },
  ],
  spec: [
    { required: true, message: t('customOrders.create.validationSpecRequired'), trigger: 'blur' },
  ],
  quantity: [
    {
      required: true,
      message: t('customOrders.create.validationQuantityRequired'),
      trigger: 'blur',
    },
  ],
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
    const custom_requirements = customReqText.value ? { note: customReqText.value } : null

    // P2-9a 修复配套：表单验证通过后 narrowing 必填字段，满足 CustomOrderCreateDto 类型
    if (!form.value.customer_id || !form.value.product_id) {
      throw new Error(t('customOrders.create.messageCustomerProductRequired'))
    }
    const payload = {
      ...form.value,
      customer_id: form.value.customer_id,
      product_id: form.value.product_id,
      custom_requirements,
    }
    const res = await createCustomOrder(payload)
    // P2-5：res.id 兼容历史取值（ApiResponse 无 id 字段），用断言保留运行时逻辑
    const orderId = res.data?.id || (res as unknown as { id?: number }).id
    ElMessage.success(t('customOrders.create.messageCreateSuccess'))
    router.push(`/custom-orders/${orderId}`)
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : String(e)
    ElMessage.error(msg || t('customOrders.create.messageCreateFailed'))
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

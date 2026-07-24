<!--
  面料多色号定价扩展 - 新建价格页
  提交后调用 color-price.ts:createColorPrice，成功后跳转到列表页
  创建时间: 2026-06-19
  关联 spec: docs/superpowers/specs/2026-06-16-color-price-extension-design.md §3.2
-->
<template>
  <div class="color-price-create">
    <el-card>
      <template #header>
        <span>{{ $t('colorPrices.create.title') }}</span>
      </template>

      <el-form
        :model="form"
        :rules="rules"
        ref="formRef"
        label-width="120px"
        style="max-width: 800px"
        :aria-label="$t('colorPrices.create.formAriaLabel')"
      >
        <el-form-item :label="$t('colorPrices.create.productId')" prop="product_id">
          <el-input v-model.number="form.product_id" :placeholder="$t('colorPrices.create.productId')" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.create.colorId')" prop="color_id">
          <el-input v-model.number="form.color_id" :placeholder="$t('colorPrices.create.colorId')" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.create.currency')" prop="currency">
          <el-select v-model="form.currency" :placeholder="$t('colorPrices.common.pleaseSelect')" style="width: 100%">
            <el-option :label="$t('colorPrices.currencyLabel.CNY')" value="CNY" />
            <el-option :label="$t('colorPrices.currencyLabel.USD')" value="USD" />
            <el-option :label="$t('colorPrices.currencyLabel.EUR')" value="EUR" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('colorPrices.create.basePrice')" prop="base_price">
          <el-input v-model.number="form.base_price" :placeholder="$t('colorPrices.create.basePrice')" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.create.effectiveFrom')" prop="effective_from">
          <el-date-picker
            v-model="form.effective_from"
            type="date"
            value-format="YYYY-MM-DD"
            :placeholder="$t('colorPrices.create.effectiveFromPlaceholder')"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.create.effectiveTo')">
          <el-date-picker
            v-model="form.effective_to"
            type="date"
            value-format="YYYY-MM-DD"
            :placeholder="$t('colorPrices.create.effectiveToPlaceholder')"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.create.customerLevel')">
          <el-select v-model="form.customer_level" :placeholder="$t('colorPrices.create.customerLevelPlaceholder')" clearable style="width: 100%">
            <el-option :label="$t('colorPrices.customerLevel.VIP')" value="VIP" />
            <el-option :label="$t('colorPrices.customerLevel.GOLD')" value="GOLD" />
            <el-option :label="$t('colorPrices.customerLevel.SILVER')" value="SILVER" />
            <el-option :label="$t('colorPrices.customerLevel.NORMAL')" value="NORMAL" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('colorPrices.create.season')">
          <el-select v-model="form.season" :placeholder="$t('colorPrices.create.seasonPlaceholder')" clearable style="width: 100%">
            <el-option :label="$t('colorPrices.season.SS')" value="SS" />
            <el-option :label="$t('colorPrices.season.AW')" value="AW" />
            <el-option :label="$t('colorPrices.season.HOLIDAY')" value="HOLIDAY" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('colorPrices.create.customerId')">
          <el-input v-model.number="form.customer_id" :placeholder="$t('colorPrices.create.customerIdPlaceholder')" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.create.minQuantity')">
          <el-input v-model.number="form.min_quantity" :placeholder="$t('colorPrices.create.minQuantityPlaceholder')" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.create.maxQuantity')">
          <el-input v-model.number="form.max_quantity" :placeholder="$t('colorPrices.create.maxQuantityPlaceholder')" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.create.priority')">
          <el-input-number v-model="form.priority" :min="0" :max="1000" style="width: 100%" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.create.notes')">
          <el-input v-model="form.notes" type="textarea" :rows="3" :placeholder="$t('colorPrices.create.notesPlaceholder')" />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :loading="submitting" @click="handleSubmit">{{ $t('colorPrices.create.submit') }}</el-button>
          <el-button @click="$router.back()">{{ $t('colorPrices.common.cancel') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>
  </div>
</template>

<script setup lang="ts">
// 新建色号价格页
// 提交逻辑：调用 createColorPrice，成功后跳转回列表页
import { ref, reactive, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { createColorPrice, type CreateColorPriceDto } from '@/api/color-price'

const { t } = useI18n({ useScope: 'global' })
const router = useRouter()
const formRef = ref<FormInstance>()
const submitting = ref(false)

// 表单状态：与 CreateColorPriceDto 对齐
const form = reactive<{
  product_id: number | undefined
  color_id: number | undefined
  currency: string
  base_price: number | undefined
  effective_from: string
  effective_to: string | null
  customer_level: string | null
  season: string | null
  customer_id: number | null
  min_quantity: number | null
  max_quantity: number | null
  priority: number
  notes: string | null
}>({
  product_id: undefined,
  color_id: undefined,
  currency: 'CNY',
  base_price: undefined,
  effective_from: '',
  effective_to: null,
  customer_level: null,
  season: null,
  customer_id: null,
  min_quantity: null,
  max_quantity: null,
  priority: 0,
  notes: null,
})

// 表单校验规则（响应式：随语言切换自动更新提示文案）
const rules = computed<FormRules>(() => ({
  product_id: [{ required: true, message: t('colorPrices.validation.productIdRequired'), trigger: 'blur' }],
  color_id: [{ required: true, message: t('colorPrices.validation.colorIdRequired'), trigger: 'blur' }],
  currency: [{ required: true, message: t('colorPrices.validation.currencyRequired'), trigger: 'change' }],
  base_price: [{ required: true, message: t('colorPrices.validation.basePriceRequired'), trigger: 'blur' }],
  effective_from: [{ required: true, message: t('colorPrices.validation.effectiveFromRequired'), trigger: 'change' }],
}))

// 提交处理：调用创建 API，成功后跳转到列表页
const handleSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async (valid) => {
    if (!valid) return
    submitting.value = true
    try {
      const payload: CreateColorPriceDto = {
        product_id: form.product_id!,
        color_id: form.color_id!,
        currency: form.currency,
        base_price: form.base_price!,
        effective_from: form.effective_from,
        effective_to: form.effective_to,
        customer_level: form.customer_level,
        season: form.season,
        customer_id: form.customer_id,
        min_quantity: form.min_quantity,
        max_quantity: form.max_quantity,
        priority: form.priority,
        notes: form.notes,
      }
      await createColorPrice(payload)
      ElMessage.success(t('colorPrices.message.createSuccess'))
      router.push('/color-prices/list')
    } catch (e: unknown) {
      ElMessage.error(t('colorPrices.message.createFailed', { msg: e instanceof Error ? e.message : t('colorPrices.message.unknownError') }))
    } finally {
      submitting.value = false
    }
  })
}
</script>

<style scoped>
.color-price-create { padding: 20px; }
</style>

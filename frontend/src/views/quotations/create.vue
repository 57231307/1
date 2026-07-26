<!--
  新建报价单页
  - 表单：客户/日期/价格条款/币种/汇率/含税/客户等级/MOQ/交期
  - 明细：QuotationItemEditor（产品/色号/数量/单价/含税）
  - 条款：TermEditor（4 类贸易条款）
  - 操作：保存草稿 / 提交审批
-->
<template>
  <div class="quotation-create">
    <el-card>
      <template #header>
        <div class="card-header">
          <span class="title">{{
            isEdit ? t('quotations.create.titleEdit') : t('quotations.create.titleCreate')
          }}</span>
          <el-button @click="$router.back()">{{ t('quotations.create.back') }}</el-button>
        </div>
      </template>

      <el-form
        ref="formRef"
        v-loading="loading"
        :model="form"
        :rules="rules"
        label-width="120px"
        :aria-label="t('quotations.create.formAriaLabel')"
      >
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('quotations.create.labelCustomer')" prop="customer_id">
              <el-select
                v-model="form.customer_id"
                filterable
                :placeholder="t('quotations.create.selectCustomer')"
                style="width: 100%"
              >
                <el-option
                  v-for="c in customers"
                  :key="c.id"
                  :label="c.customer_name || c.name"
                  :value="c.id"
                />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('quotations.create.labelQuotationDate')" prop="quotation_date">
              <el-date-picker
                v-model="form.quotation_date"
                type="date"
                value-format="YYYY-MM-DD"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>

        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('quotations.create.labelValidUntil')" prop="valid_until">
              <el-date-picker
                v-model="form.valid_until"
                type="date"
                value-format="YYYY-MM-DD"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('quotations.create.labelPriceTerms')" prop="price_terms">
              <el-select
                v-model="form.price_terms"
                placeholder="Incoterms 2020"
                style="width: 100%"
              >
                <el-option
                  v-for="(label, value) in PRICE_TERMS_LABELS"
                  :key="value"
                  :label="label"
                  :value="value"
                />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>

        <el-row :gutter="20">
          <el-col :span="8">
            <el-form-item :label="t('quotations.create.labelCurrency')" prop="currency">
              <el-select v-model="form.currency" style="width: 100%">
                <el-option :label="t('quotations.create.currencyCny')" value="CNY" />
                <el-option :label="t('quotations.create.currencyUsd')" value="USD" />
                <el-option :label="t('quotations.create.currencyEur')" value="EUR" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item :label="t('quotations.create.labelExchangeRate')" prop="exchange_rate">
              <el-input-number
                v-model="form.exchange_rate"
                :min="0"
                :precision="6"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item :label="t('quotations.create.labelTaxInclusive')">
              <el-switch v-model="form.tax_inclusive" />
            </el-form-item>
          </el-col>
        </el-row>

        <el-row :gutter="20">
          <el-col :span="8">
            <el-form-item :label="t('quotations.create.labelCustomerLevel')">
              <el-select v-model="form.customer_level" clearable style="width: 100%">
                <el-option label="VIP" value="VIP" />
                <el-option :label="t('quotations.create.customerLevelNormal')" value="NORMAL" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="MOQ">
              <el-input-number v-model="form.moq" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item :label="t('quotations.create.labelLeadTime')">
              <el-input-number v-model="form.lead_time_days" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>

        <h3 class="section-title">{{ t('quotations.create.sectionItems') }}</h3>
        <QuotationItemEditor v-model="form.items" :currency="form.currency" />

        <h3 class="section-title">{{ t('quotations.create.sectionTerms') }}</h3>
        <TermEditor :model-value="form.terms || []" @update:model-value="onTermsChange" />

        <el-form-item :label="t('quotations.create.labelRemark')" style="margin-top: 16px">
          <el-input
            v-model="form.notes"
            type="textarea"
            :rows="3"
            :placeholder="t('quotations.create.remarkPlaceholder')"
          />
        </el-form-item>

        <!-- 金额合计 -->
        <div class="totals">
          <span
            >{{ t('quotations.create.subtotal') }}{{ form.currency }}
            {{ formatAmount(subtotal) }}</span
          >
          <span
            >{{ t('quotations.create.taxAmount') }}{{ form.currency }}
            {{ formatAmount(taxAmount) }}</span
          >
          <span class="grand-total"
            >{{ t('quotations.create.total') }}{{ form.currency }}
            {{ formatAmount(totalAmount) }}</span
          >
        </div>

        <el-form-item>
          <el-button :loading="submitting" @click="handleSaveDraft">{{
            t('quotations.create.saveDraft')
          }}</el-button>
          <el-button type="primary" :loading="submitting" @click="handleSubmit">
            {{ t('quotations.create.submitApproval') }}
          </el-button>
          <el-button @click="$router.back()">{{ t('quotations.create.cancel') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>
  </div>
</template>

<script setup lang="ts">
// 新建/编辑报价单页脚本
// - 接受 quotationId prop 时为编辑模式，否则为新建
// - 加载客户列表
// - 提交保存草稿 / 提交审批
import { ref, reactive, computed, onMounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import {
  createQuotation,
  updateQuotation,
  submitQuotation,
  getQuotation,
  PRICE_TERMS_LABELS,
  type CreateQuotationDto,
  type CreateQuotationItemDto,
  type CreateQuotationTermDto,
  type PriceTerms,
  type CurrencyCode,
  type CustomerLevel,
} from '@/api/quotation'
import { getCustomerList } from '@/api/customer'
import { useUserStore } from '@/store/user'
import QuotationItemEditor from './components/QuotationItemEditor.vue'
import TermEditor from './components/TermEditor.vue'

const { t } = useI18n({ useScope: 'global' })

const props = defineProps<{
  quotationId?: number | string
}>()

const router = useRouter()
const route = useRoute()
const userStore = useUserStore()
const formRef = ref<FormInstance>()
const loading = ref(false)
const submitting = ref(false)

const isEdit = computed(() => !!props.quotationId || !!route.params.id)

/** 当前日期 YYYY-MM-DD */
function todayStr(): string {
  return new Date().toISOString().slice(0, 10)
}

/** 默认 30 天后 */
function defaultValidUntil(): string {
  return new Date(Date.now() + 30 * 24 * 60 * 60 * 1000).toISOString().slice(0, 10)
}

/** 表单数据 */
const form = reactive<CreateQuotationDto>({
  // v11 批次 163 P2-1 修复：undefined as any 改为类型断言
  customer_id: undefined as unknown as number,
  sales_user_id: 0,
  quotation_date: todayStr(),
  valid_until: defaultValidUntil(),
  currency: 'CNY',
  exchange_rate: 1.0,
  base_currency: 'CNY',
  price_terms: 'FOB',
  incoterms_version: '2020',
  incoterm_location: '',
  tax_inclusive: true,
  tax_rate: 13.0,
  moq: undefined,
  lead_time_days: undefined,
  customer_level: 'NORMAL',
  notes: '',
  items: [] as CreateQuotationItemDto[],
  terms: [] as CreateQuotationTermDto[],
})

/** 表单校验规则 */
const rules: FormRules = {
  customer_id: [
    { required: true, message: t('quotations.create.validateCustomer'), trigger: 'change' },
  ],
  quotation_date: [
    { required: true, message: t('quotations.create.validateQuotationDate'), trigger: 'change' },
  ],
  valid_until: [
    { required: true, message: t('quotations.create.validateValidUntil'), trigger: 'change' },
  ],
  price_terms: [
    { required: true, message: t('quotations.create.validatePriceTerms'), trigger: 'change' },
  ],
  currency: [
    { required: true, message: t('quotations.create.validateCurrency'), trigger: 'change' },
  ],
  exchange_rate: [
    { required: true, message: t('quotations.create.validateExchangeRate'), trigger: 'blur' },
  ],
  items: [
    {
      // v11 批次 163 P2-1 修复：validator 参数类型化（FormItemRule validator 签名）
      validator: (_rule: unknown, value: CreateQuotationItemDto[], cb: (error?: Error) => void) => {
        if (!value || value.length === 0) {
          cb(new Error(t('quotations.create.validateItemsRequired')))
          return
        }
        const invalid = value.find(i => !i.product_id || i.quantity <= 0 || i.unit_price < 0)
        if (invalid) {
          cb(new Error(t('quotations.create.validateItemsInvalid')))
          return
        }
        cb()
      },
      trigger: 'change',
    },
  ],
}

const customers = ref<Array<{ id: number; customer_name?: string; name?: string }>>([])

/** 金额计算 */
const subtotal = computed(() =>
  form.items.reduce(
    (sum: number, i: CreateQuotationItemDto) => sum + (i.quantity || 0) * (i.unit_price || 0),
    0
  )
)
const taxAmount = computed(() => (form.tax_inclusive ? 0 : (subtotal.value * form.tax_rate) / 100))
const totalAmount = computed(() => subtotal.value + taxAmount.value)

/** 加载客户下拉 */
async function loadCustomers() {
  try {
    const res = await getCustomerList({ page: 1, page_size: 1000 })
    // v11 批次 163 P2-1 修复：res.data as any 改为运行时安全访问
    const data = (res.data || {}) as { list?: unknown[]; items?: unknown[] }
    const list = data.list || data.items || []
    customers.value = list as { id: number; name: string }[]
  } catch {
    customers.value = []
  }
}

/** 编辑模式：加载已有数据 */
async function loadExisting() {
  const id = Number(props.quotationId || route.params.id)
  if (!id) return
  loading.value = true
  try {
    const res = await getQuotation(id)
    const data = res.data
    if (data) {
      Object.assign(form, {
        customer_id: data.customer_id,
        sales_user_id: data.sales_user_id,
        quotation_date: data.quotation_date,
        valid_until: data.valid_until,
        currency: data.currency as CurrencyCode,
        exchange_rate: Number(data.exchange_rate),
        base_currency: data.base_currency || 'CNY',
        price_terms: data.price_terms as PriceTerms,
        incoterms_version: data.incoterms_version || '2020',
        incoterm_location: data.incoterm_location || '',
        tax_inclusive: data.tax_inclusive,
        tax_rate: Number(data.tax_rate),
        moq: data.moq,
        lead_time_days: data.lead_time_days,
        customer_level: (data.customer_level as CustomerLevel) || 'NORMAL',
        notes: data.notes || '',
        items: (data.items || []) as CreateQuotationItemDto[],
        terms: (data.terms || []) as CreateQuotationTermDto[],
      })
    }
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('quotations.create.loadFailed')
    )
  } finally {
    loading.value = false
  }
}

/** 确保有 sales_user_id（默认当前用户） */
function ensureSalesUserId() {
  if (!form.sales_user_id && userStore.userInfo?.id) {
    form.sales_user_id = userStore.userInfo.id
  }
}

/** 保存草稿 */
async function handleSaveDraft() {
  if (!formRef.value) return
  try {
    await formRef.value.validate()
  } catch {
    ElMessage.error(t('quotations.create.validateForm'))
    return
  }
  ensureSalesUserId()
  submitting.value = true
  try {
    if (isEdit.value) {
      const id = Number(props.quotationId || route.params.id)
      const res = await updateQuotation(id, form)
      ElMessage.success(t('quotations.create.draftUpdated'))
      // v11 批次 163 P2-1 修复：res.data as any 改为 QuotationResponseDto
      router.push(`/quotations/${res.data?.id ?? id}`)
    } else {
      const res = await createQuotation(form)
      ElMessage.success(t('quotations.create.draftSaved'))
      router.push(`/quotations/${res.data?.id ?? ''}`)
    }
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('quotations.create.saveFailed')
    )
  } finally {
    submitting.value = false
  }
}

/** 提交审批 */
async function handleSubmit() {
  if (!formRef.value) return
  try {
    await formRef.value.validate()
  } catch {
    ElMessage.error(t('quotations.create.validateForm'))
    return
  }
  ensureSalesUserId()
  submitting.value = true
  try {
    let quotationId: number
    if (isEdit.value) {
      const id = Number(props.quotationId || route.params.id)
      const res = await updateQuotation(id, form)
      // v11 批次 163 P2-1 修复：res.data as any 改为 QuotationResponseDto
      quotationId = res.data?.id ?? id
    } else {
      const res = await createQuotation(form)
      quotationId = res.data?.id ?? 0
    }
    await submitQuotation(quotationId)
    ElMessage.success(t('quotations.create.submitSuccess'))
    router.push(`/quotations/${quotationId}`)
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('quotations.create.submitFailed')
    )
  } finally {
    submitting.value = false
  }
}

function formatAmount(value: number): string {
  return Number(value).toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })
}

/** 贸易条款变化（处理可选字段） */
function onTermsChange(value: CreateQuotationTermDto[]) {
  form.terms = value
}

onMounted(async () => {
  await loadCustomers()
  if (isEdit.value) {
    await loadExisting()
  }
})
</script>

<style scoped>
.quotation-create {
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
.section-title {
  margin: 24px 0 12px 0;
  font-size: 16px;
  font-weight: 600;
  color: #303133;
  border-left: 3px solid #409eff;
  padding-left: 8px;
}
.totals {
  text-align: right;
  margin: 20px 0;
  font-size: 15px;
  display: flex;
  justify-content: flex-end;
  gap: 24px;
}
.totals .grand-total {
  font-weight: bold;
  color: #f56c6c;
  font-size: 18px;
}
</style>

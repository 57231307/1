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
          <span class="title">{{ isEdit ? '编辑报价单' : '新建报价单' }}</span>
          <el-button @click="$router.back()">返回</el-button>
        </div>
      </template>

      <el-form ref="formRef" v-loading="loading" :model="form" :rules="rules" label-width="120px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="客户" prop="customer_id">
              <el-select
                v-model="form.customer_id"
                filterable
                placeholder="选择客户"
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
            <el-form-item label="报价日期" prop="quotation_date">
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
            <el-form-item label="有效期至" prop="valid_until">
              <el-date-picker
                v-model="form.valid_until"
                type="date"
                value-format="YYYY-MM-DD"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="价格条款" prop="price_terms">
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
            <el-form-item label="币种" prop="currency">
              <el-select v-model="form.currency" style="width: 100%">
                <el-option label="CNY 人民币" value="CNY" />
                <el-option label="USD 美元" value="USD" />
                <el-option label="EUR 欧元" value="EUR" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="汇率" prop="exchange_rate">
              <el-input-number
                v-model="form.exchange_rate"
                :min="0"
                :precision="6"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="含税">
              <el-switch v-model="form.tax_inclusive" />
            </el-form-item>
          </el-col>
        </el-row>

        <el-row :gutter="20">
          <el-col :span="8">
            <el-form-item label="客户等级">
              <el-select v-model="form.customer_level" clearable style="width: 100%">
                <el-option label="VIP" value="VIP" />
                <el-option label="NORMAL 普通" value="NORMAL" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="MOQ">
              <el-input-number v-model="form.moq" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="交期(天)">
              <el-input-number v-model="form.lead_time_days" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>

        <h3 class="section-title">报价明细</h3>
        <QuotationItemEditor v-model="form.items" :currency="form.currency" />

        <h3 class="section-title">贸易条款</h3>
        <TermEditor v-model="form.terms" />

        <el-form-item label="备注" style="margin-top: 16px">
          <el-input v-model="form.notes" type="textarea" :rows="3" placeholder="备注（选填）" />
        </el-form-item>

        <!-- 金额合计 -->
        <div class="totals">
          <span>小计：{{ form.currency }} {{ formatAmount(subtotal) }}</span>
          <span>税额：{{ form.currency }} {{ formatAmount(taxAmount) }}</span>
          <span class="grand-total">合计：{{ form.currency }} {{ formatAmount(totalAmount) }}</span>
        </div>

        <el-form-item>
          <el-button :loading="submitting" @click="handleSaveDraft">保存草稿</el-button>
          <el-button type="primary" :loading="submitting" @click="handleSubmit">
            提交审批
          </el-button>
          <el-button @click="$router.back()">取消</el-button>
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
  type Currency,
  type CustomerLevel,
} from '@/api/quotation'
import { listCustomers } from '@/api/customer'
import { useUserStore } from '@/store/user'
import QuotationItemEditor from './components/QuotationItemEditor.vue'
import TermEditor from './components/TermEditor.vue'

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
  customer_id: undefined as any,
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
  customer_id: [{ required: true, message: '请选择客户', trigger: 'change' }],
  quotation_date: [{ required: true, message: '请选择报价日期', trigger: 'change' }],
  valid_until: [{ required: true, message: '请选择有效期', trigger: 'change' }],
  price_terms: [{ required: true, message: '请选择价格条款', trigger: 'change' }],
  currency: [{ required: true, message: '请选择币种', trigger: 'change' }],
  exchange_rate: [{ required: true, message: '请输入汇率', trigger: 'blur' }],
  items: [
    {
      validator: (_rule: any, value: any[], cb: any) => {
        if (!value || value.length === 0) {
          cb(new Error('请至少添加 1 个产品明细'))
          return
        }
        const invalid = value.find((i: any) => !i.product_id || i.quantity <= 0 || i.unit_price < 0)
        if (invalid) {
          cb(new Error('明细行必须选择产品且数量 > 0'))
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
    const res = await listCustomers({ page: 1, page_size: 1000 })
    const data = (res.data as any) || {}
    customers.value = data.list || data.items || []
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
        currency: data.currency as Currency,
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
  } catch (e: any) {
    ElMessage.error(e?.message || '加载报价单失败')
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
    ElMessage.error('请检查表单填写是否正确')
    return
  }
  ensureSalesUserId()
  submitting.value = true
  try {
    if (isEdit.value) {
      const id = Number(props.quotationId || route.params.id)
      const res = await updateQuotation(id, form)
      ElMessage.success('草稿已更新')
      router.push(`/quotations/${(res.data as any).id}`)
    } else {
      const res = await createQuotation(form)
      ElMessage.success('草稿保存成功')
      router.push(`/quotations/${(res.data as any).id}`)
    }
  } catch (e: any) {
    ElMessage.error(e?.message || '保存失败')
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
    ElMessage.error('请检查表单填写是否正确')
    return
  }
  ensureSalesUserId()
  submitting.value = true
  try {
    let quotationId: number
    if (isEdit.value) {
      const id = Number(props.quotationId || route.params.id)
      const res = await updateQuotation(id, form)
      quotationId = (res.data as any).id
    } else {
      const res = await createQuotation(form)
      quotationId = (res.data as any).id
    }
    await submitQuotation(quotationId)
    ElMessage.success('已提交审批')
    router.push(`/quotations/${quotationId}`)
  } catch (e: any) {
    ElMessage.error(e?.message || '提交失败')
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

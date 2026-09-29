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
// - query.copyFrom 存在时为"复制为新单"：复用按 id 载入详情预填表头/明细，但仍保持新建态（isEdit=false），
//   单号由新建流程生成、明细/条款剥离源行 id，保存走 POST 生成新单，不覆盖源单
// - 加载客户列表
// - 提交保存草稿 / 提交审批
import { ref, reactive, computed, onMounted } from 'vue';
import { logger } from '@/utils/logger';
import { useRouter, useRoute } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { ElMessage, type FormInstance, type FormRules } from 'element-plus';
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
  type QuotationResponseDto,
  type QuotationItemResponseDto,
  type QuotationTermResponseDto,
} from '@/api/quotation';
import { getCustomerList } from '@/api/customer';
import { useUserStore } from '@/store/user';
import QuotationItemEditor from './components/QuotationItemEditor.vue';
import TermEditor from './components/TermEditor.vue';

const { t } = useI18n({ useScope: 'global' });

const props = defineProps<{
  quotationId?: number | string;
}>();

const router = useRouter();
const route = useRoute();
const userStore = useUserStore();
const formRef = ref<FormInstance>();
const loading = ref(false);
const submitting = ref(false);

const isEdit = computed(() => !!props.quotationId || !!route.params.id);

/** 复制来源报价单 id（来自 query.copyFrom，非编辑目标，故 isEdit 保持 false） */
const copyFromId = computed(() => {
  const raw = route.query.copyFrom;
  const value = Array.isArray(raw) ? raw[0] : raw;
  const id = Number(value);
  return Number.isFinite(id) && id > 0 ? id : 0;
});

/** 当前日期 YYYY-MM-DD */
function todayStr(): string {
  return new Date().toISOString().slice(0, 10);
}

/** 默认 30 天后 */
function defaultValidUntil(): string {
  return new Date(Date.now() + 30 * 24 * 60 * 60 * 1000).toISOString().slice(0, 10);
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
});

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
          cb(new Error(t('quotations.create.validateItemsRequired')));
          return;
        }
        const invalid = value.find(
          i => !i.product_id || !i.unit || i.quantity <= 0 || i.unit_price < 0
        );
        if (invalid) {
          cb(new Error(t('quotations.create.validateItemsInvalid')));
          return;
        }
        cb();
      },
      trigger: 'change',
    },
  ],
};

const customers = ref<Array<{ id: number; customer_name?: string; name?: string }>>([]);

/** 金额计算 */
const subtotal = computed(() =>
  form.items.reduce(
    (sum: number, i: CreateQuotationItemDto) => sum + (i.quantity || 0) * (i.unit_price || 0),
    0
  )
);
const taxAmount = computed(() => (form.tax_inclusive ? 0 : (subtotal.value * form.tax_rate) / 100));
const totalAmount = computed(() => subtotal.value + taxAmount.value);

/** 加载客户下拉 */
async function loadCustomers() {
  try {
    const res = await getCustomerList({ page: 1, page_size: 1000 });
    // v11 批次 163 P2-1 修复：res.data as any 改为运行时安全访问
    const data = (res.data || {}) as { list?: unknown[]; items?: unknown[] };
    const list = data.list || data.items || [];
    customers.value = list as { id: number; name: string }[];
  } catch (error) {
    logger.error(t('quotations.create.customerLoadFailed'), error);
    customers.value = [];
  }
}

/** 由报价详情构建表头字段（编辑态与复制态共用；不含明细/条款） */
function buildQuotationHeader(data: QuotationResponseDto) {
  return {
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
  };
}

/** 复制明细：按新建处理，剥离源行 id/序列/金额等响应字段，仅保留 CreateQuotationItemDto 字段 */
function mapQuotationItemsForCopy(items: QuotationItemResponseDto[]): CreateQuotationItemDto[] {
  return items.map(i => ({
    product_id: i.product_id,
    color_id: i.color_id,
    specification: i.specification,
    unit: i.unit,
    quantity: i.quantity,
    unit_price: i.unit_price,
    unit_price_with_tax: i.unit_price_with_tax,
    tier_pricing: i.tier_pricing,
    discount_rate: i.discount_rate,
    notes: i.notes,
  }));
}

/** 复制条款：按新建处理，剥离源条款 id，仅保留 CreateQuotationTermDto 字段 */
function mapQuotationTermsForCopy(terms: QuotationTermResponseDto[]): CreateQuotationTermDto[] {
  return terms.map(term => ({
    term_type: term.term_type,
    term_key: term.term_key,
    term_value: term.term_value,
    sequence: term.sequence,
  }));
}

/** 编辑模式：加载已有数据 */
async function loadExisting() {
  const id = Number(props.quotationId || route.params.id);
  if (!id) return;
  loading.value = true;
  try {
    const res = await getQuotation(id);
    const data = res.data;
    if (data) {
      Object.assign(form, buildQuotationHeader(data), {
        items: (data.items || []) as CreateQuotationItemDto[],
        terms: (data.terms || []) as CreateQuotationTermDto[],
      });
    }
  } catch (e: unknown) {
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('quotations.create.loadFailed')
    );
  } finally {
    loading.value = false;
  }
}

/**
 * 复制为新单：复用"按 id 载入详情"逻辑预填表头/明细/条款，但保持新建态。
 * - 不设置 editingId、isEdit 仍为 false，保存走 createQuotation(POST) 生成新单；
 * - 表单本身不含 quotation_no/status/id 字段，单号由新建流程重新生成、状态为新草稿；
 * - 明细/条款剥离源行 id，避免被当更新或污染源单。
 */
async function loadForCopy() {
  const id = copyFromId.value;
  if (!id) return;
  loading.value = true;
  try {
    const res = await getQuotation(id);
    const data = res.data;
    if (data) {
      Object.assign(form, buildQuotationHeader(data), {
        items: mapQuotationItemsForCopy(data.items || []),
        terms: mapQuotationTermsForCopy(data.terms || []),
      });
      // 复制的新单归属当前操作人：清零源单销售员，交由 ensureSalesUserId 按当前用户填充（与空白新建一致）
      form.sales_user_id = 0;
    }
  } catch (e: unknown) {
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('quotations.create.loadFailed')
    );
  } finally {
    loading.value = false;
  }
}

/**
 * 确保有 sales_user_id（默认当前用户）。
 * 兼容 localStorage 缓存路径：路由守卫在 store 已有 permissions 缓存时跳过 fetchUserInfo，
 * 导致 userInfo 对象可能仅有 permissions 而无 id/username；此处主动补刷以获取完整 id。
 */
async function ensureSalesUserId() {
  if (form.sales_user_id) return;
  if (!userStore.userInfo?.id) {
    // 缓存态不完整：主动获取用户信息（cookie 有效时 API 正常返回）
    try {
      await userStore.fetchUserInfo();
    } catch {
      // 网络/API 失败不阻塞保存流程，后端会根据 session 取当前用户
    }
  }
  if (userStore.userInfo?.id) {
    form.sales_user_id = userStore.userInfo.id;
  }
}

/** 保存草稿 */
async function handleSaveDraft() {
  if (!formRef.value) return;
  try {
    await formRef.value.validate();
  } catch {
    ElMessage.error(t('quotations.create.validateForm'));
    return;
  }
  await ensureSalesUserId();
  submitting.value = true;
  try {
    if (isEdit.value) {
      const id = Number(props.quotationId || route.params.id);
      const res = await updateQuotation(id, form);
      ElMessage.success(t('quotations.create.draftUpdated'));
      // v11 批次 163 P2-1 修复：res.data as any 改为 QuotationResponseDto
      router.push(`/quotations/${res.data?.id ?? id}`);
    } else {
      const res = await createQuotation(form);
      ElMessage.success(t('quotations.create.draftSaved'));
      router.push(`/quotations/${res.data?.id ?? ''}`);
    }
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('quotations.create.saveFailed')
    );
  } finally {
    submitting.value = false;
  }
}

/** 提交审批 */
async function handleSubmit() {
  if (!formRef.value) return;
  try {
    await formRef.value.validate();
  } catch {
    ElMessage.error(t('quotations.create.validateForm'));
    return;
  }
  await ensureSalesUserId();
  submitting.value = true;
  try {
    let quotationId: number;
    if (isEdit.value) {
      const id = Number(props.quotationId || route.params.id);
      const res = await updateQuotation(id, form);
      // v11 批次 163 P2-1 修复：res.data as any 改为 QuotationResponseDto
      quotationId = res.data?.id ?? id;
    } else {
      const res = await createQuotation(form);
      quotationId = res.data?.id ?? 0;
    }
    await submitQuotation(quotationId);
    ElMessage.success(t('quotations.create.submitSuccess'));
    router.push(`/quotations/${quotationId}`);
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('quotations.create.submitFailed')
    );
  } finally {
    submitting.value = false;
  }
}

function formatAmount(value: number): string {
  return Number(value).toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
}

/** 贸易条款变化（处理可选字段） */
function onTermsChange(value: CreateQuotationTermDto[]) {
  form.terms = value;
}

onMounted(async () => {
  await loadCustomers();
  if (isEdit.value) {
    await loadExisting();
  } else if (copyFromId.value) {
    await loadForCopy();
  }
});
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

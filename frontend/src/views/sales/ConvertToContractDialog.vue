<!--
  sales/ConvertToContractDialog.vue - 销售订单转销售合同 预填对话框
  由销售订单列表行操作"转合同"或详情页"转合同"打开。
  打开即按 orderId 拉取 preview 带出客户、结算口径、交期与明细行，用户可改后提交；
  提交调用转换端点，服务端回读来源订单行折扣率/税率并经 so/amounts 重算金额，
  前端只送可编辑白名单字段（数量、单价、色号、允差、备注、交期、表头口径字段）。
  数值录入项一律用 el-input + 提交前校验（全项目禁 el-input-number 红线），
  范围与精度靠校验器，不靠步进按钮。
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('sales.convert.title')"
    width="960px"
    :close-on-click-modal="false"
    :aria-label="t('sales.convert.title')"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <div v-loading="loading">
      <!-- 既有非终态转换合同提示：命中即不再提交转换，给跳转链接 -->
      <el-alert
        v-if="existingContract"
        type="warning"
        :closable="false"
        show-icon
        class="existing-alert"
      >
        <template #title>
          <span>{{ t('sales.convert.existingTip', { no: existingContract.contract_no }) }}</span>
          <el-button link type="primary" @click="goExisting(existingContract.id)">
            {{ t('sales.convert.existingGo') }}
          </el-button>
        </template>
      </el-alert>

      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        label-width="100px"
        :aria-label="t('sales.convert.ariaForm')"
      >
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('sales.convert.orderNo')">
              <el-input :model-value="preview?.order_no" readonly />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('sales.convert.customer')">
              <!-- 签约主体与来源订单一致，只读回显，不可改派 -->
              <el-input :model-value="preview?.customer_name" readonly />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('sales.convert.contractName')" prop="contract_name">
              <el-input v-model="form.contract_name" maxlength="200" show-word-limit />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('sales.convert.contractType')">
              <el-select v-model="form.contract_type" clearable>
                <el-option :label="t('sales.convert.optionSales')" value="sale" />
                <el-option :label="t('sales.convert.optionFramework')" value="framework" />
                <el-option :label="t('sales.convert.optionSupplement')" value="supplement" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('sales.convert.paymentTerms')">
              <el-input v-model="form.payment_terms" maxlength="255" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('sales.convert.paymentMethod')">
              <el-select v-model="form.payment_method" clearable>
                <el-option :label="t('sales.convert.optionBankTransfer')" value="bank_transfer" />
                <el-option :label="t('sales.convert.optionCheck')" value="check" />
                <el-option :label="t('sales.convert.optionCash')" value="cash" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="8">
            <el-form-item :label="t('sales.convert.signedDate')">
              <el-date-picker
                v-model="form.signed_date"
                type="date"
                value-format="YYYY-MM-DD"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item :label="t('sales.convert.effectiveDate')">
              <el-date-picker
                v-model="form.effective_date"
                type="date"
                value-format="YYYY-MM-DD"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item :label="t('sales.convert.expiryDate')">
              <el-date-picker
                v-model="form.expiry_date"
                type="date"
                value-format="YYYY-MM-DD"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('sales.convert.deliveryDate')">
              <el-date-picker
                v-model="form.delivery_date"
                type="date"
                value-format="YYYY-MM-DD"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('sales.convert.deliveryLocation')">
              <el-input v-model="form.delivery_location" maxlength="200" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item :label="t('sales.convert.remark')">
          <el-input
            v-model="form.remark"
            type="textarea"
            :rows="2"
            maxlength="500"
            show-word-limit
          />
        </el-form-item>

        <el-divider content-position="left">{{ t('sales.convert.itemsTitle') }}</el-divider>
        <div class="amount-hint">{{ t('sales.convert.amountHint') }}</div>
        <el-table :data="form.items" border size="small" style="width: 100%">
          <el-table-column :label="t('sales.convert.colProduct')" min-width="180">
            <template #default="{ row }">
              <div class="cell-strong">{{ row.product_code }} {{ row.product_name }}</div>
              <div class="cell-sub">
                {{ specLine(row) }}
              </div>
            </template>
          </el-table-column>
          <el-table-column :label="t('sales.convert.colWidthGram')" width="120">
            <template #default="{ row }">{{ widthGramLine(row) }}</template>
          </el-table-column>
          <el-table-column :label="t('sales.convert.colUnit')" width="70">
            <template #default="{ row }">{{ row.unit }}</template>
          </el-table-column>
          <el-table-column :label="t('sales.convert.colQuantity')" width="120">
            <template #default="{ row }">
              <el-input v-model="row.quantity" size="small" />
            </template>
          </el-table-column>
          <el-table-column :label="t('sales.convert.colUnitPrice')" width="120">
            <template #default="{ row }">
              <el-input v-model="row.unit_price" size="small" />
            </template>
          </el-table-column>
          <el-table-column :label="t('sales.convert.colColor')" width="120">
            <template #default="{ row }">
              <el-input v-model="row.color_no" size="small" maxlength="50" />
            </template>
          </el-table-column>
          <el-table-column :label="t('sales.convert.colTolerance')" width="110">
            <template #default="{ row }">
              <el-input v-model="row.quantity_tolerance_pct" size="small" />
            </template>
          </el-table-column>
          <el-table-column :label="t('sales.convert.colSourceAmount')" width="120">
            <template #default="{ row }">{{ row.source_amount }}</template>
          </el-table-column>
        </el-table>

        <el-form-item :label="t('sales.convert.lineRemarks')" class="line-remarks">
          <el-input
            v-for="(row, idx) in form.items"
            :key="row.order_item_id"
            v-model="row.remarks"
            type="textarea"
            :rows="1"
            class="line-remark-input"
            :placeholder="t('sales.convert.lineRemarkPlaceholder', { index: idx + 1 })"
          />
        </el-form-item>
      </el-form>
    </div>

    <template #footer>
      <el-button @click="emit('update:visible', false)">{{
        t('sales.convert.btnCancel')
      }}</el-button>
      <el-button
        type="primary"
        :loading="submitting"
        :disabled="!preview || !!existingContract || form.items.length === 0"
        @click="onSubmit"
      >
        {{ t('sales.convert.btnSubmit') }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRouter } from 'vue-router';
import { ElMessage } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import {
  previewConvertFromOrder,
  convertOrderToContract,
  type ContractPreview,
  type ConvertFromOrderPayload,
} from '@/api/sales-contract';
import { logAuxLoadFailure } from '@/utils/logger';

const props = defineProps<{
  visible: boolean;
  orderId: number | null;
}>();

const emit = defineEmits<{
  'update:visible': [v: boolean];
  success: [contractId: number];
}>();

const { t } = useI18n({ useScope: 'global' });
const router = useRouter();

const loading = ref(false);
const submitting = ref(false);
const preview = ref<ContractPreview | null>(null);
const formRef = ref<FormInstance>();

interface ItemForm {
  order_item_id: number;
  product_code: string;
  product_name: string;
  product_spec: string | null;
  fabric_composition: string | null;
  width: string | null;
  gram_weight: string | null;
  unit: string;
  // 数值录入项用字符串承载、提交前解析校验（禁 el-input-number）
  quantity: string;
  unit_price: string;
  color_no: string;
  quantity_tolerance_pct: string;
  source_amount: string;
  remarks: string | null;
}

const form = reactive({
  contract_name: '',
  contract_type: null as string | null,
  payment_terms: null as string | null,
  payment_method: null as string | null,
  signed_date: null as string | null,
  effective_date: null as string | null,
  expiry_date: null as string | null,
  delivery_date: null as string | null,
  delivery_location: null as string | null,
  remark: null as string | null,
  items: [] as ItemForm[],
});

const rules: FormRules = {
  contract_name: [
    { required: true, message: () => t('sales.convert.needContractName'), trigger: 'blur' },
    { max: 200, message: () => t('sales.convert.contractNameTooLong'), trigger: 'blur' },
  ],
};

const existingContract = computed(() => preview.value?.existing_contract ?? null);

function specLine(row: ItemForm): string {
  const parts: string[] = [];
  if (row.product_spec) parts.push(row.product_spec);
  if (row.fabric_composition) parts.push(row.fabric_composition);
  return parts.join(' · ');
}

function widthGramLine(row: ItemForm): string {
  const parts: string[] = [];
  if (row.width) parts.push(`${row.width}cm`);
  if (row.gram_weight) parts.push(`${row.gram_weight}g/m²`);
  return parts.join(' / ') || '-';
}

/** 金额/数值列后端序列化为 string，转成数字提交；空串按 null（允差），数量单价必填 */
function parseNumber(raw: string): number | null {
  const trimmed = raw.trim();
  if (trimmed === '') return null;
  const n = Number(trimmed);
  return Number.isFinite(n) ? n : null;
}

async function loadPreview(orderId: number) {
  loading.value = true;
  preview.value = null;
  try {
    const res = await previewConvertFromOrder(orderId);
    const data = res.data;
    preview.value = data;
    form.contract_name = data.default_contract_name;
    form.contract_type = null;
    form.payment_terms = null;
    form.payment_method = null;
    form.signed_date = null;
    form.effective_date = null;
    form.expiry_date = null;
    form.delivery_date = data.default_delivery_date;
    form.delivery_location = null;
    form.remark = data.default_remark;
    form.items = data.items.map(item => ({
      order_item_id: item.order_item_id,
      product_code: item.product_code,
      product_name: item.product_name,
      product_spec: item.product_spec,
      fabric_composition: item.fabric_composition,
      width: item.width,
      gram_weight: item.gram_weight,
      unit: item.unit,
      quantity: item.quantity,
      unit_price: item.unit_price,
      color_no: item.color_no,
      quantity_tolerance_pct: item.quantity_tolerance_pct ?? '',
      source_amount: item.subtotal,
      remarks: item.remarks,
    }));
  } catch (e: unknown) {
    logAuxLoadFailure('销售订单转合同预填加载失败', e);
    msgError(e);
  } finally {
    loading.value = false;
  }
}

watch(
  () => [props.visible, props.orderId] as const,
  ([vis, id]) => {
    if (vis && id != null) {
      void loadPreview(id);
    }
  },
  { immediate: true }
);

function msgError(e: unknown) {
  const message =
    e && typeof e === 'object' && 'message' in e ? String((e as { message: unknown }).message) : '';
  ElMessage.error(message || t('sales.convert.loadFailed'));
}

/** 提交前逐行校验数值与色号长度；返回是否通过 */
function validateItems(): boolean {
  for (let i = 0; i < form.items.length; i++) {
    const row = form.items[i];
    const qty = parseNumber(row.quantity);
    if (qty === null || qty <= 0) {
      ElMessage.warning(t('sales.convert.needPositiveQty', { line: i + 1 }));
      return false;
    }
    const price = parseNumber(row.unit_price);
    if (price === null || price < 0) {
      ElMessage.warning(t('sales.convert.needNonNegPrice', { line: i + 1 }));
      return false;
    }
    if (row.color_no && row.color_no.length > 50) {
      ElMessage.warning(t('sales.convert.colorTooLong', { line: i + 1 }));
      return false;
    }
    const tolRaw = row.quantity_tolerance_pct.trim();
    if (tolRaw !== '') {
      const tol = Number(tolRaw);
      if (!Number.isFinite(tol) || tol < 0 || tol > 100) {
        ElMessage.warning(t('sales.convert.toleranceOutOfRange', { line: i + 1 }));
        return false;
      }
    }
  }
  return true;
}

function buildPayload(): ConvertFromOrderPayload {
  return {
    contract_name: form.contract_name.trim(),
    contract_type: form.contract_type,
    payment_terms: blankToNull(form.payment_terms),
    payment_method: form.payment_method,
    signed_date: form.signed_date,
    effective_date: form.effective_date,
    expiry_date: form.expiry_date,
    delivery_date: form.delivery_date,
    delivery_location: blankToNull(form.delivery_location),
    remark: blankToNull(form.remark),
    items: form.items.map(row => {
      const tol = parseNumber(row.quantity_tolerance_pct);
      return {
        order_item_id: row.order_item_id,
        quantity: parseNumber(row.quantity) ?? 0,
        unit_price: parseNumber(row.unit_price) ?? 0,
        delivery_date: form.delivery_date,
        color_no: row.color_no.trim() === '' ? null : row.color_no.trim(),
        quantity_tolerance_pct: tol,
        remarks: row.remarks === null || row.remarks === '' ? null : row.remarks,
      };
    }),
  };
}

function blankToNull(v: string | null): string | null {
  if (v === null) return null;
  return v.trim() === '' ? null : v;
}

async function onSubmit() {
  if (!preview.value || props.orderId == null) return;
  if (existingContract.value) return;
  const ok = await formRef.value?.validate().catch(() => false);
  if (!ok) return;
  if (!validateItems()) return;

  submitting.value = true;
  try {
    const res = await convertOrderToContract(props.orderId, buildPayload());
    ElMessage.success(t('sales.convert.success'));
    emit('update:visible', false);
    emit('success', res.data.id);
  } catch (e: unknown) {
    msgError(e);
  } finally {
    submitting.value = false;
  }
}

function goExisting(contractId: number) {
  emit('update:visible', false);
  router.push('/sales-contract');
  void contractId;
}
</script>

<style scoped>
.existing-alert {
  margin-bottom: 12px;
}
.amount-hint {
  color: #909399;
  font-size: 12px;
  margin-bottom: 8px;
}
.cell-strong {
  font-weight: 600;
}
.cell-sub {
  color: #909399;
  font-size: 12px;
}
.line-remarks {
  margin-top: 12px;
}
.line-remark-input {
  margin-bottom: 6px;
}
</style>

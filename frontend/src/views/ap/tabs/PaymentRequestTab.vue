<!--
  PaymentRequestTab.vue - 应付付款申请管理
  覆盖付款申请全生命周期：创建（含应付单明细录入）/编辑/删除/提交/审批/驳回 + 详情查看
  数据源：/ap/payment-requests（api/ap.ts 封装）；
  创建对话框明细候选：/ap/verifications/unverified/invoices（已审批未付清应付单）
-->
<template>
  <div class="payment-request-tab">
    <div class="tab-toolbar">
      <el-button type="primary" :icon="Plus" @click="openCreateDialog">
        {{ t('apModule.paymentRequest.create') }}
      </el-button>
      <el-button @click="fetchRequests">{{ t('common.refresh') }}</el-button>
    </div>

    <el-table v-loading="loading" :data="requests" border stripe>
      <el-table-column prop="id" label="ID" width="70" />
      <el-table-column
        prop="request_no"
        :label="t('apModule.paymentRequest.requestNo')"
        min-width="150"
      />
      <el-table-column prop="supplier_id" :label="t('apModule.payment.supplier')" width="90" />
      <el-table-column
        prop="request_date"
        :label="t('apModule.paymentRequest.requestDate')"
        width="110"
      />
      <el-table-column
        prop="payment_type"
        :label="t('apModule.paymentRequest.paymentType')"
        width="100"
      />
      <el-table-column
        prop="payment_method"
        :label="t('apModule.payment.paymentMethod')"
        width="110"
      />
      <el-table-column
        prop="request_amount"
        :label="t('apModule.paymentRequest.requestAmount')"
        width="130"
        align="right"
      >
        <template #default="{ row }">{{ formatMoney(row.request_amount) }}</template>
      </el-table-column>
      <el-table-column
        prop="currency"
        :label="t('apModule.paymentRequest.currencyLabel')"
        width="70"
      />
      <el-table-column prop="approval_status" :label="t('common.status')" width="100">
        <template #default="{ row }">
          <el-tag :type="statusTagType(row.approval_status)" size="small">{{
            approvalStatusLabel(row.approval_status)
          }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('common.action')" width="280" fixed="right">
        <template #default="{ row }">
          <el-button
            v-if="row.approval_status === 'DRAFT' || row.approval_status === 'REJECTED'"
            size="small"
            type="primary"
            link
            @click="openEditDialog(row)"
          >
            {{ t('common.edit') }}
          </el-button>
          <el-button
            v-if="row.approval_status === 'DRAFT' || row.approval_status === 'REJECTED'"
            size="small"
            type="warning"
            link
            @click="submitRequest(row)"
          >
            {{ t('apModule.paymentRequest.submit') }}
          </el-button>
          <el-button
            v-if="row.approval_status === 'APPROVING'"
            size="small"
            type="success"
            link
            @click="approveRequest(row)"
          >
            {{ t('apModule.paymentRequest.approve') }}
          </el-button>
          <el-button
            v-if="row.approval_status === 'APPROVING'"
            size="small"
            type="danger"
            link
            @click="rejectRequest(row)"
          >
            {{ t('apModule.paymentRequest.reject') }}
          </el-button>
          <el-button
            v-if="row.approval_status === 'DRAFT' || row.approval_status === 'REJECTED'"
            size="small"
            type="danger"
            link
            @click="removeRequest(row)"
          >
            {{ t('common.delete') }}
          </el-button>
          <el-button size="small" link @click="showDetail(row)">
            {{ t('common.detail') }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- 新建/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="editId ? t('apModule.paymentRequest.edit') : t('apModule.paymentRequest.create')"
      :width="editId ? '560px' : '820px'"
    >
      <el-form ref="formRef" :model="form" :rules="rules" label-width="110px">
        <el-form-item :label="t('apModule.payment.supplier')" prop="supplier_id">
          <el-select
            v-model="form.supplier_id"
            filterable
            :placeholder="t('apModule.paymentRequest.supplierPlaceholder')"
            style="width: 220px"
          >
            <el-option v-for="s in suppliers" :key="s.id" :label="s.supplier_name" :value="s.id" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('apModule.paymentRequest.requestDate')" prop="request_date">
          <el-date-picker v-model="form.request_date" type="date" value-format="YYYY-MM-DD" />
        </el-form-item>
        <el-form-item :label="t('apModule.paymentRequest.paymentType')" prop="payment_type">
          <el-select v-model="form.payment_type">
            <el-option label="采购付款" value="purchase" />
            <el-option label="费用付款" value="expense" />
            <el-option label="预付款" value="prepayment" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('apModule.payment.paymentMethod')" prop="payment_method">
          <el-select v-model="form.payment_method">
            <el-option label="银行转账" value="bank_transfer" />
            <el-option label="现金" value="cash" />
            <el-option label="支票" value="check" />
            <el-option label="承兑" value="bill" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('apModule.paymentRequest.requestAmount')" prop="request_amount">
          <el-input-number v-model="form.request_amount" :min="0.01" :precision="2" />
        </el-form-item>
        <el-form-item :label="t('apModule.paymentRequest.currencyLabel')" prop="currency">
          <el-select v-model="form.currency" style="width: 120px">
            <el-option label="CNY" value="CNY" />
            <el-option label="USD" value="USD" />
            <el-option label="EUR" value="EUR" />
          </el-select>
        </el-form-item>
        <!--
          汇率条件必填（与后端 resolve_currency_and_rate 同口径）：
          仅非本位币显示并必填；本位币不显示、不发送该键，汇率由服务端权威短路为 1。
          精度 6 位对齐后端 Decimal(18,6)；min>0 守形状规则，0.01 历史缺陷值等
          精确裁决仍以后端 validator 为准（错误经响应拦截照常外显）。
          编辑态不显示：更新契约（UpdateApPaymentRequest）不含币种/汇率，
          编辑态维持现状不动（币种创建后不可改）。
        -->
        <el-form-item
          v-if="!editId && form.currency !== BASE_CURRENCY"
          :label="t('apModule.paymentRequest.exchangeRateLabel')"
          prop="exchange_rate"
        >
          <div>
            <el-input-number
              v-model="form.exchange_rate"
              :min="0.000001"
              :precision="6"
              controls-position="right"
              style="width: 200px"
            />
            <div class="rate-hint">{{ t('apModule.paymentRequest.exchangeRateHint') }}</div>
          </div>
        </el-form-item>
        <el-form-item :label="t('apModule.paymentRequest.bankName')">
          <el-input v-model="form.bank_name" />
        </el-form-item>
        <el-form-item :label="t('apModule.paymentRequest.notes')">
          <el-input v-model="form.notes" type="textarea" :rows="2" />
        </el-form-item>

        <!--
          明细录入（决策定案 #10）：后端 submit 门控要求「至少一条真实已审批应付单明细」
          （ap_payment_request_service.rs:330-339），候选=已审批且未付清（AUDITED/PARTIAL_PAID
          且 unpaid_amount>0；validate_invoice_items_txn :100-142 拒绝 DRAFT/CANCELLED 与超未付额，
          本地预校验仅提前暴露问题，最终以服务端校验为准）。
          明细仅创建契约（CreateApPaymentRequest.items :655-656）可送：更新契约
          UpdateApPaymentRequest（:699-730）无 items，且后端无付款申请明细行级端点
          （routes/finance.rs:708-746），编辑态给出真实口径提示而非假编辑器。
        -->
        <el-form-item v-if="!editId" :label="t('apModule.paymentRequest.itemsTitle')">
          <div style="width: 100%">
            <el-table v-loading="invoicesLoading" :data="itemRows" size="small" border>
              <el-table-column :label="t('apModule.paymentRequest.colInvoice')" min-width="240">
                <template #default="{ row }">
                  <el-select
                    v-model="row.invoice_id"
                    filterable
                    :disabled="!form.supplier_id"
                    :placeholder="
                      form.supplier_id
                        ? t('apModule.paymentRequest.invoicePlaceholder')
                        : t('apModule.paymentRequest.invoiceNeedSupplier')
                    "
                  >
                    <el-option
                      v-for="inv in payableInvoices"
                      :key="inv.id"
                      :label="
                        t('apModule.paymentRequest.invoiceOption', {
                          no: inv.invoice_no,
                          amount: formatMoney(inv.unpaid_amount),
                        })
                      "
                      :value="inv.id"
                    />
                  </el-select>
                </template>
              </el-table-column>
              <el-table-column
                :label="t('apModule.paymentRequest.colUnpaid')"
                width="130"
                align="right"
              >
                <template #default="{ row }">
                  {{ row.invoice_id ? formatMoney(unpaidOf(row.invoice_id)) : '-' }}
                </template>
              </el-table-column>
              <el-table-column :label="t('apModule.paymentRequest.colApplyAmount')" width="170">
                <template #default="{ row }">
                  <el-input-number
                    v-model="row.apply_amount"
                    :min="0.01"
                    :precision="2"
                    controls-position="right"
                    style="width: 100%"
                  />
                </template>
              </el-table-column>
              <el-table-column :label="t('apModule.paymentRequest.colItemNotes')" min-width="140">
                <template #default="{ row }">
                  <el-input v-model="row.notes" size="small" />
                </template>
              </el-table-column>
              <el-table-column :label="t('common.action')" width="80" align="center">
                <template #default="{ $index }">
                  <el-button size="small" type="danger" link @click="itemRows.splice($index, 1)">
                    {{ t('common.delete') }}
                  </el-button>
                </template>
              </el-table-column>
            </el-table>
            <el-button size="small" class="add-item-row-btn" @click="addItemRow">
              {{ t('apModule.paymentRequest.addItemRow') }}
            </el-button>
            <div v-if="itemRows.length === 0" class="items-tip">
              {{ t('apModule.paymentRequest.itemsEmptyTip') }}
            </div>
          </div>
        </el-form-item>
        <el-alert
          v-else
          class="items-edit-tip"
          :title="t('apModule.paymentRequest.itemsEditTip')"
          type="info"
          :closable="false"
          show-icon
        />
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="submitting" @click="handleSubmit">
          {{ t('common.save') }}
        </el-button>
      </template>
    </el-dialog>

    <!-- 详情对话框 -->
    <el-dialog
      v-model="detailVisible"
      :title="t('apModule.paymentRequest.detailTitle')"
      width="620px"
    >
      <el-descriptions v-if="detailRow" :column="2" border>
        <el-descriptions-item label="ID">{{ detailRow.id }}</el-descriptions-item>
        <el-descriptions-item :label="t('apModule.paymentRequest.requestNo')">
          {{ detailRow.request_no }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('apModule.payment.supplier')">
          {{ detailRow.supplier_id }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('common.status')">{{
          approvalStatusLabel(detailRow.approval_status)
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('apModule.paymentRequest.requestAmount')">
          {{ formatMoney(detailRow.request_amount) }} {{ detailRow.currency }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('apModule.paymentRequest.requestDate')">
          {{ detailRow.request_date }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('apModule.paymentRequest.bankName')">
          {{ detailRow.bank_name || '-' }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('apModule.paymentRequest.notes')">
          {{ detailRow.notes || '-' }}
        </el-descriptions-item>
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { isDialogDismissal } from '@/utils/monitor';
import { ref, reactive, watch, onMounted } from 'vue';
import { logger } from '@/utils/logger';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import type { FormInstance, FormRules } from 'element-plus';
import { getSupplierList, type Supplier } from '@/api/supplier';
import {
  getAPPaymentRequestList,
  getAPPaymentRequest,
  createAPPaymentRequest,
  updateAPPaymentRequest,
  deleteAPPaymentRequest,
  submitAPPaymentRequest,
  approveAPPaymentRequest,
  rejectAPPaymentRequest,
  getUnverifiedAPInvoices,
  type APPaymentRequest,
  type APInvoice,
  type CreateApPaymentRequestInput,
  type UpdateApPaymentRequestInput,
} from '@/api/ap';

const { t } = useI18n({ useScope: 'global' });

const requests = ref<APPaymentRequest[]>([]);
const loading = ref(false);
const dialogVisible = ref(false);
const detailVisible = ref(false);
const editId = ref<number | null>(null);
const detailRow = ref<APPaymentRequest | null>(null);
const formRef = ref<FormInstance>();
const submitting = ref(false);

// 供应商下拉（与核销 Tab 同款 getSupplierList，仅加载一次供对话框复用）
const suppliers = ref<Supplier[]>([]);

// 明细录入（决策定案 #10）：候选应付单 = 已审批且未付清
interface PaymentRequestItemRow {
  invoice_id: number | undefined;
  /** 未采集=undefined：提交前显式转十进制字符串，禁止 ?? 0 把未采集伪装成 0 */
  apply_amount: number | undefined;
  notes: string;
}
const itemRows = ref<PaymentRequestItemRow[]>([]);
const payableInvoices = ref<APInvoice[]>([]);
const invoicesLoading = ref(false);

const formatMoney = (amount: number | string | undefined) => {
  const n = Number(amount ?? 0);
  return n.toLocaleString('zh-CN', { minimumFractionDigits: 2 });
};

// 词表来源 models/status/finance.rs:53-58 与 common（大写），此前按小写 draft/pending_approval 建映射
// 加上后端根本不会写入的 paid/cancelled 两个幻 token，导致标签恒落 info、文案恒显原文。
const statusTagType = (status: string) => {
  const map: Record<string, string> = {
    DRAFT: 'info',
    APPROVING: 'warning',
    APPROVED: 'success',
    REJECTED: 'danger',
  };
  return map[status] || 'info';
};

const approvalStatusLabel = (status: string) => {
  const keys: Record<string, string> = {
    DRAFT: 'apModule.paymentRequest.statusDraft',
    APPROVING: 'apModule.paymentRequest.statusApproving',
    APPROVED: 'apModule.paymentRequest.statusApproved',
    REJECTED: 'apModule.paymentRequest.statusRejected',
  };
  const key = keys[status];
  return key ? t(key) : status;
};

const fetchRequests = async () => {
  loading.value = true;
  try {
    const res = await getAPPaymentRequestList();
    // 后端 ap_payment_request_handler::list_requests 返回 PaginatedResponse（data.items）
    requests.value = res.data.items;
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  } finally {
    loading.value = false;
  }
};

/** 本位币：与后端 crate::constants::DEFAULT_CURRENCY / models/ap_payment_request.rs
 *  币种列 DEFAULT 'CNY' 同源取值；非本位币即外币，汇率条件必填（服务端权威短路本位币=1） */
const BASE_CURRENCY = 'CNY';

const form = reactive({
  supplier_id: undefined as number | undefined,
  request_date: new Date().toISOString().split('T')[0],
  payment_type: 'purchase',
  payment_method: 'bank_transfer',
  request_amount: 0,
  currency: BASE_CURRENCY,
  /** 外币汇率：未录入=undefined，提交前显式转十进制字符串，禁止 ?? 兜底伪装成 1 */
  exchange_rate: undefined as number | undefined,
  bank_name: '',
  notes: '',
});

const rules: FormRules = {
  supplier_id: [
    { required: true, message: t('apModule.payment.supplierRequired'), trigger: 'change' },
  ],
  request_date: [
    { required: true, message: t('apModule.payment.dateRequired'), trigger: 'change' },
  ],
  request_amount: [
    { required: true, message: t('apModule.payment.amountRequired'), trigger: 'blur' },
  ],
  exchange_rate: [
    {
      validator: (_rule: unknown, value: unknown, callback: (err?: Error) => void) => {
        if (form.currency === BASE_CURRENCY) {
          callback();
          return;
        }
        const num = Number(value);
        if (value === undefined || value === null || !Number.isFinite(num) || num <= 0) {
          callback(new Error(t('apModule.paymentRequest.exchangeRateRequired')));
          return;
        }
        callback();
      },
      trigger: ['blur', 'change'],
    },
  ],
};

const fetchSuppliers = async () => {
  try {
    const res = await getSupplierList({ page: 1, page_size: 1000 });
    suppliers.value = res.data.items;
    logger.info(`[付款申请] 供应商选项加载成功，共 ${suppliers.value.length} 条`);
  } catch (e) {
    logger.error(t('apModule.paymentRequest.loadSuppliersFailed'), e);
    ElMessage.error(t('apModule.paymentRequest.loadSuppliersFailed'));
  }
};

// 可选应付单：GET /ap/verifications/unverified/invoices 返回裸数组（ap_verification_handler.rs:208，
// 元素=ap_invoice Model 逐键 snake_case）。该端点排除口径是「非 CANCELLED 且 unpaid>0」，
// 还含 DRAFT；此处再按 submit/validate_invoice_items_txn 的准入词表收窄为
// AUDITED/PARTIAL_PAID（models/ap_invoice.rs:68 注释即写入方词表），DRAFT 单不可申请付款。
const loadPayableInvoices = async (supplierId: number) => {
  invoicesLoading.value = true;
  try {
    const res = await getUnverifiedAPInvoices(supplierId);
    payableInvoices.value = res.data.filter(
      inv =>
        (inv.invoice_status === 'AUDITED' || inv.invoice_status === 'PARTIAL_PAID') &&
        Number(inv.unpaid_amount) > 0
    );
    logger.info(
      `[付款申请] 供应商 ${supplierId} 可选应付单加载成功，共 ${payableInvoices.value.length} 条`
    );
  } catch (e) {
    payableInvoices.value = [];
    logger.error(t('apModule.paymentRequest.loadInvoicesFailed'), e);
    ElMessage.error(t('apModule.paymentRequest.loadInvoicesFailed'));
  } finally {
    invoicesLoading.value = false;
  }
};

// 换供应商：明细行整体作废（应付单归属该供应商），并按新供应商重载候选。
// 编辑态无明细编辑器（更新契约不含 items），不触发候选加载。
watch(
  () => form.supplier_id,
  (supplierId, prev) => {
    if (supplierId === prev || editId.value) return;
    itemRows.value = [];
    payableInvoices.value = [];
    if (supplierId) void loadPayableInvoices(supplierId);
  }
);

const addItemRow = () => {
  itemRows.value.push({ invoice_id: undefined, apply_amount: undefined, notes: '' });
};

// 切回本位币：清空汇率输入，防止隐藏的残值被误当作外币请求值发送
// （本位币下服务端恒按 1 短路，前端不保留、不伪造任何汇率值）
watch(
  () => form.currency,
  currency => {
    if (currency === BASE_CURRENCY) form.exchange_rate = undefined;
  }
);

const unpaidOf = (invoiceId: number) =>
  payableInvoices.value.find(inv => inv.id === invoiceId)?.unpaid_amount;

/**
 * 明细本地预校验（仅提前暴露问题提升体验；最终裁决以后端 validate_invoice_items_txn
 * ap_payment_request_service.rs:100-142 为准，后端错误经响应拦截照常外显）：
 * - 每行须选定应付单；申请金额 > 0（后端表头同口径 validate_positive_decimal_payment）；
 * - ≤ 该票未付余额（后端 :133 同口径，unpaid_amount 为 rust_decimal 出参字符串，显式 Number 比较）；
 * - 同一应付单不允许重复行（后端逐行校验，重复行会造成同票超额合计）。
 * 全部通过时返回可直接下发的 items（金额转两位小数十进制字符串）；无行返回 undefined（不送空数组伪装明细）。
 */
const buildItemsPayload = (): {
  ok: boolean;
  items?: NonNullable<CreateApPaymentRequestInput['items']>;
} => {
  if (itemRows.value.length === 0) return { ok: true };
  const seen = new Set<number>();
  const items: NonNullable<CreateApPaymentRequestInput['items']> = [];
  for (const row of itemRows.value) {
    if (row.invoice_id == null) {
      ElMessage.warning(t('apModule.paymentRequest.invoiceRequired'));
      return { ok: false };
    }
    if (row.apply_amount == null || !Number.isFinite(row.apply_amount) || row.apply_amount <= 0) {
      ElMessage.warning(t('apModule.paymentRequest.applyAmountRequired'));
      return { ok: false };
    }
    if (seen.has(row.invoice_id)) {
      const inv = payableInvoices.value.find(i => i.id === row.invoice_id);
      ElMessage.warning(
        t('apModule.paymentRequest.duplicateInvoice', { no: inv?.invoice_no ?? row.invoice_id })
      );
      return { ok: false };
    }
    seen.add(row.invoice_id);
    // 候选列表内才做余额预校验；不在候选内（加载后外部变化）交后端裁决
    const candidate = payableInvoices.value.find(i => i.id === row.invoice_id);
    if (candidate && row.apply_amount > Number(candidate.unpaid_amount)) {
      ElMessage.warning(
        t('apModule.paymentRequest.applyAmountExceed', {
          no: candidate.invoice_no,
          unpaid: formatMoney(candidate.unpaid_amount),
        })
      );
      return { ok: false };
    }
    items.push({
      invoice_id: row.invoice_id,
      // rust_decimal 请求侧用字符串，避免 number 二进制浮点误差落进金额
      apply_amount: row.apply_amount.toFixed(2),
      notes: row.notes.trim() || undefined,
    });
  }
  return { ok: true, items };
};

const resetForm = () => {
  form.supplier_id = undefined;
  form.request_date = new Date().toISOString().split('T')[0];
  form.payment_type = 'purchase';
  form.payment_method = 'bank_transfer';
  form.request_amount = 0;
  form.currency = BASE_CURRENCY;
  form.exchange_rate = undefined;
  form.bank_name = '';
  form.notes = '';
  itemRows.value = [];
  payableInvoices.value = [];
};

const openCreateDialog = () => {
  editId.value = null;
  resetForm();
  dialogVisible.value = true;
};

const openEditDialog = (row: APPaymentRequest) => {
  editId.value = row.id;
  form.supplier_id = row.supplier_id;
  form.request_date = row.request_date;
  form.payment_type = row.payment_type || '';
  form.payment_method = row.payment_method || '';
  form.request_amount = Number(row.request_amount ?? 0);
  form.currency = row.currency || BASE_CURRENCY;
  // 编辑契约不含币种/汇率：不采集、不回填汇率（更新 payload 从不发送该键）
  form.exchange_rate = undefined;
  form.bank_name = row.bank_name || '';
  form.notes = row.notes || '';
  dialogVisible.value = true;
};

const handleSubmit = async () => {
  const valid = await formRef.value?.validate();
  if (!valid) return;
  // 表头金额显式前置校验（el-input-number 清空会送 null/undefined，直接 .toFixed 即运行期崩；
  // 后端 validate_positive_decimal_payment :677-682 仍是最终裁决，本地只是把错误提前且不留崩溃面）
  const headerAmount = Number(form.request_amount);
  if (!Number.isFinite(headerAmount) || headerAmount <= 0) {
    ElMessage.warning(t('apModule.payment.amountRequired'));
    logger.warn(`[付款申请] 表头申请金额非法：${String(form.request_amount)}，拒绝提交`);
    return;
  }
  // 外币汇率显式前置校验（与 rules 同口径提前暴露、不留运行期崩面；
  // 后端 resolve_currency_and_rate 仍是最终裁决：外币缺汇率 400，本位币恒短路为 1）
  let foreignExchangeRate: string | undefined;
  if (!editId.value && form.currency !== BASE_CURRENCY) {
    const rate = Number(form.exchange_rate);
    if (form.exchange_rate === undefined || !Number.isFinite(rate) || rate <= 0) {
      ElMessage.warning(t('apModule.paymentRequest.exchangeRateRequired'));
      logger.warn(
        `[付款申请] 外币(${form.currency}) 创建未录入合法汇率：${String(form.exchange_rate)}，拒绝提交`
      );
      return;
    }
    // rust_decimal 请求侧用十进制字符串；6 位小数对齐后端汇率列 Decimal(18,6)
    foreignExchangeRate = rate.toFixed(6);
  }
  // 金额出参为 rust_decimal 字符串（见 formatMoney），提交统一转两位小数十进制字符串（表头
  // request_amount 与明细 apply_amount 同口径，validate :616/677 要求正数）
  let items: CreateApPaymentRequestInput['items'];
  if (!editId.value) {
    const built = buildItemsPayload();
    if (!built.ok) return;
    items = built.items;
    if (!items) {
      logger.warn(
        `[付款申请] 本次创建未录入明细（与后端 items 可选口径一致），提交审批前须删除重建补明细`
      );
    }
  }
  submitting.value = true;
  try {
    if (editId.value) {
      // 更新契约（UpdateApPaymentRequest）不含 supplier_id/items/currency：不在请求里假装可改
      const payload: UpdateApPaymentRequestInput = {
        request_date: form.request_date,
        payment_type: form.payment_type,
        payment_method: form.payment_method,
        request_amount: headerAmount.toFixed(2),
        bank_name: form.bank_name.trim() || undefined,
        notes: form.notes.trim() || undefined,
      };
      await updateAPPaymentRequest(editId.value, payload);
    } else {
      const payload: CreateApPaymentRequestInput = {
        supplier_id: form.supplier_id as number,
        request_date: form.request_date,
        payment_type: form.payment_type,
        payment_method: form.payment_method,
        request_amount: headerAmount.toFixed(2),
        currency: form.currency,
        // 条件发送：仅外币携带 exchange_rate；本位币不发该键（不伪造请求值），
        // 由服务端权威短路为 1（后端 create 入口 resolve_currency_and_rate）
        ...(foreignExchangeRate !== undefined ? { exchange_rate: foreignExchangeRate } : {}),
        bank_name: form.bank_name.trim() || undefined,
        notes: form.notes.trim() || undefined,
        items,
      };
      await createAPPaymentRequest(payload);
    }
    ElMessage.success(t('common.success'));
    dialogVisible.value = false;
    fetchRequests();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  } finally {
    submitting.value = false;
  }
};

const submitRequest = async (row: APPaymentRequest) => {
  try {
    await ElMessageBox.confirm(t('apModule.paymentRequest.submitConfirm'), t('common.confirm'), {
      type: 'info',
    });
    await submitAPPaymentRequest(row.id);
    ElMessage.success(t('common.success'));
    fetchRequests();
  } catch (e) {
    if (!isDialogDismissal(e)) {
      const err = e as { message?: string };
      ElMessage.error(err.message || t('common.failed'));
    }
  }
};

const approveRequest = async (row: APPaymentRequest) => {
  try {
    await ElMessageBox.confirm(t('apModule.paymentRequest.approveConfirm'), t('common.confirm'), {
      type: 'warning',
    });
    await approveAPPaymentRequest(row.id);
    ElMessage.success(t('common.success'));
    fetchRequests();
  } catch (e) {
    if (!isDialogDismissal(e)) {
      const err = e as { message?: string };
      ElMessage.error(err.message || t('common.failed'));
    }
  }
};

const rejectRequest = async (row: APPaymentRequest) => {
  try {
    const { value } = await ElMessageBox.prompt(
      t('apModule.paymentRequest.rejectReason'),
      t('apModule.paymentRequest.reject'),
      {
        type: 'warning',
        inputPattern: /\S+/,
        inputErrorMessage: t('apModule.paymentRequest.rejectReasonRequired'),
      }
    );
    await rejectAPPaymentRequest(row.id, value);
    ElMessage.success(t('common.success'));
    fetchRequests();
  } catch (e) {
    if (!isDialogDismissal(e)) {
      const err = e as { message?: string };
      ElMessage.error(err.message || t('common.failed'));
    }
  }
};

const removeRequest = async (row: APPaymentRequest) => {
  try {
    await ElMessageBox.confirm(t('apModule.paymentRequest.deleteConfirm'), t('common.delete'), {
      type: 'warning',
    });
    await deleteAPPaymentRequest(row.id);
    ElMessage.success(t('common.success'));
    fetchRequests();
  } catch (e) {
    if (!isDialogDismissal(e)) {
      const err = e as { message?: string };
      ElMessage.error(err.message || t('common.failed'));
    }
  }
};

const showDetail = async (row: APPaymentRequest) => {
  try {
    const res = await getAPPaymentRequest(row.id);
    detailRow.value = (res.data as APPaymentRequest) || row;
  } catch (error) {
    logger.error(t('apModule.paymentRequest.detailFailed'), error);
    detailRow.value = row;
  }
  detailVisible.value = true;
};

defineExpose({ refresh: fetchRequests });

onMounted(() => {
  fetchRequests();
  fetchSuppliers();
});
</script>

<style scoped>
.tab-toolbar {
  margin-bottom: 12px;
  display: flex;
  gap: 8px;
}
.add-item-row-btn {
  margin-top: 8px;
}
.items-tip {
  margin-top: 6px;
  font-size: 12px;
  color: #e6a23c;
}
.rate-hint {
  font-size: 12px;
  color: #909399;
  line-height: 1.5;
}
.items-edit-tip {
  margin-top: 4px;
}
</style>

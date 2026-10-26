<!--
  InvoiceTab.vue - 应付发票 Tab
  来源：原 ap/index.vue 中 应付发票 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="invoice-tab">
    <div class="page-header">
      <h2 class="page-title">{{ $t('apModule.invoice.title') }}</h2>
      <div class="header-actions">
        <el-button type="primary" @click="openInvoiceDialog()">
          <el-icon><Plus /></el-icon> {{ $t('apModule.invoice.create') }}
        </el-button>
        <el-button :loading="autoGenerating" @click="handleAutoGenerate">
          {{ $t('apModule.invoice.autoGenerate') }}
        </el-button>
        <el-button @click="showAgingAnalysis">{{ $t('apModule.invoice.agingAnalysis') }}</el-button>
        <!-- 打印入口只把本页已读到的发票行交给浏览器本地打印，不落后端打印端点，门控取读面键；
             导出走 GET /ap/invoices/export，权限键按该路径段派生为 ap-invoices:export -->
        <el-button v-permission="PERMISSIONS.AP_INVOICE_READ" @click="handlePrintInvoices">
          <el-icon><Printer /></el-icon> {{ $t('common.print') }}
        </el-button>
        <el-button v-permission="PERMISSIONS.AP_INVOICE_EXPORT" @click="handleExportInvoices">
          <el-icon><Download /></el-icon> {{ $t('common.export') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="invoiceQuery" :aria-label="$t('apModule.invoice.filterAria')">
        <el-form-item :label="$t('common.status')">
          <el-select
            v-model="invoiceQuery.invoice_status"
            :placeholder="$t('apModule.invoice.statusPlaceholder')"
            clearable
          >
            <el-option :label="$t('apModule.invoice.statusDraft')" value="DRAFT" />
            <el-option :label="$t('apModule.invoice.statusAudited')" value="AUDITED" />
            <el-option :label="$t('apModule.invoice.statusPartialPaid')" value="PARTIAL_PAID" />
            <el-option :label="$t('apModule.invoice.statusPaid')" value="PAID" />
            <el-option :label="$t('apModule.invoice.statusCancelled')" value="CANCELLED" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="fetchInvoices">{{ $t('common.search') }}</el-button>
          <el-button @click="resetInvoiceQuery">{{ $t('common.reset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover">
      <el-table
        v-loading="invoiceLoading"
        :data="invoices"
        stripe
        :aria-label="$t('apModule.invoice.listAria')"
      >
        <el-table-column prop="invoice_no" :label="$t('apModule.invoice.invoiceNo')" width="140" />
        <el-table-column :label="$t('apModule.invoice.supplier')" width="150">
          <template #default="{ row }">{{ supplierLabel(row.supplier_id) }}</template>
        </el-table-column>
        <el-table-column
          prop="invoice_date"
          :label="$t('apModule.invoice.invoiceDate')"
          width="120"
        />
        <el-table-column :label="$t('apModule.invoice.invoiceAmount')" width="120" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.amount) }}
          </template>
        </el-table-column>
        <el-table-column :label="$t('apModule.invoice.taxAmount')" width="100" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.tax_amount) }}
          </template>
        </el-table-column>
        <el-table-column :label="$t('apModule.invoice.paidAmount')" width="110" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.paid_amount) }}
          </template>
        </el-table-column>
        <el-table-column :label="$t('apModule.invoice.unpaidAmount')" width="110" align="right">
          <template #default="{ row }">
            <span :class="{ 'text-red': Number(row.unpaid_amount) > 0 }">
              {{ formatMoney(row.unpaid_amount) }}
            </span>
          </template>
        </el-table-column>
        <el-table-column
          prop="invoice_status"
          :label="$t('common.status')"
          width="90"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="getInvoiceStatusType(row.invoice_status)" size="small">
              {{ getInvoiceStatusLabel(row.invoice_status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="due_date" :label="$t('apModule.invoice.dueDate')" width="120" />
        <el-table-column :label="$t('common.operation')" width="180" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="viewInvoice(row as APInvoice)">{{
              $t('common.detail')
            }}</el-button>
            <el-button
              v-if="row.invoice_status === 'DRAFT'"
              type="success"
              link
              size="small"
              @click="approveInvoice(row as APInvoice)"
              >{{ $t('apModule.invoice.approve') }}</el-button
            >
            <el-button
              v-if="row.invoice_status === 'DRAFT'"
              type="danger"
              link
              size="small"
              @click="cancelInvoice(row as APInvoice)"
              >{{ $t('common.cancel') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog
      v-model="invoiceDialogVisible"
      :title="$t('apModule.invoice.createTitle')"
      width="600px"
      :aria-label="$t('apModule.invoice.createAria')"
    >
      <el-form
        ref="invoiceFormRef"
        :model="invoiceForm"
        :rules="invoiceRules"
        label-width="80px"
        :aria-label="$t('apModule.invoice.formAria')"
      >
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('apModule.invoice.supplier')" prop="supplier_id">
              <el-select
                v-model="invoiceForm.supplier_id"
                :placeholder="$t('apModule.invoice.supplierPlaceholder')"
                style="width: 100%"
              >
                <el-option
                  v-for="s in suppliers"
                  :key="s.id"
                  :label="s.supplier_name"
                  :value="s.id"
                />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <!-- 应付类型是后端 NOT NULL 无默认列，且后端不代填：必须由建单人选择声明，
                 选项取自 models/ap_invoice.rs 对该列的既有说明，后端本批不做值域校验 -->
            <el-form-item :label="$t('apModule.invoice.invoiceType')" prop="invoice_type">
              <el-select
                v-model="invoiceForm.invoice_type"
                :placeholder="$t('apModule.invoice.invoiceTypePlaceholder')"
                style="width: 100%"
              >
                <el-option
                  v-for="opt in invoiceTypeOptions"
                  :key="opt.value"
                  :label="$t(opt.labelKey)"
                  :value="opt.value"
                />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('apModule.invoice.invoiceDate')" prop="invoice_date">
              <el-date-picker
                v-model="invoiceForm.invoice_date"
                type="date"
                :placeholder="$t('apModule.invoice.datePlaceholder')"
                value-format="YYYY-MM-DD"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="$t('apModule.invoice.dueDate')" prop="due_date">
              <el-date-picker
                v-model="invoiceForm.due_date"
                type="date"
                :placeholder="$t('apModule.invoice.datePlaceholder')"
                value-format="YYYY-MM-DD"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('apModule.invoice.invoiceAmount')" prop="invoice_amount">
              <el-input
                v-model="invoiceAmountField.text"
                style="width: 100%"
                @blur="invoiceAmountField.normalize()"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <!-- 税额不预置 0：0 只能由建单人显式敲出，代表"本票无进项税"这条声明 -->
            <el-form-item :label="$t('apModule.invoice.taxAmount')" prop="tax_amount">
              <el-input
                v-model="taxAmountField.text"
                :placeholder="$t('apModule.invoice.taxAmountPlaceholder')"
                style="width: 100%"
                @blur="taxAmountField.normalize()"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item :label="$t('apModule.invoice.remark')">
          <el-input v-model="invoiceForm.remark" type="textarea" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="invoiceDialogVisible = false">{{ $t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="invoiceSubmitLoading" @click="submitInvoice">{{
          $t('common.confirm')
        }}</el-button>
      </template>
    </el-dialog>

    <!-- 账龄分析弹窗 -->
    <el-dialog v-model="agingVisible" :title="$t('apModule.invoice.agingAnalysis')" width="520px">
      <el-table :data="agingRows" border size="small">
        <el-table-column
          prop="aging_bucket"
          :label="$t('apModule.invoice.agingBucket')"
          min-width="140"
        />
        <el-table-column
          prop="invoice_count"
          :label="$t('apModule.invoice.agingInvoiceCount')"
          width="100"
          align="right"
        />
        <el-table-column
          prop="total_amount"
          :label="$t('apModule.invoice.agingAmount')"
          width="140"
          align="right"
        >
          <template #default="{ row }">{{
            Number(row.total_amount ?? 0).toLocaleString('zh-CN', { minimumFractionDigits: 2 })
          }}</template>
        </el-table-column>
      </el-table>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { isDialogDismissal } from '@/utils/monitor';
import { ref, reactive, toRef, onMounted } from 'vue';
import { useNumberField } from '@/composables/useNumberField';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus, Printer, Download } from '@element-plus/icons-vue';
import { promptCancelReason } from '@/composables/useActionPrompts';
import printJS from 'print-js';
import type { FormInstance, FormRules } from 'element-plus';
import {
  getAPInvoiceList,
  getAPInvoice,
  createAPInvoice,
  approveAPInvoice,
  cancelAPInvoice,
  getAPInvoiceStatusText,
  type APInvoice,
} from '@/api/ap-invoice';
import { exportFromBackend } from '@/utils/export';
import { logger, logAuxLoadFailure } from '@/utils/logger';
import { autoGenerateAPInvoices, getAPAgingAnalysis, type APAgingItem } from '@/api/ap';
import { getSupplierList, type Supplier } from '@/api/supplier';
import { PERMISSIONS } from '@/constants/permissions';

const { t } = useI18n({ useScope: 'global' });

// 应付发票自动生成（按到期的采购收货单批量开票）
const autoGenerating = ref(false);
const handleAutoGenerate = async () => {
  try {
    // 后端契约：按入库单 ID 生成应付单，需用户输入入库单 ID
    const { value } = await ElMessageBox.prompt(
      t('apModule.invoice.autoGenerateConfirm'),
      t('apModule.invoice.autoGenerate'),
      {
        type: 'info',
        inputValidator: v => {
          const n = Number(v);
          return Number.isInteger(n) && n > 0 ? true : '请输入有效的入库单 ID';
        },
      }
    );
    autoGenerating.value = true;
    await autoGenerateAPInvoices({ receipt_id: Number(value) });
    ElMessage.success(t('apModule.invoice.autoGenerateSuccess'));
    fetchInvoices();
  } catch (e) {
    if (isDialogDismissal(e)) return;
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  } finally {
    autoGenerating.value = false;
  }
};

// 账龄分析：调后端账龄接口，弹窗展示汇总
const agingVisible = ref(false);
const agingRows = ref<APAgingItem[]>([]);
const showAgingAnalysis = async () => {
  try {
    const res = await getAPAgingAnalysis();
    // 后端 get_aging_analysis 直接返回裸数组 Vec<AgingAnalysisItem>
    agingRows.value = res.data;
    agingVisible.value = true;
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  }
};

const invoices = ref<APInvoice[]>([]);
const invoiceLoading = ref(false);
const suppliers = ref<Supplier[]>([]);

const invoiceQuery = reactive({
  invoice_status: '',
});

const formatMoney = (amount: number | string | undefined) => {
  // 后端 rust_decimal（features=["serde"]，无 serde-float）将金额序列化为字符串，
  // 直接对字符串调用 toLocaleString 不会补千分位。统一 Number() 归一再格式化。
  const n = Number(amount);
  if (!Number.isFinite(n)) return '0.00';
  return n.toLocaleString('zh-CN', { minimumFractionDigits: 2 });
};

const getInvoiceStatusType = (status: string) => {
  const map: Record<string, string> = {
    DRAFT: 'warning',
    AUDITED: 'primary',
    PARTIAL_PAID: 'warning',
    PAID: 'success',
    CANCELLED: 'danger',
  };
  return map[status] || 'info';
};

/** 列表响应只带 supplier_id（无 JOIN），用本页已加载的供应商主数据映射名称 */
const supplierLabel = (id: number) =>
  suppliers.value.find(s => s.id === id)?.supplier_name ?? String(id);

const getInvoiceStatusLabel = (status: string) => {
  const keyMap: Record<string, string> = {
    DRAFT: 'apModule.invoice.statusDraft',
    AUDITED: 'apModule.invoice.statusAudited',
    PARTIAL_PAID: 'apModule.invoice.statusPartialPaid',
    PAID: 'apModule.invoice.statusPaid',
    CANCELLED: 'apModule.invoice.statusCancelled',
  };
  const key = keyMap[status];
  if (key) return t(key);
  return getAPInvoiceStatusText(status) || status;
};

const fetchInvoices = async () => {
  invoiceLoading.value = true;
  try {
    const res = await getAPInvoiceList(invoiceQuery);
    const d = res.data as
      { list?: APInvoice[]; items?: APInvoice[]; data?: APInvoice[] } | APInvoice[] | undefined;
    if (d && typeof d === 'object' && !Array.isArray(d)) {
      invoices.value = d.list || d.items || d.data || [];
    } else {
      invoices.value = (d as APInvoice[]) || [];
    }
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('apModule.invoice.fetchListFailed'));
  } finally {
    invoiceLoading.value = false;
  }
};

const resetInvoiceQuery = () => {
  invoiceQuery.invoice_status = '';
  fetchInvoices();
};

const invoiceDialogVisible = ref(false);
const invoiceFormRef = ref<FormInstance>();
const invoiceSubmitLoading = ref(false);
/**
 * 应付类型选项：值域取自后端实体对 invoice_type 列的既有说明
 * （backend/src/models/ap_invoice.rs「PURCHASE=采购应付，EXPENSE=费用应付，OTHER=其他」）。
 * 后端本批只把"缺键"改成点名拒绝、不做值域校验，因此这里的取值是建单人的声明项，
 * 不是第二套受控词表；词表要不要落到后端属另立挂账项。
 */
const invoiceTypeOptions = [
  { value: 'PURCHASE', labelKey: 'apModule.invoice.typePurchase' },
  { value: 'EXPENSE', labelKey: 'apModule.invoice.typeExpense' },
  { value: 'OTHER', labelKey: 'apModule.invoice.typeOther' },
];
const invoiceForm = reactive({
  supplier_id: undefined as number | undefined,
  invoice_type: undefined as string | undefined,
  invoice_date: '',
  due_date: '',
  invoice_amount: 0,
  /**
   * 税额：undefined=未填写（提交前由必填规则拦下），0 是建单人显式敲出的合法声明值
   * （本票无进项税）。初值不预置 0：预置等于替用户做了"无税"这条他没说过的声明。
   */
  tax_amount: undefined as number | undefined,
  remark: '',
});

// 数值手输桥 范围与精度由单源校验器裁决（全项目禁步进控件）
const invoiceAmountField = useNumberField(toRef(invoiceForm, 'invoice_amount'), {
  label: t('apModule.invoice.invoiceAmount'),
  required: true,
  min: 0,
  precision: 2,
  requiredMessage: t('apModule.invoice.invoiceAmountRequired'),
});
const taxAmountField = useNumberField(
  toRef(invoiceForm, 'tax_amount'),
  {
    label: t('apModule.invoice.taxAmount'),
    required: true,
    min: 0,
    precision: 2,
    requiredMessage: t('apModule.invoice.taxAmountRequired'),
  },
  'undefined'
);

const invoiceRules: FormRules = {
  supplier_id: [
    { required: true, message: t('apModule.invoice.supplierRequired'), trigger: 'change' },
  ],
  invoice_type: [
    { required: true, message: t('apModule.invoice.invoiceTypeRequired'), trigger: 'change' },
  ],
  invoice_date: [
    { required: true, message: t('apModule.invoice.invoiceDateRequired'), trigger: 'change' },
  ],
  due_date: [{ required: true, message: t('apModule.invoice.dueDateRequired'), trigger: 'change' }],
  invoice_amount: [invoiceAmountField.rule],
  tax_amount: [taxAmountField.rule],
};

const openInvoiceDialog = () => {
  Object.assign(invoiceForm, {
    supplier_id: undefined,
    invoice_type: undefined,
    invoice_date: '',
    due_date: '',
    invoice_amount: 0,
    tax_amount: undefined,
    remark: '',
  });
  invoiceDialogVisible.value = true;
};

const submitInvoice = async () => {
  // el-form 的 validate() 在校不通过时以字段对象 reject，提交入口是事件处理器，
  // reject 冒出去会被 Vue 记为组件事件处理器运行时错误，这里就地转为 false 短路，
  // 让校验失败只体现为字段级红字提示
  const valid = await invoiceFormRef.value?.validate().catch(() => false);
  if (!valid) return;
  invoiceSubmitLoading.value = true;
  try {
    // 契约对齐后端 CreateApInvoiceRequest（services/ap_invoice_ops/types.rs:24）：
    // 金额键为 amount（非表单本地键 invoice_amount）、备注键为 notes（非 remark）；
    // invoice_no 由后端 generate_invoice_no() 生成，不入请求体。
    // 应付类型/发票日期/到期日期/税额四项后端一律必填且缺键即点名拒绝，
    // 前端必填规则先拦在这里，不把缺键发给后端。
    await createAPInvoice({
      supplier_id: invoiceForm.supplier_id,
      invoice_type: invoiceForm.invoice_type,
      invoice_date: invoiceForm.invoice_date,
      due_date: invoiceForm.due_date,
      // 后端 CreateApInvoiceRequest.amount/tax_amount 为 Option<rust_decimal>（serde-floats
      // 未启用，JSON 浮点字面量反序列化即拒）：视图态 number 转两位小数十进制字符串下发
      amount: invoiceForm.invoice_amount.toFixed(2),
      // 税额只在有值时下发：对 undefined 调 toFixed 会送出字符串 "NaN"，
      // 而后端把缺键与显式 0 视为两回事，这里不做任何代填
      tax_amount: invoiceForm.tax_amount == null ? undefined : invoiceForm.tax_amount.toFixed(2),
      notes: invoiceForm.remark || undefined,
    });
    ElMessage.success(t('common.success'));
    invoiceDialogVisible.value = false;
    fetchInvoices();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  } finally {
    invoiceSubmitLoading.value = false;
  }
};

// 批次 157a P1-1 修复：接入 getAPInvoice API 展示应付发票详情
const viewInvoice = async (row: APInvoice) => {
  try {
    const res = await getAPInvoice(row.id);
    const d = res.data;
    if (!d) {
      ElMessage.warning(t('apModule.invoice.notFoundDetail'));
      return;
    }
    const lines = [
      t('apModule.invoice.detailNo', { value: d.invoice_no }),
      t('apModule.invoice.detailSupplier', { value: supplierLabel(d.supplier_id) }),
      t('apModule.invoice.detailDate', { value: d.invoice_date }),
      t('apModule.invoice.detailDueDate', { value: d.due_date || '-' }),
      t('apModule.invoice.detailAmount', { value: formatMoney(d.amount) }),
      t('apModule.invoice.detailTax', { value: formatMoney(d.tax_amount) }),
      t('apModule.invoice.detailPaid', { value: formatMoney(d.paid_amount) }),
      t('apModule.invoice.detailUnpaid', { value: formatMoney(d.unpaid_amount) }),
      t('apModule.invoice.detailStatus', { value: getInvoiceStatusLabel(d.invoice_status) }),
      t('apModule.invoice.detailRemark', { value: d.notes || '-' }),
    ];
    await ElMessageBox.alert(lines.join('\n'), t('apModule.invoice.detailTitle'), {
      confirmButtonText: t('common.close'),
    });
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('apModule.invoice.fetchDetailFailed'));
  }
};

const approveInvoice = async (row: APInvoice) => {
  try {
    await ElMessageBox.confirm(
      t('apModule.invoice.approveConfirm'),
      t('apModule.invoice.approveTitle'),
      { type: 'info' }
    );
    await approveAPInvoice(row.id);
    ElMessage.success(t('apModule.invoice.approveSuccess'));
    fetchInvoices();
  } catch (e) {
    if (!isDialogDismissal(e)) {
      const err = e as { message?: string };
      ElMessage.error(err.message || t('common.failed'));
    }
  }
};

const cancelInvoice = async (row: APInvoice) => {
  // 后端 CancelInvoiceRequest 必填 reason：弹框真实采集取消原因（必填校验），取消即中断。
  const reason = await promptCancelReason();
  if (!reason) return;
  try {
    await cancelAPInvoice(row.id, reason);
    ElMessage.success(t('apModule.invoice.cancelSuccess'));
    fetchInvoices();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  }
};

// 批次 157a P1-1 修复：实现应付发票打印（参考 ar 模块 printJS 实现）
const handlePrintInvoices = () => {
  if (invoices.value.length === 0) {
    ElMessage.warning(t('apModule.invoice.noPrintData'));
    return;
  }
  const printData = invoices.value.map((item, index) => ({
    [t('apModule.invoice.colSeq')]: index + 1,
    [t('apModule.invoice.invoiceNo')]: item.invoice_no,
    [t('apModule.invoice.supplier')]: supplierLabel(item.supplier_id),
    [t('apModule.invoice.invoiceAmount')]: `¥${item.amount}`,
    [t('apModule.invoice.taxAmount')]: `¥${item.tax_amount}`,
    [t('common.status')]: getInvoiceStatusLabel(item.invoice_status),
    [t('apModule.invoice.invoiceDate')]: item.invoice_date,
  }));
  printJS({
    printable: printData,
    properties: Object.keys(printData[0] || {}) as string[],
    type: 'json',
    header: t('apModule.invoice.printHeader'),
    style: 'padding: 20px; font-size: 14px;',
    headerStyle: 'font-size: 18px; font-weight: bold; margin-bottom: 20px;',
    gridHeaderStyle: 'font-weight: bold; background-color: #f5f7fa;',
    gridStyle: 'border-collapse: collapse; width: 100%;',
  } as never);
};

// 批次 157a P1-1 修复：实现应付发票导出（规则 3：使用 .xlsx 格式，非 CSV）
// V15 P0-S12 修复（Batch 475e）：迁移到后端导出，注入水印 + 审计日志
const handleExportInvoices = async () => {
  const params: Record<string, unknown> = {
    invoice_status: invoiceQuery.invoice_status || undefined,
  };
  await exportFromBackend('/ap/invoices/export', params, 'ap_invoices_export');
  logger.info(t('apModule.invoice.exportedLog'));
};

const fetchSuppliers = async () => {
  try {
    const res = await getSupplierList({ page: 1, page_size: 1000 });
    suppliers.value = res.data?.items || [];
  } catch (error) {
    logAuxLoadFailure(t('apModule.invoice.loadSuppliersFailed'), error);
  }
};

defineExpose({ refresh: fetchInvoices });

onMounted(() => {
  fetchInvoices();
  fetchSuppliers();
});
</script>

<style scoped>
.invoice-tab {
  background: #fff;
  border-radius: 4px;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.page-title {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
.header-actions {
  display: flex;
  gap: 10px;
}
.filter-card {
  margin-bottom: 20px;
}
.text-red {
  color: #f56c6c;
}
</style>

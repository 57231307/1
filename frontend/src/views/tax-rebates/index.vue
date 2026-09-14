<template>
  <div class="tax-rebates-page">
    <el-card shadow="never">
      <div class="page-header">
        <h2>出口退税管理</h2>
        <div class="header-actions">
          <el-input-number
            v-model="queryYear"
            placeholder="年份"
            :min="2000"
            :max="2100"
            :controls="false"
            style="width: 110px"
            @change="handleFilter"
          />
          <el-input-number
            v-model="queryMonth"
            placeholder="月份"
            :min="1"
            :max="12"
            :controls="false"
            style="width: 90px"
            @change="handleFilter"
          />
          <el-button type="primary" @click="createVisible = true">生成申报表</el-button>
          <el-button type="primary" plain @click="declarationVisible = true">新建报关单</el-button>
          <el-button @click="calcVisible = true">免抵退试算</el-button>
          <el-button @click="verifyVisible = true">单证校验</el-button>
        </div>
      </div>

      <el-table v-loading="loading" :data="declarationList" border>
        <el-table-column prop="declaration_no" label="申报表编号" min-width="180" />
        <el-table-column label="所属期间" width="120" align="center">
          <template #default="{ row }">{{ row.period_year }}-{{ String(row.period_month).padStart(2, '0') }}</template>
        </el-table-column>
        <el-table-column prop="declaration_date" label="申报日期" width="120" align="center" />
        <el-table-column prop="export_sales_amount" label="出口销售额" width="130" align="right" />
        <el-table-column prop="refund_rate" label="退税率" width="90" align="right">
          <template #default="{ row }">{{ formatPercent(row.refund_rate) }}</template>
        </el-table-column>
        <el-table-column prop="refundable_vat_amount" label="免抵退税额" width="130" align="right" />
        <el-table-column prop="actual_refund_amount" label="应退税额" width="130" align="right" />
        <el-table-column prop="documents_complete" label="单证齐全" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="row.documents_complete ? 'success' : 'warning'" size="small">
              {{ row.documents_complete ? '齐全' : '未齐' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="refundDeclarationStatusTagMap[row.status] ?? 'info'">
              {{ statusTextMap[row.status] ?? row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="110" fixed="right">
          <template #default="{ row }">
            <el-button size="small" link type="primary" @click="handlePrint(row)">打印</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="createVisible" title="生成退税申报表" width="520px" @close="resetCreateForm">
      <el-form ref="createFormRef" :model="createForm" :rules="createRules" label-width="120px">
        <el-form-item label="所属年份" prop="period_year">
          <el-input-number v-model="createForm.period_year" :min="2000" :max="2100" :precision="0" style="width: 100%" />
        </el-form-item>
        <el-form-item label="所属月份" prop="period_month">
          <el-input-number v-model="createForm.period_month" :min="1" :max="12" :precision="0" style="width: 100%" />
        </el-form-item>
        <el-form-item label="退税率" prop="refund_rate">
          <el-input-number v-model="createForm.refund_rate" :min="0" :max="1" :precision="4" :step="0.01" style="width: 100%" />
        </el-form-item>
        <el-form-item label="进项税额" prop="input_vat_amount">
          <el-input-number v-model="createForm.input_vat_amount" :min="0" :precision="2" style="width: 100%" />
        </el-form-item>
        <el-form-item label="上期留抵税额" prop="carryforward_from_prev">
          <el-input-number v-model="createForm.carryforward_from_prev" :min="0" :precision="2" style="width: 100%" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="submitCreate">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="declarationVisible" title="新建出口报关单" width="560px" @close="resetDeclarationForm">
      <el-form ref="declarationFormRef" :model="declarationForm" :rules="declarationRules" label-width="120px">
        <el-form-item label="报关单号" prop="declaration_no">
          <el-input v-model="declarationForm.declaration_no" placeholder="必填" />
        </el-form-item>
        <el-form-item label="销售订单 ID" prop="sales_order_id">
          <el-input-number v-model="declarationForm.sales_order_id" :min="1" :precision="0" style="width: 100%" />
        </el-form-item>
        <el-form-item label="出口日期" prop="export_date">
          <el-date-picker
            v-model="declarationForm.export_date"
            type="date"
            value-format="YYYY-MM-DD"
            placeholder="必填"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="目的国" prop="destination_country">
          <el-input v-model="declarationForm.destination_country" />
        </el-form-item>
        <el-form-item label="币种" prop="currency_code">
          <el-input v-model="declarationForm.currency_code" placeholder="如 USD" />
        </el-form-item>
        <el-form-item label="总金额" prop="total_amount">
          <el-input-number v-model="declarationForm.total_amount" :min="0" :precision="2" style="width: 100%" />
        </el-form-item>
        <el-form-item label="汇率" prop="exchange_rate">
          <el-input-number v-model="declarationForm.exchange_rate" :min="0" :precision="6" style="width: 100%" />
        </el-form-item>
        <el-form-item label="海关编码" prop="customs_code">
          <el-input v-model="declarationForm.customs_code" />
        </el-form-item>
        <el-form-item label="备注" prop="remarks">
          <el-input v-model="declarationForm.remarks" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="declarationVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="submitDeclaration">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="calcVisible" title="免抵退税额试算" width="520px">
      <el-form :model="calcForm" label-width="120px">
        <el-form-item label="出口销售额">
          <el-input-number v-model="calcForm.export_sales_amount" :min="0" :precision="2" style="width: 100%" />
        </el-form-item>
        <el-form-item label="退税率">
          <el-input-number v-model="calcForm.refund_rate" :min="0" :max="1" :precision="4" :step="0.01" style="width: 100%" />
        </el-form-item>
        <el-form-item label="进项税额">
          <el-input-number v-model="calcForm.input_vat_amount" :min="0" :precision="2" style="width: 100%" />
        </el-form-item>
        <el-form-item label="上期留抵税额">
          <el-input-number v-model="calcForm.carryforward_from_prev" :min="0" :precision="2" style="width: 100%" />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :loading="submitLoading" @click="submitCalc">计算</el-button>
        </el-form-item>
      </el-form>
      <el-descriptions v-if="calcResult" :column="2" border style="margin-top: 8px">
        <el-descriptions-item label="免抵退税额">{{ calcResult.refundable_vat_amount }}</el-descriptions-item>
        <el-descriptions-item label="应退税额">{{ calcResult.actual_refund_amount }}</el-descriptions-item>
        <el-descriptions-item label="免抵税额">{{ calcResult.exempt_vat_amount }}</el-descriptions-item>
        <el-descriptions-item label="结转下期">{{ calcResult.carryforward_amount }}</el-descriptions-item>
      </el-descriptions>
      <template #footer>
        <el-button @click="calcVisible = false">关闭</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="verifyVisible" title="单证齐全校验" width="420px">
      <el-form label-width="120px">
        <el-form-item label="销售订单 ID">
          <el-input-number v-model="verifyOrderId" :min="1" :precision="0" style="width: 100%" />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :loading="submitLoading" @click="submitVerify">校验</el-button>
        </el-form-item>
      </el-form>
      <el-result
        v-if="verifyResult !== null"
        :icon="verifyResult ? 'success' : 'warning'"
        :title="verifyResult ? '单证齐全' : '单证未齐'"
        :sub-title="`销售订单 ${verifyOrderId ?? '-'}`"
      />
      <template #footer>
        <el-button @click="verifyVisible = false">关闭</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { ElMessage } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import {
  getRefundDeclarationList,
  generateRefundDeclaration,
  createCustomsDeclaration,
  verifyDocumentsCompleteness,
  calculateRefund,
  getRefundDeclarationPrintUrl,
  refundDeclarationStatusTagMap,
  type ExportRefundDeclaration,
  type RefundCalculationResult,
} from '@/api/tax-rebate';

const statusTextMap: Record<string, string> = {
  draft: '草稿',
  pending: '待申报',
  submitted: '已申报',
  approved: '已审核',
  refunded: '已退税',
  rejected: '已驳回',
};

const loading = ref(false);
const submitLoading = ref(false);
const declarationList = ref<ExportRefundDeclaration[]>([]);
const queryYear = ref<number | undefined>(undefined);
const queryMonth = ref<number | undefined>(undefined);

const createVisible = ref(false);
const declarationVisible = ref(false);
const calcVisible = ref(false);
const verifyVisible = ref(false);
const calcResult = ref<RefundCalculationResult | null>(null);
const verifyOrderId = ref<number | undefined>(undefined);
const verifyResult = ref<boolean | null>(null);

const createFormRef = ref<FormInstance>();
const declarationFormRef = ref<FormInstance>();

const createForm = reactive<{
  period_year: number | undefined;
  period_month: number | undefined;
  refund_rate: number | undefined;
  input_vat_amount: number | undefined;
  carryforward_from_prev: number | undefined;
}>({
  period_year: new Date().getFullYear(),
  period_month: new Date().getMonth() + 1,
  refund_rate: undefined,
  input_vat_amount: undefined,
  carryforward_from_prev: 0,
});

const createRules: FormRules = {
  period_year: [{ required: true, message: '请输入所属年份', trigger: 'blur' }],
  period_month: [{ required: true, message: '请输入所属月份', trigger: 'blur' }],
  refund_rate: [{ required: true, message: '请输入退税率', trigger: 'blur' }],
  input_vat_amount: [{ required: true, message: '请输入进项税额', trigger: 'blur' }],
};

const declarationForm = reactive<{
  declaration_no: string;
  sales_order_id: number | undefined;
  export_date: string;
  destination_country: string;
  currency_code: string;
  total_amount: number | undefined;
  exchange_rate: number | undefined;
  customs_code: string;
  remarks: string;
}>({
  declaration_no: '',
  sales_order_id: undefined,
  export_date: '',
  destination_country: '',
  currency_code: '',
  total_amount: undefined,
  exchange_rate: undefined,
  customs_code: '',
  remarks: '',
});

const declarationRules: FormRules = {
  declaration_no: [{ required: true, message: '请输入报关单号', trigger: 'blur' }],
  export_date: [{ required: true, message: '请选择出口日期', trigger: 'change' }],
  total_amount: [{ required: true, message: '请输入总金额', trigger: 'blur' }],
  exchange_rate: [{ required: true, message: '请输入汇率', trigger: 'blur' }],
};

const calcForm = reactive({
  export_sales_amount: undefined as number | undefined,
  refund_rate: undefined as number | undefined,
  input_vat_amount: undefined as number | undefined,
  carryforward_from_prev: 0,
});

/** 响应解包防御：兼容数组 / { items } / { list } 分页包装，避免 el-table "r is not iterable" */
const unwrapList = (payload: unknown): ExportRefundDeclaration[] => {
  if (Array.isArray(payload)) return payload;
  const paged = payload as { items?: ExportRefundDeclaration[]; list?: ExportRefundDeclaration[] } | null;
  return paged?.items ?? paged?.list ?? [];
};

const formatPercent = (value: string | number | null | undefined): string => {
  if (value === null || value === undefined || value === '') return '-';
  const num = Number(value);
  if (!Number.isFinite(num)) return String(value);
  return `${(num * 100).toFixed(2)}%`;
};

const loadList = async () => {
  loading.value = true;
  try {
    const res = await getRefundDeclarationList({
      period_year: queryYear.value || undefined,
      period_month: queryMonth.value || undefined,
    });
    declarationList.value = unwrapList(res.data);
  } catch {
    ElMessage.error('加载退税申报表列表失败');
  } finally {
    loading.value = false;
  }
};

const handleFilter = () => {
  loadList();
};

const resetCreateForm = () => {
  createForm.refund_rate = undefined;
  createForm.input_vat_amount = undefined;
  createForm.carryforward_from_prev = 0;
  createFormRef.value?.resetFields();
};

const resetDeclarationForm = () => {
  declarationForm.declaration_no = '';
  declarationForm.sales_order_id = undefined;
  declarationForm.export_date = '';
  declarationForm.destination_country = '';
  declarationForm.currency_code = '';
  declarationForm.total_amount = undefined;
  declarationForm.exchange_rate = undefined;
  declarationForm.customs_code = '';
  declarationForm.remarks = '';
  declarationFormRef.value?.resetFields();
};

const submitCreate = async () => {
  if (!createFormRef.value) return;
  await createFormRef.value.validate(async valid => {
    if (!valid) return;
    if (
      !createForm.period_year ||
      !createForm.period_month ||
      createForm.refund_rate === undefined ||
      createForm.input_vat_amount === undefined
    ) {
      return;
    }
    submitLoading.value = true;
    try {
      await generateRefundDeclaration({
        period_year: createForm.period_year,
        period_month: createForm.period_month,
        refund_rate: createForm.refund_rate,
        input_vat_amount: createForm.input_vat_amount,
        carryforward_from_prev: createForm.carryforward_from_prev ?? 0,
      });
      ElMessage.success('退税申报表生成成功');
      createVisible.value = false;
      await loadList();
    } catch {
      ElMessage.error('退税申报表生成失败');
    } finally {
      submitLoading.value = false;
    }
  });
};

const submitDeclaration = async () => {
  if (!declarationFormRef.value) return;
  await declarationFormRef.value.validate(async valid => {
    if (!valid) return;
    if (
      !declarationForm.declaration_no ||
      !declarationForm.export_date ||
      declarationForm.total_amount === undefined ||
      declarationForm.exchange_rate === undefined
    ) {
      return;
    }
    submitLoading.value = true;
    try {
      await createCustomsDeclaration({
        declaration_no: declarationForm.declaration_no,
        sales_order_id: declarationForm.sales_order_id,
        export_date: declarationForm.export_date,
        destination_country: declarationForm.destination_country || undefined,
        currency_code: declarationForm.currency_code || undefined,
        total_amount: declarationForm.total_amount,
        exchange_rate: declarationForm.exchange_rate,
        customs_code: declarationForm.customs_code || undefined,
        remarks: declarationForm.remarks || undefined,
      });
      ElMessage.success('报关单创建成功');
      declarationVisible.value = false;
    } catch {
      ElMessage.error('报关单创建失败');
    } finally {
      submitLoading.value = false;
    }
  });
};

const submitCalc = async () => {
  if (
    calcForm.export_sales_amount === undefined ||
    calcForm.refund_rate === undefined ||
    calcForm.input_vat_amount === undefined
  ) {
    ElMessage.warning('请完整填写出口销售额、退税率与进项税额');
    return;
  }
  submitLoading.value = true;
  try {
    const res = await calculateRefund({
      export_sales_amount: calcForm.export_sales_amount,
      refund_rate: calcForm.refund_rate,
      input_vat_amount: calcForm.input_vat_amount,
      carryforward_from_prev: calcForm.carryforward_from_prev ?? 0,
    });
    calcResult.value = res.data;
  } catch {
    ElMessage.error('免抵退税额计算失败');
  } finally {
    submitLoading.value = false;
  }
};

const submitVerify = async () => {
  if (!verifyOrderId.value) return;
  submitLoading.value = true;
  try {
    const res = await verifyDocumentsCompleteness(verifyOrderId.value);
    verifyResult.value = Boolean(res.data?.documents_complete);
  } catch {
    ElMessage.error('单证校验失败');
  } finally {
    submitLoading.value = false;
  }
};

const handlePrint = (row: ExportRefundDeclaration) => {
  window.open(getRefundDeclarationPrintUrl(row.id), '_blank');
};

onMounted(() => {
  loadList();
});
</script>

<style scoped>
.tax-rebates-page {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
  flex-wrap: wrap;
  gap: 12px;
}

.page-header h2 {
  margin: 0;
  font-size: 18px;
}

.header-actions {
  display: flex;
  gap: 12px;
  align-items: center;
  flex-wrap: wrap;
}
</style>

<template>
  <div class="page">
    <el-tabs v-model="activeTab" type="border-card">
      <!-- 出口商检 -->
      <el-tab-pane label="出口商检" name="inspection">
        <el-table v-loading="loading" :data="inspections" border>
          <el-table-column prop="id" label="ID" width="70" />
          <template v-for="col in inspectionCols" :key="col">
            <el-table-column
              :prop="col"
              :label="col.replace(/_/g, ' ')"
              min-width="140"
              show-overflow-tooltip
            />
          </template>
          <el-table-column label="操作" width="220" fixed="right">
            <template #default="{ row }">
              <el-button size="small" @click="onViewCertificates(row)">产地证</el-button>
              <el-button size="small" type="primary" plain @click="onPrintDoc(row)"
                >打印报检单</el-button
              >
            </template>
          </el-table-column>
        </el-table>
        <el-dialog v-model="certVisible" title="产地证列表" width="680">
          <el-table :data="certificates" border max-height="380">
            <el-table-column
              v-for="col in certCols"
              :key="col"
              :prop="col"
              :label="col.replace(/_/g, ' ')"
              min-width="130"
              show-overflow-tooltip
            />
          </el-table>
        </el-dialog>
      </el-tab-pane>

      <!-- 出口退税 -->
      <el-tab-pane label="出口退税" name="refund">
        <el-card shadow="never" class="mb">
          <div class="toolbar">
            <el-button type="primary" @click="declDialogVisible = true">新建报关单</el-button>
            <el-input-number
              v-model="verifyOrderId"
              :min="1"
              placeholder="订单ID"
              style="width: 130px"
            />
            <el-button @click="onVerifyDocs">单证核验</el-button>
            <el-button plain @click="onCalcRefund">计算退税</el-button>
          </div>
        </el-card>

        <pre v-if="refundResult" class="result-box">{{ refundResult }}</pre>
      </el-tab-pane>

      <!-- 贸易术语 -->
      <el-tab-pane label="贸易术语" name="incoterms">
        <el-form inline label-width="110px">
          <el-form-item label="报价单ID"
            ><el-input-number v-model="quotationId" :min="1"
          /></el-form-item>
          <el-form-item
            ><el-button type="primary" @click="onLoadPriceComposition"
              >查询价格构成</el-button
            ></el-form-item
          >
          <el-form-item
            ><el-button plain @click="onLoadUsageReport">术语使用报表</el-button></el-form-item
          >
        </el-form>
        <pre v-if="incotermResult" class="result-box">{{ incotermResult }}</pre>
      </el-tab-pane>

      <!-- 环保税 -->
      <el-tab-pane label="环保税" name="env-tax">
        <el-card shadow="never" class="mb">
          <div class="toolbar">
            <!-- 申报期间：后端 list/declaration 端点 period_year/period_month 为必填查询参数
                 （environmental_tax_handler.rs:19-22 PeriodQuery），环保税按月申报，默认取当前年月 -->
            <span class="period-label">申报期间</span>
            <el-date-picker
              v-model="envPeriod"
              type="month"
              value-format="YYYY-MM"
              :clearable="false"
              @change="loadDischarge"
            />
            <el-button type="primary" @click="openDischargeDialog">新增排放记录</el-button>
            <el-button plain :loading="generating" @click="onGenDeclaration"
              >生成纳税申报</el-button
            >
          </div>
        </el-card>
        <el-table v-loading="loadingDischarge" :data="dischargeRecords" border>
          <el-table-column prop="id" label="ID" width="70" />
          <template v-for="col in dischargeCols" :key="col">
            <el-table-column
              :prop="col"
              :label="col.replace(/_/g, ' ')"
              min-width="140"
              show-overflow-tooltip
            />
          </template>
        </el-table>
        <pre v-if="declarationResult" class="result-box">{{ declarationResult }}</pre>

        <el-dialog v-model="dischargeDialogVisible" title="新增排放记录" width="520">
          <el-form :model="dischargeForm" label-width="110px">
            <el-form-item label="排放类型" required>
              <el-select v-model="dischargeForm.discharge_type" class="w-full">
                <el-option
                  v-for="type in DISCHARGE_TYPES"
                  :key="type"
                  :label="t(`environmentalTax.dischargeType.${type}`)"
                  :value="type"
                />
              </el-select>
            </el-form-item>
            <el-form-item label="污染物名称" required
              ><el-input v-model="dischargeForm.pollutant_name"
            /></el-form-item>
            <el-form-item label="排放量" required
              ><el-input-number
                v-model="dischargeForm.discharge_amount"
                :min="0"
                :precision="2"
                class="w-full"
            /></el-form-item>
            <el-form-item label="计量单位"
              ><el-input v-model="dischargeForm.discharge_unit"
            /></el-form-item>
            <el-form-item label="所属期间" required
              ><el-date-picker
                v-model="dischargeForm.period"
                type="month"
                value-format="YYYY-MM"
                :clearable="false"
                class="w-full"
            /></el-form-item>
            <el-form-item label="浓度"
              ><el-input-number
                v-model="dischargeForm.concentration"
                :min="0"
                :precision="2"
                class="w-full"
            /></el-form-item>
            <el-form-item label="浓度单位"
              ><el-input v-model="dischargeForm.concentration_unit"
            /></el-form-item>
            <el-form-item label="监测点位"
              ><el-input v-model="dischargeForm.monitoring_point"
            /></el-form-item>
            <el-form-item label="备注"
              ><el-input v-model="dischargeForm.remarks" type="textarea" :rows="2"
            /></el-form-item>
          </el-form>
          <template #footer>
            <el-button @click="dischargeDialogVisible = false">取消</el-button>
            <el-button type="primary" :loading="savingDischarge" @click="onCreateDischarge"
              >保存</el-button
            >
          </template>
        </el-dialog>
      </el-tab-pane>
    </el-tabs>

    <el-dialog v-model="declDialogVisible" title="新建报关单" width="480">
      <el-form :model="declForm" label-width="110px">
        <el-form-item label="销售订单ID" required
          ><el-input-number v-model="declForm.sales_order_id" :min="1" class="w-full"
        /></el-form-item>
        <el-form-item label="报关单号" required
          ><el-input v-model="declForm.declaration_no"
        /></el-form-item>
        <el-form-item label="出口日期" required
          ><el-date-picker v-model="declForm.export_date" type="date" value-format="YYYY-MM-DD"
        /></el-form-item>
        <el-form-item label="总金额" required
          ><el-input-number v-model="declForm.total_amount" :min="0" :precision="2"
        /></el-form-item>
        <el-form-item label="汇率"
          ><el-input-number v-model="declForm.exchange_rate" :min="0" :precision="4"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="declDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="onCreateDeclaration">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import {
  createDischargeRecord,
  getDischargeRecords,
  getIncotermUsageReport,
  getPriceComposition,
  getTaxDeclaration,
} from '@/api/export-compliance';
import {
  getExportCertificates,
  getExportInspectionList,
  getExportInspectionPrintUrl,
} from '@/api/export-inspection';
import {
  calculateRefund,
  createCustomsDeclaration,
  verifyDocumentsCompleteness,
} from '@/api/tax-rebate';

const activeTab = ref('inspection');
const { t } = useI18n();

const unwrapList = <T,>(p: unknown): T[] =>
  Array.isArray(p) ? p : ((p as { items?: T[] })?.items ?? []);

const objKeys = (rows: Array<Record<string, unknown>>, skip: string[], n: number) =>
  rows.length
    ? Object.keys(rows[0])
        .filter(k => !skip.includes(k) && typeof rows[0][k] !== 'object')
        .slice(0, n)
    : [];

// 商检
const inspections = ref<Array<Record<string, unknown>>>([]);
const certificates = ref<Array<Record<string, unknown>>>([]);
const loading = ref(false);
const certVisible = ref(false);
const inspectionCols = ref<string[]>([]);
const certCols = ref<string[]>([]);

async function loadInspections() {
  loading.value = true;
  try {
    inspections.value = unwrapList(await getExportInspectionList());
    inspectionCols.value = objKeys(inspections.value, ['id'], 6);
  } finally {
    loading.value = false;
  }
}

async function onViewCertificates(row: Record<string, unknown>) {
  certificates.value = unwrapList(await getExportCertificates(row.id as number));
  certCols.value = objKeys(certificates.value, ['id'], 7);
  certVisible.value = true;
}

function onPrintDoc(row: Record<string, unknown>) {
  window.open(getExportInspectionPrintUrl(row.id as number), '_blank');
}

// 退税（后端仅提供创建/核验/试算/申报写端点，无报关单列表查询）
const refundResult = ref('');
const declDialogVisible = ref(false);
const declForm = reactive({
  sales_order_id: undefined as number | undefined,
  declaration_no: '',
  export_date: '',
  total_amount: undefined as number | undefined,
  exchange_rate: 1,
});
const verifyOrderId = ref<number | undefined>();
const calcForm = reactive({
  export_sales_amount: undefined as number | undefined,
  refund_rate: undefined as number | undefined,
  input_vat_amount: undefined as number | undefined,
  carryforward_from_prev: 0,
});

async function onCreateDeclaration() {
  if (!declForm.sales_order_id) {
    ElMessage.warning('请填写销售订单ID');
    return;
  }
  if (!declForm.declaration_no || !declForm.export_date) {
    ElMessage.warning('请填写报关单号与出口日期');
    return;
  }
  await createCustomsDeclaration({
    declaration_no: declForm.declaration_no,
    sales_order_id: declForm.sales_order_id,
    export_date: declForm.export_date,
    total_amount: Number(declForm.total_amount ?? 0),
    exchange_rate: Number(declForm.exchange_rate ?? 1),
  });
  ElMessage.success('报关单已创建');
  declDialogVisible.value = false;
}

async function onVerifyDocs() {
  if (!verifyOrderId.value) {
    ElMessage.warning('请填写销售订单ID');
    return;
  }
  const res = await verifyDocumentsCompleteness(verifyOrderId.value);
  refundResult.value = JSON.stringify(res, null, 2);
}

async function onCalcRefund() {
  if (!calcForm.export_sales_amount || !calcForm.refund_rate) {
    ElMessage.warning('请填写出口销售额与退税率');
    return;
  }
  const res = await calculateRefund({
    export_sales_amount: Number(calcForm.export_sales_amount ?? 0),
    refund_rate: Number(calcForm.refund_rate ?? 0),
    input_vat_amount: Number(calcForm.input_vat_amount ?? 0),
    carryforward_from_prev: Number(calcForm.carryforward_from_prev ?? 0),
  });
  refundResult.value = JSON.stringify(res, null, 2);
}

// 术语
const quotationId = ref<number | undefined>();
const incotermResult = ref('');

async function onLoadPriceComposition() {
  if (!quotationId.value) {
    ElMessage.warning('请填写报价单ID');
    return;
  }
  incotermResult.value = JSON.stringify(await getPriceComposition(quotationId.value), null, 2);
}

async function onLoadUsageReport() {
  incotermResult.value = JSON.stringify(await getIncotermUsageReport(), null, 2);
}

// 环保税
/**
 * 排放类型受控词表以后端校验为唯一来源：
 * validate_discharge_type（backend/src/services/environmental_tax_service.rs:179-187）
 * → wastewater / exhaust / solid_waste
 */
const DISCHARGE_TYPES = ['wastewater', 'exhaust', 'solid_waste'];

/** 后端可外显文案优先（AppError 失败信封 message），缺失时用 i18n 通用失败兜底键 */
function envErrorText(error: unknown, fallbackKey: string): string {
  const m = (error as { response?: { data?: { message?: string } } })?.response?.data?.message;
  return m && m.trim() !== '' ? m : t(fallbackKey);
}

/** 'YYYY-MM' → 后端 PeriodQuery 必填两键（environmental_tax_handler.rs:19-22） */
function periodQuery(period: string): { period_year: number; period_month: number } {
  const [year, month] = period.split('-');
  return { period_year: Number(year), period_month: Number(month) };
}

const now = new Date();
// 默认当前年月：环保税按自然月申报（《环境保护税法》按月计算、按季申报），当前月即真实默认期间
const envPeriod = ref(`${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`);

const dischargeRecords = ref<Array<Record<string, unknown>>>([]);
const dischargeCols = ref<string[]>([]);
const loadingDischarge = ref(false);
const generating = ref(false);
const declarationResult = ref('');
const dischargeDialogVisible = ref(false);
const savingDischarge = ref(false);
const dischargeForm = reactive({
  discharge_type: '',
  pollutant_name: '',
  discharge_amount: undefined as number | undefined,
  discharge_unit: '',
  period: envPeriod.value,
  concentration: undefined as number | undefined,
  concentration_unit: '',
  monitoring_point: '',
  remarks: '',
});

async function loadDischarge() {
  loadingDischarge.value = true;
  try {
    const res = await getDischargeRecords(periodQuery(envPeriod.value));
    // data 为裸数组 Vec<pollutant_discharge_record::Model>（handler :42-46），非分页信封
    dischargeRecords.value = res.data;
    dischargeCols.value = objKeys(dischargeRecords.value, ['id'], 6);
  } catch (error: unknown) {
    ElMessage.error(envErrorText(error, 'environmentalTax.message.loadFailed'));
  } finally {
    loadingDischarge.value = false;
  }
}

function openDischargeDialog() {
  // 新建记录默认归属当前列表所选期间，用户可在弹窗内改
  dischargeForm.period = envPeriod.value;
  dischargeDialogVisible.value = true;
}

async function onCreateDischarge() {
  if (
    !dischargeForm.discharge_type ||
    !dischargeForm.pollutant_name ||
    dischargeForm.discharge_amount === undefined ||
    !dischargeForm.period
  ) {
    ElMessage.warning(t('environmentalTax.message.required'));
    return;
  }
  savingDischarge.value = true;
  try {
    await createDischargeRecord({
      discharge_type: dischargeForm.discharge_type,
      pollutant_name: dischargeForm.pollutant_name,
      discharge_amount: dischargeForm.discharge_amount,
      // 可选键留空时省略（不发空串/null 覆盖后端默认，如 discharge_unit 缺省由后端落 'kg'）
      discharge_unit: dischargeForm.discharge_unit || undefined,
      concentration: dischargeForm.concentration ?? undefined,
      concentration_unit: dischargeForm.concentration_unit || undefined,
      ...periodQuery(dischargeForm.period),
      monitoring_point: dischargeForm.monitoring_point || undefined,
      remarks: dischargeForm.remarks || undefined,
    });
    ElMessage.success(t('environmentalTax.message.created'));
    dischargeDialogVisible.value = false;
    // 新建记录归属所选申报期间之外的月份时，列表按当前期间过滤可能不呈现，属真实过滤语义
    await loadDischarge();
  } catch (error: unknown) {
    ElMessage.error(envErrorText(error, 'environmentalTax.message.createFailed'));
  } finally {
    savingDischarge.value = false;
  }
}

async function onGenDeclaration() {
  generating.value = true;
  try {
    // 后端 DTO 无 'period' 键，必填 period_year/period_month（Query<PeriodQuery>）；
    // data 为裸数组 Vec<EnvironmentalTaxResult>（handler :50-60），如实展示汇总本体
    const res = await getTaxDeclaration(periodQuery(envPeriod.value));
    declarationResult.value = JSON.stringify(res.data, null, 2);
  } catch (error: unknown) {
    ElMessage.error(envErrorText(error, 'environmentalTax.message.declarationFailed'));
  } finally {
    generating.value = false;
  }
}

onMounted(() => {
  loadInspections();
  loadDischarge();
});
</script>

<style scoped>
.mb {
  margin-bottom: 12px;
}
.mt {
  margin-top: 12px;
}
.toolbar {
  display: flex;
  gap: 8px;
}
.period-label {
  align-self: center;
  font-size: 14px;
}
.w-full {
  width: 100%;
}
.result-box {
  background: var(--el-fill-color-light);
  border-radius: 4px;
  padding: 12px;
  font-size: 12px;
  max-height: 320px;
  overflow: auto;
  white-space: pre-wrap;
}
</style>

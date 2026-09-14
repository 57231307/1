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
            <el-button @click="onVerifyDocs">单证核验</el-button>
          </div>
        </el-card>
        <el-form inline label-width="90px">
          <el-form-item label="销售订单ID"
            ><el-input-number v-model="calcForm.sales_order_id" :min="1"
          /></el-form-item>
          <el-form-item
            ><el-button type="primary" plain @click="onCalcRefund"
              >计算退税</el-button
            ></el-form-item
          >
        </el-form>
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
            <el-button type="primary" @click="dischargeDialogVisible = true"
              >新增排放记录</el-button
            >
            <el-button plain @click="onGenDeclaration">生成纳税申报</el-button>
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

        <el-dialog v-model="dischargeDialogVisible" title="新增排放记录" width="480">
          <el-form :model="dischargeForm" label-width="110px">
            <el-form-item label="污染物代码"
              ><el-input v-model="dischargeForm.pollutant_code"
            /></el-form-item>
            <el-form-item label="排放量"
              ><el-input-number
                v-model="dischargeForm.quantity"
                :min="0"
                :precision="2"
                class="w-full"
            /></el-form-item>
            <el-form-item label="浓度"
              ><el-input-number
                v-model="dischargeForm.concentration"
                :min="0"
                :precision="2"
                class="w-full"
            /></el-form-item>
          </el-form>
          <template #footer>
            <el-button @click="dischargeDialogVisible = false">取消</el-button>
            <el-button type="primary" @click="onCreateDischarge">保存</el-button>
          </template>
        </el-dialog>
      </el-tab-pane>
    </el-tabs>

    <el-dialog v-model="declDialogVisible" title="新建报关单" width="480">
      <el-form :model="declForm" label-width="110px">
        <el-form-item label="销售订单ID" required
          ><el-input-number v-model="declForm.sales_order_id" :min="1" class="w-full"
        /></el-form-item>
        <el-form-item label="报关单号"><el-input v-model="declForm.declaration_no" /></el-form-item>
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
import { ElMessage } from 'element-plus';
import {
  createCustomsDeclaration,
  createDischargeRecord,
  getDischargeRecords,
  getExportCertificates,
  getExportInspectionList,
  getIncotermUsageReport,
  getPriceComposition,
  calculateRefund,
  getTaxDeclaration,
  verifyDocuments,
} from '@/api/export-compliance';

const activeTab = ref('inspection');

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
  window.open(`/api/v1/erp/export-inspections/${row.id}/print`, '_blank');
}

// 退税（后端仅提供创建/核验/试算/申报写端点，无报关单列表查询）
const refundResult = ref('');
const declDialogVisible = ref(false);
const declForm = reactive({ sales_order_id: undefined as number | undefined, declaration_no: '' });
const calcForm = reactive({ sales_order_id: undefined as number | undefined });

async function onCreateDeclaration() {
  if (!declForm.sales_order_id) {
    ElMessage.warning('请填写销售订单ID');
    return;
  }
  await createCustomsDeclaration({
    sales_order_id: declForm.sales_order_id,
    declaration_no: declForm.declaration_no || undefined,
  });
  ElMessage.success('报关单已创建');
  declDialogVisible.value = false;
}

async function onVerifyDocs() {
  if (!calcForm.sales_order_id) {
    ElMessage.warning('请填写销售订单ID');
    return;
  }
  const res = await verifyDocuments(calcForm.sales_order_id);
  refundResult.value = JSON.stringify(res, null, 2);
}

async function onCalcRefund() {
  if (!calcForm.sales_order_id) {
    ElMessage.warning('请填写销售订单ID');
    return;
  }
  const res = await calculateRefund({ sales_order_id: calcForm.sales_order_id });
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
const dischargeRecords = ref<Array<Record<string, unknown>>>([]);
const dischargeCols = ref<string[]>([]);
const loadingDischarge = ref(false);
const declarationResult = ref('');
const dischargeDialogVisible = ref(false);
const dischargeForm = reactive({
  pollutant_code: '',
  quantity: undefined as number | undefined,
  concentration: undefined as number | undefined,
});

async function loadDischarge() {
  loadingDischarge.value = true;
  try {
    dischargeRecords.value = unwrapList(await getDischargeRecords());
    dischargeCols.value = objKeys(dischargeRecords.value, ['id'], 6);
  } finally {
    loadingDischarge.value = false;
  }
}

async function onCreateDischarge() {
  if (!dischargeForm.pollutant_code || !dischargeForm.quantity) {
    ElMessage.warning('请填写污染物代码与排放量');
    return;
  }
  await createDischargeRecord({
    pollutant_code: dischargeForm.pollutant_code,
    quantity: dischargeForm.quantity,
    concentration: dischargeForm.concentration ?? undefined,
  });
  ElMessage.success('排放记录已保存');
  dischargeDialogVisible.value = false;
  await loadDischarge();
}

async function onGenDeclaration() {
  const res = await getTaxDeclaration({ period: new Date().toISOString().slice(0, 7) });
  declarationResult.value = JSON.stringify(res, null, 2);
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

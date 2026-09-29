<template>
  <div class="export-inspections-page">
    <el-card shadow="never">
      <div class="page-header">
        <h2>出口商检管理</h2>
        <div class="header-actions">
          <el-input
            v-model="queryNo"
            placeholder="商检单号"
            clearable
            style="width: 180px"
            @keyup.enter="handleFilter"
            @clear="handleFilter"
          />
          <el-select
            v-model="queryResult"
            placeholder="全部结果"
            clearable
            style="width: 140px"
            @change="handleFilter"
          >
            <el-option
              v-for="(label, key) in resultTextMap"
              :key="key"
              :label="label"
              :value="key"
            />
          </el-select>
        </div>
      </div>

      <el-table v-loading="loading" :data="inspectionList" border>
        <el-table-column prop="inspection_no" label="商检单号" min-width="170" />
        <el-table-column prop="sales_order_id" label="销售订单" width="100" align="center" />
        <el-table-column prop="product_name" label="产品" min-width="130" show-overflow-tooltip />
        <el-table-column prop="hs_code" label="HS编码" width="120" />
        <el-table-column prop="inspection_type" label="商检类型" width="110" align="center" />
        <el-table-column
          prop="inspection_agency"
          label="检验机构"
          min-width="130"
          show-overflow-tooltip
        />
        <el-table-column prop="inspection_date" label="商检日期" width="120" align="center" />
        <el-table-column prop="result" label="结果" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="inspectionResultTagMap[row.result] ?? 'info'">
              {{ resultTextMap[row.result] ?? row.result }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="certificate_no" label="证书号" min-width="140">
          <template #default="{ row }">{{ row.certificate_no || '-' }}</template>
        </el-table-column>
        <el-table-column label="操作" width="150" fixed="right">
          <template #default="{ row }">
            <el-button size="small" link type="primary" @click="handleDetail(row)">详情</el-button>
            <el-button size="small" link type="primary" @click="handlePrint(row)">打印</el-button>
          </template>
        </el-table-column>
      </el-table>

      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :total="total"
        :page-sizes="[10, 20, 50]"
        layout="total, sizes, prev, pager, next"
        style="margin-top: 16px; justify-content: flex-end"
        @current-change="loadList"
        @size-change="handleFilter"
      />
    </el-card>

    <el-dialog v-model="detailVisible" title="商检单详情" width="640px">
      <el-descriptions v-if="detailRow" :column="2" border>
        <el-descriptions-item label="商检单号">{{ detailRow.inspection_no }}</el-descriptions-item>
        <el-descriptions-item label="结果">
          <el-tag :type="inspectionResultTagMap[detailRow.result] ?? 'info'">
            {{ resultTextMap[detailRow.result] ?? detailRow.result }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="销售订单 ID">{{
          detailRow.sales_order_id
        }}</el-descriptions-item>
        <el-descriptions-item label="发货单 ID">{{
          detailRow.delivery_id ?? '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="产品">{{ detailRow.product_name }}</el-descriptions-item>
        <el-descriptions-item label="HS编码">{{ detailRow.hs_code }}</el-descriptions-item>
        <el-descriptions-item label="商检类型">{{
          detailRow.inspection_type
        }}</el-descriptions-item>
        <el-descriptions-item label="检验机构">{{
          detailRow.inspection_agency
        }}</el-descriptions-item>
        <el-descriptions-item label="商检日期">{{
          detailRow.inspection_date
        }}</el-descriptions-item>
        <el-descriptions-item label="证书号">{{
          detailRow.certificate_no || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="证书有效期">{{
          detailRow.certificate_expiry || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="报告链接">{{
          detailRow.report_url || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="备注" :span="2">{{
          detailRow.remarks || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="创建时间" :span="2">{{
          detailRow.created_at
        }}</el-descriptions-item>
      </el-descriptions>
      <template #footer>
        <el-button @click="detailVisible = false">关闭</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { ElMessage } from 'element-plus';
import {
  getExportInspectionList,
  getExportInspectionPrintUrl,
  inspectionResultTagMap,
  type ExportInspection,
} from '@/api/export-inspection';

const resultTextMap: Record<string, string> = {
  pending: '待检',
  pass: '合格',
  fail: '不合格',
};

const loading = ref(false);
const inspectionList = ref<ExportInspection[]>([]);
const total = ref(0);
const page = ref(1);
const pageSize = ref(20);
const queryNo = ref('');
const queryResult = ref('');
const detailVisible = ref(false);
const detailRow = ref<ExportInspection | null>(null);

/** 响应解包防御：兼容数组 / { items } 分页包装，避免 el-table "r is not iterable" */
const unwrapList = (payload: unknown): ExportInspection[] => {
  if (Array.isArray(payload)) return payload;
  const paged = payload as { items?: ExportInspection[] } | null;
  return paged?.items ?? [];
};

const loadList = async () => {
  loading.value = true;
  try {
    const data = await getExportInspectionList({
      page: page.value,
      page_size: pageSize.value,
      inspection_no: queryNo.value || undefined,
      result: queryResult.value || undefined,
    });
    inspectionList.value = unwrapList(data);
    total.value = Array.isArray(data) ? data.length : (data?.total ?? 0);
  } catch {
    ElMessage.error('加载商检单列表失败');
  } finally {
    loading.value = false;
  }
};

const handleFilter = () => {
  page.value = 1;
  loadList();
};

const handleDetail = (row: ExportInspection) => {
  detailRow.value = row;
  detailVisible.value = true;
};

const handlePrint = (row: ExportInspection) => {
  window.open(getExportInspectionPrintUrl(row.id), '_blank');
};

onMounted(() => {
  loadList();
});
</script>

<style scoped>
.export-inspections-page {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.page-header h2 {
  margin: 0;
  font-size: 18px;
}

.header-actions {
  display: flex;
  gap: 12px;
  align-items: center;
}
</style>

<template>
  <div class="certificates-page">
    <el-card shadow="never">
      <div class="page-header">
        <h2>出口产地证管理</h2>
        <div class="header-actions">
          <el-input
            v-model="queryInspectionId"
            placeholder="商检单 ID"
            clearable
            style="width: 160px"
            @keyup.enter="handleFilter"
            @clear="handleFilter"
          />
          <el-select
            v-model="queryStatus"
            placeholder="全部状态"
            clearable
            style="width: 140px"
            @change="handleFilter"
          >
            <el-option
              v-for="(label, key) in statusTextMap"
              :key="key"
              :label="label"
              :value="key"
            />
          </el-select>
        </div>
      </div>

      <el-table v-loading="loading" :data="certificateList" border>
        <el-table-column prop="certificate_no" label="证书编号" min-width="170" />
        <el-table-column prop="inspection_id" label="商检单 ID" width="110" align="center">
          <template #default="{ row }">{{ row.inspection_id ?? '-' }}</template>
        </el-table-column>
        <el-table-column prop="product_name" label="产品" min-width="130" show-overflow-tooltip />
        <el-table-column prop="hs_code" label="HS编码" width="120" />
        <el-table-column prop="origin_country" label="原产国" width="100" align="center" />
        <el-table-column prop="destination_country" label="目的国" width="100" align="center" />
        <el-table-column label="数量" width="110" align="right">
          <template #default="{ row }">{{ row.quantity }} {{ row.unit }}</template>
        </el-table-column>
        <el-table-column prop="certificate_type" label="证书类型" width="110" align="center" />
        <el-table-column prop="issue_date" label="签发日期" width="120" align="center" />
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="certificateStatusTagMap[row.status] ?? 'info'">
              {{ statusTextMap[row.status] ?? row.status }}
            </el-tag>
          </template>
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

    <el-dialog v-model="detailVisible" title="产地证详情" width="640px">
      <el-descriptions v-if="detailRow" :column="2" border>
        <el-descriptions-item label="证书编号">{{ detailRow.certificate_no }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="certificateStatusTagMap[detailRow.status] ?? 'info'">
            {{ statusTextMap[detailRow.status] ?? detailRow.status }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="商检单 ID">{{ detailRow.inspection_id ?? '-' }}</el-descriptions-item>
        <el-descriptions-item label="证书类型">{{ detailRow.certificate_type }}</el-descriptions-item>
        <el-descriptions-item label="产品">{{ detailRow.product_name }}</el-descriptions-item>
        <el-descriptions-item label="HS编码">{{ detailRow.hs_code }}</el-descriptions-item>
        <el-descriptions-item label="原产国">{{ detailRow.origin_country }}</el-descriptions-item>
        <el-descriptions-item label="目的国">{{ detailRow.destination_country }}</el-descriptions-item>
        <el-descriptions-item label="数量">{{ detailRow.quantity }} {{ detailRow.unit }}</el-descriptions-item>
        <el-descriptions-item label="发票金额">{{ detailRow.invoice_amount ?? '-' }}</el-descriptions-item>
        <el-descriptions-item label="签发日期">{{ detailRow.issue_date }}</el-descriptions-item>
        <el-descriptions-item label="有效期至">{{ detailRow.expiry_date || '-' }}</el-descriptions-item>
        <el-descriptions-item label="备注" :span="2">{{ detailRow.remarks || '-' }}</el-descriptions-item>
        <el-descriptions-item label="创建时间" :span="2">{{ detailRow.created_at }}</el-descriptions-item>
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
  getCertificateOfOriginList,
  getCertificateOfOriginPrintUrl,
  certificateStatusTagMap,
  type CertificateOfOrigin,
} from '@/api/certificate-of-origin';

const statusTextMap: Record<string, string> = {
  active: '有效',
  revoked: '已撤销',
  expired: '已过期',
};

const loading = ref(false);
const certificateList = ref<CertificateOfOrigin[]>([]);
const total = ref(0);
const page = ref(1);
const pageSize = ref(20);
const queryInspectionId = ref('');
const queryStatus = ref('');
const detailVisible = ref(false);
const detailRow = ref<CertificateOfOrigin | null>(null);

/** 响应解包防御：兼容数组 / { items } 分页包装，避免 el-table "r is not iterable" */
const unwrapList = (payload: unknown): CertificateOfOrigin[] => {
  if (Array.isArray(payload)) return payload;
  const paged = payload as { items?: CertificateOfOrigin[] } | null;
  return paged?.items ?? [];
};

const loadList = async () => {
  loading.value = true;
  try {
    const inspectionId = Number(queryInspectionId.value);
    const data = await getCertificateOfOriginList({
      page: page.value,
      page_size: pageSize.value,
      inspection_id: Number.isFinite(inspectionId) && inspectionId > 0 ? inspectionId : undefined,
      status: queryStatus.value || undefined,
    });
    certificateList.value = unwrapList(data);
    total.value = Array.isArray(data) ? data.length : (data?.total ?? 0);
  } catch {
    ElMessage.error('加载产地证列表失败');
  } finally {
    loading.value = false;
  }
};

const handleFilter = () => {
  page.value = 1;
  loadList();
};

const handleDetail = (row: CertificateOfOrigin) => {
  detailRow.value = row;
  detailVisible.value = true;
};

const handlePrint = (row: CertificateOfOrigin) => {
  window.open(getCertificateOfOriginPrintUrl(row.id), '_blank');
};

onMounted(() => {
  loadList();
});
</script>

<style scoped>
.certificates-page {
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

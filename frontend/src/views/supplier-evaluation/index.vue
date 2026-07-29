<template>
  <div class="supplier-evaluation">
    <el-card>
      <template #header>
        <div class="card-header">
          <span>{{ t('supplierEvaluation.index.title') }}</span>
        </div>
      </template>

      <el-tabs v-model="activeTab" :aria-label="t('supplierEvaluation.index.tabsAriaLabel')">
        <el-tab-pane :label="t('supplierEvaluation.index.tab.records')" name="records">
          <div class="toolbar">
            <el-button type="primary" @click="handleCreateRecord">{{
              t('supplierEvaluation.index.button.create')
            }}</el-button>
          </div>

          <el-table
            :data="recordList"
            border
            stripe
            :aria-label="t('supplierEvaluation.index.recordsTableAriaLabel')"
          >
            <el-table-column
              prop="supplierName"
              :label="t('supplierEvaluation.index.column.supplierName')"
            />
            <el-table-column prop="period" :label="t('supplierEvaluation.index.column.period')" />
            <el-table-column
              prop="totalScore"
              :label="t('supplierEvaluation.index.column.totalScore')"
            />
            <el-table-column prop="rating" :label="t('supplierEvaluation.index.column.rating')">
              <template #default="{ row }">
                <el-tag v-if="row.rating === 'A'" type="success">A</el-tag>
                <el-tag v-else-if="row.rating === 'B'" type="warning">B</el-tag>
                <el-tag v-else-if="row.rating === 'C'" type="danger">C</el-tag>
                <el-tag v-else type="info">{{ row.rating }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="status" :label="t('supplierEvaluation.index.column.status')" />
            <el-table-column
              prop="evaluatorName"
              :label="t('supplierEvaluation.index.column.evaluator')"
            />
            <el-table-column
              prop="createdAt"
              :label="t('supplierEvaluation.index.column.createdAt')"
            />
            <el-table-column
              :label="t('supplierEvaluation.index.column.operation')"
              fixed="right"
              width="200"
            >
              <template #default="{ row }">
                <el-button link type="primary" @click="handleViewRecord(row as EvaluationRecord)">{{
                  t('supplierEvaluation.index.button.view')
                }}</el-button>
              </template>
            </el-table-column>
          </el-table>

          <el-pagination
            v-model:current-page="recordPage"
            v-model:page-size="recordPageSize"
            :total="recordTotal"
            layout="total, prev, pager, next, jumper"
            :aria-label="t('supplierEvaluation.index.recordsPaginationAriaLabel')"
            @current-change="onRecordPageChange"
          />
        </el-tab-pane>

        <el-tab-pane :label="t('supplierEvaluation.index.tab.rankings')" name="rankings">
          <el-button type="primary" @click="fetchRankings">{{
            t('supplierEvaluation.index.button.refresh')
          }}</el-button>

          <el-table
            :data="rankingList"
            border
            stripe
            :aria-label="t('supplierEvaluation.index.rankingsTableAriaLabel')"
          >
            <el-table-column
              prop="rank"
              :label="t('supplierEvaluation.index.column.rank')"
              width="80"
            >
              <template #default="{ row }">
                <span v-if="row.rank === 1" class="rank-first">🥇 {{ row.rank }}</span>
                <span v-else-if="row.rank === 2" class="rank-second">🥈 {{ row.rank }}</span>
                <span v-else-if="row.rank === 3" class="rank-third">🥉 {{ row.rank }}</span>
                <span v-else>{{ row.rank }}</span>
              </template>
            </el-table-column>
            <el-table-column
              prop="supplierName"
              :label="t('supplierEvaluation.index.column.supplierName')"
            />
            <el-table-column
              prop="totalScore"
              :label="t('supplierEvaluation.index.column.totalScore')"
            />
            <el-table-column prop="rating" :label="t('supplierEvaluation.index.column.rating')">
              <template #default="{ row }">
                <el-tag v-if="row.rating === 'A'" type="success">A</el-tag>
                <el-tag v-else-if="row.rating === 'B'" type="warning">B</el-tag>
                <el-tag v-else-if="row.rating === 'C'" type="danger">C</el-tag>
                <el-tag v-else type="info">{{ row.rating }}</el-tag>
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>
      </el-tabs>
    </el-card>

    <!-- 创建/编辑对话框 -->
    <el-dialog
      v-model="recordDialogVisible"
      :title="
        isEdit
          ? t('supplierEvaluation.index.dialog.editTitle')
          : t('supplierEvaluation.index.dialog.createTitle')
      "
      width="600px"
      :aria-label="t('supplierEvaluation.index.dialog.ariaLabel')"
    >
      <el-form
        ref="recordFormRef"
        :model="recordForm"
        :rules="recordRules"
        label-width="120px"
        :aria-label="t('supplierEvaluation.index.dialog.formAriaLabel')"
      >
        <el-form-item
          :label="t('supplierEvaluation.index.dialog.label.supplier')"
          prop="supplierId"
        >
          <el-select
            v-model="recordForm.supplierId"
            :placeholder="t('supplierEvaluation.index.dialog.placeholder.supplier')"
            style="width: 100%"
            filterable
          >
            <el-option
              v-for="supplier in supplierList"
              :key="supplier.id"
              :label="supplier.supplier_name"
              :value="supplier.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('supplierEvaluation.index.dialog.label.period')" prop="period">
          <el-input
            v-model="recordForm.period"
            :placeholder="t('supplierEvaluation.index.dialog.placeholder.period')"
          />
        </el-form-item>
        <el-form-item :label="t('supplierEvaluation.index.dialog.label.remark')" prop="remark">
          <el-input v-model="recordForm.remark" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="recordDialogVisible = false">{{
          t('supplierEvaluation.index.dialog.button.cancel')
        }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSaveRecord">{{
          t('supplierEvaluation.index.dialog.button.save')
        }}</el-button>
      </template>
    </el-dialog>

    <!-- 详情对话框 -->
    <el-dialog
      v-model="detailDialogVisible"
      :title="t('supplierEvaluation.index.detail.title')"
      width="700px"
      :aria-label="t('supplierEvaluation.index.detail.dialogAriaLabel')"
    >
      <el-descriptions
        v-if="currentRecord"
        :column="2"
        border
        :aria-label="t('supplierEvaluation.index.detail.ariaLabel')"
      >
        <el-descriptions-item :label="t('supplierEvaluation.index.detail.label.supplier')">{{
          currentRecord.supplierName
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('supplierEvaluation.index.detail.label.period')">{{
          currentRecord.period
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('supplierEvaluation.index.detail.label.totalScore')">{{
          currentRecord.totalScore
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('supplierEvaluation.index.detail.label.rating')">{{
          currentRecord.rating
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('supplierEvaluation.index.detail.label.status')">{{
          currentRecord.status
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('supplierEvaluation.index.detail.label.evaluator')">{{
          currentRecord.evaluatorName
        }}</el-descriptions-item>
        <el-descriptions-item
          :label="t('supplierEvaluation.index.detail.label.createdAt')"
          :span="2"
          >{{ currentRecord.createdAt }}</el-descriptions-item
        >
        <el-descriptions-item
          :label="t('supplierEvaluation.index.detail.label.remark')"
          :span="2"
          >{{ currentRecord.remark || '-' }}</el-descriptions-item
        >
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import { ElMessage } from 'element-plus';
import { useTableApi } from '@/composables/useTableApi';
import {
  createEvaluationRecord,
  getSupplierRankings,
  type EvaluationRecord,
  type SupplierScore,
  type CreateEvaluationRequest,
} from '@/api/supplier-evaluation';
import { getSupplierList, type Supplier } from '@/api/supplier';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

const activeTab = ref('records');
const rankingList = ref<SupplierScore[]>([]);
const supplierList = ref<Supplier[]>([]);

// 批次 268：接入 useTableApi，消除手写 recordPagination + fetchRecords 重复
// API 参数用驼峰 pageSize，配置 pageSizeKey: 'pageSize'
// API 返回 res.data.list，useTableApi 默认 listKey='list' 兼容
const {
  data: recordList,
  page: recordPage,
  pageSize: recordPageSize,
  total: recordTotal,
  refresh: fetchRecords,
} = useTableApi<EvaluationRecord>({
  url: '/purchase/supplier-evaluations/records',
  pageSizeKey: 'pageSize',
  onError: () => ElMessage.error(t('supplierEvaluation.index.message.fetchRecordsFailed')),
});

const recordDialogVisible = ref(false);
const detailDialogVisible = ref(false);
const isEdit = ref(false);
const submitLoading = ref(false);
const recordFormRef = ref();
const currentRecord = ref<EvaluationRecord | null>(null);

const recordForm = reactive({
  supplierId: undefined as number | undefined,
  period: '',
  remark: '',
});

const recordRules = computed(() => ({
  supplierId: [
    {
      required: true,
      message: t('supplierEvaluation.index.validation.supplierRequired'),
      trigger: 'change',
    },
  ],
  period: [
    {
      required: true,
      message: t('supplierEvaluation.index.validation.periodRequired'),
      trigger: 'blur',
    },
  ],
}));

const fetchSuppliers = async () => {
  try {
    const res = await getSupplierList({ page: 1, page_size: 1000 });
    supplierList.value = res.data?.list || [];
  } catch (e) {
    logger.error(t('supplierEvaluation.index.message.fetchSuppliersFailed'), String(e));
  }
};

const fetchRankings = async () => {
  try {
    // v11 批次 176 P2-1 修复：res: any 改为直接使用 API 返回类型
    const res = await getSupplierRankings({ limit: 20 });
    if (res.data) {
      rankingList.value = (res.data || []).map((item: SupplierScore, index: number) => ({
        ...item,
        rank: index + 1,
      }));
    }
  } catch (e) {
    ElMessage.error(t('supplierEvaluation.index.message.fetchRankingsFailed'));
  }
};

const handleCreateRecord = () => {
  isEdit.value = false;
  Object.assign(recordForm, { supplierId: undefined, period: '', remark: '' });
  recordDialogVisible.value = true;
};

const handleViewRecord = (row: EvaluationRecord) => {
  currentRecord.value = row;
  detailDialogVisible.value = true;
};

const handleSaveRecord = async () => {
  if (!recordFormRef.value) return;

  await recordFormRef.value.validate(async (valid: boolean) => {
    if (!valid) return;

    submitLoading.value = true;
    try {
      // v11 批次 176 P2-1 修复：recordForm as any 改为 as CreateEvaluationRequest
      await createEvaluationRecord(recordForm as CreateEvaluationRequest);
      ElMessage.success(t('supplierEvaluation.index.message.saveSuccess'));
      recordDialogVisible.value = false;
      fetchRecords();
    } catch (e: unknown) {
      const errMsg = e instanceof Error ? e.message : String(e);
      ElMessage.error(errMsg || t('supplierEvaluation.index.message.saveFailed'));
    } finally {
      submitLoading.value = false;
    }
  });
};

// 批次 268：分页变化（useTableApi 自动 watch 重载，此处无需手动调用）
const onRecordPageChange = (_p: number) => {
  // useTableApi watch page 自动触发 refresh
};

const hasLoaded = createLazyLoader();

// 批次 268：useTableApi 构造时自动初始加载列表，onMounted 仅加载排名 + 供应商下拉
onMounted(() => {
  loadIfNot('rankings', fetchRankings, hasLoaded);
  loadIfNot('suppliers', fetchSuppliers, hasLoaded);
});
</script>

<style scoped>
.supplier-evaluation .card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.supplier-evaluation .toolbar {
  margin-bottom: 16px;
}

.supplier-evaluation .el-table {
  margin-bottom: 16px;
}

.supplier-evaluation .rank-first {
  color: #ffd700;
  font-weight: bold;
}

.supplier-evaluation .rank-second {
  color: #c0c0c0;
  font-weight: bold;
}

.supplier-evaluation .rank-third {
  color: #cd7f32;
  font-weight: bold;
}
</style>

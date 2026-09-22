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

        <el-tab-pane :label="t('supplierEvaluation.index.tab.indicators')" name="indicators">
          <div class="toolbar">
            <el-button type="primary" @click="indicatorDialogVisible = true">{{
              t('supplierEvaluation.index.indicators.dialogTitle')
            }}</el-button>
          </div>
          <el-table
            v-loading="indicatorLoading"
            :data="indicatorList"
            border
            stripe
            :aria-label="t('supplierEvaluation.index.indicators.tableAriaLabel')"
          >
            <el-table-column
              prop="indicatorCode"
              :label="t('supplierEvaluation.index.indicators.label.code')"
              width="140"
            />
            <el-table-column
              prop="indicatorName"
              :label="t('supplierEvaluation.index.indicators.label.name')"
            />
            <el-table-column
              prop="category"
              :label="t('supplierEvaluation.index.indicators.label.category')"
              width="120"
            />
            <el-table-column
              prop="weight"
              :label="t('supplierEvaluation.index.indicators.label.weight')"
              width="90"
            />
            <el-table-column
              prop="maxScore"
              :label="t('supplierEvaluation.index.indicators.label.maxScore')"
              width="90"
            />
            <el-table-column
              prop="status"
              :label="t('supplierEvaluation.index.column.status')"
              width="100"
            />
            <el-table-column
              prop="description"
              :label="t('supplierEvaluation.index.indicators.label.description')"
              min-width="160"
              show-overflow-tooltip
            />
          </el-table>
        </el-tab-pane>

        <el-tab-pane :label="t('supplierEvaluation.index.tab.evaluations')" name="evaluations">
          <div class="toolbar">
            <el-button type="primary" @click="handleCreateEvaluation">{{
              t('supplierEvaluation.index.button.create')
            }}</el-button>
          </div>
          <el-table
            v-loading="evaluationLoading"
            :data="evaluationList"
            border
            stripe
            :aria-label="t('supplierEvaluation.index.evaluations.tableAriaLabel')"
          >
            <el-table-column prop="id" label="ID" width="80" />
            <el-table-column
              prop="supplierName"
              :label="t('supplierEvaluation.index.column.supplierName')"
            />
            <el-table-column prop="period" :label="t('supplierEvaluation.index.column.period')" />
            <el-table-column
              prop="totalScore"
              :label="t('supplierEvaluation.index.column.totalScore')"
            />
            <el-table-column prop="rating" :label="t('supplierEvaluation.index.column.rating')" />
            <el-table-column prop="status" :label="t('supplierEvaluation.index.column.status')" />
            <el-table-column
              :label="t('supplierEvaluation.index.column.operation')"
              fixed="right"
              width="220"
            >
              <template #default="{ row }">
                <el-button
                  link
                  type="primary"
                  @click="handleViewEvaluation(row as EvaluationRecord)"
                  >{{ t('supplierEvaluation.index.button.view') }}</el-button
                >
                <el-button
                  link
                  type="primary"
                  @click="handleEditEvaluation(row as EvaluationRecord)"
                  >{{ t('supplierEvaluation.index.evaluations.button.edit') }}</el-button
                >
                <el-button
                  link
                  type="danger"
                  @click="handleDeleteEvaluation(row as EvaluationRecord)"
                  >{{ t('supplierEvaluation.index.evaluations.button.delete') }}</el-button
                >
              </template>
            </el-table-column>
          </el-table>
          <div class="toolbar">
            <span class="score-label">{{ t('supplierEvaluation.index.score.label') }}</span>
            <el-input
              v-model="scoreSupplierId"
              :placeholder="t('supplierEvaluation.index.score.placeholder')"
              style="width: 160px"
            />
            <el-button type="primary" plain @click="handleQueryScore">{{
              t('supplierEvaluation.index.score.button')
            }}</el-button>
            <el-descriptions v-if="scoreResult" :column="3" border class="score-result">
              <el-descriptions-item :label="t('supplierEvaluation.index.score.totalScore')">{{
                scoreResult.totalScore
              }}</el-descriptions-item>
              <el-descriptions-item :label="t('supplierEvaluation.index.score.rating')">{{
                scoreResult.rating
              }}</el-descriptions-item>
              <el-descriptions-item :label="t('supplierEvaluation.index.score.rank')">{{
                scoreResult.rank
              }}</el-descriptions-item>
            </el-descriptions>
          </div>
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

    <!-- 新增评估指标对话框（createIndicator + handleSaveIndicator） -->
    <el-dialog
      v-model="indicatorDialogVisible"
      :title="t('supplierEvaluation.index.indicators.dialogTitle')"
      width="520px"
    >
      <el-form :model="indicatorForm" label-width="110px">
        <el-form-item :label="t('supplierEvaluation.index.indicators.label.code')" required>
          <el-input v-model="indicatorForm.indicatorCode" />
        </el-form-item>
        <el-form-item :label="t('supplierEvaluation.index.indicators.label.name')" required>
          <el-input v-model="indicatorForm.indicatorName" />
        </el-form-item>
        <el-form-item :label="t('supplierEvaluation.index.indicators.label.category')" required>
          <el-input v-model="indicatorForm.category" />
        </el-form-item>
        <el-form-item :label="t('supplierEvaluation.index.indicators.label.weight')">
          <el-input-number v-model="indicatorForm.weight" :min="0" :max="100" />
        </el-form-item>
        <el-form-item :label="t('supplierEvaluation.index.indicators.label.maxScore')">
          <el-input-number v-model="indicatorForm.maxScore" :min="0" :max="100" />
        </el-form-item>
        <el-form-item :label="t('supplierEvaluation.index.indicators.label.description')">
          <el-input v-model="indicatorForm.description" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="indicatorDialogVisible = false">{{
          t('supplierEvaluation.index.dialog.button.cancel')
        }}</el-button>
        <el-button type="primary" @click="handleSaveIndicator">{{
          t('supplierEvaluation.index.dialog.button.save')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import { ElMessage, ElMessageBox } from 'element-plus';
import { useTableApi } from '@/composables/useTableApi';
import {
  createEvaluationRecord,
  getSupplierRankings,
  getEvaluationIndicatorList,
  createIndicator,
  getEvaluationRecord,
  getEvaluationList,
  getEvaluation,
  updateEvaluation,
  deleteEvaluation,
  getSupplierScore,
  type EvaluationRecord,
  type EvaluationIndicator,
  type SupplierScore,
  type CreateEvaluationRequest,
  type CreateEvaluationIndicatorRequest,
} from '@/api/supplier-evaluation';
import { getSupplierList, type Supplier } from '@/api/supplier';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

const activeTab = ref('records');
const rankingList = ref<SupplierScore[]>([]);
const supplierList = ref<Supplier[]>([]);

// 批次 268：接入 useTableApi，消除手写 recordPagination + fetchRecords 重复
// API 参数用驼峰 pageSize，配置 pageSizeKey: 'pageSize'
// API 返回 res.data?.items，useTableApi 默认 listKey='list' 兼容
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
    supplierList.value = res.data?.items || [];
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
  void handleViewRecordRemote(row);
};

// 详情回源：按 ID 重新拉取最新数据，失败时保留行数据
const handleViewRecordRemote = async (row: EvaluationRecord) => {
  if (!row.id) return;
  try {
    const res = await getEvaluationRecord(row.id);
    if (res.data) {
      currentRecord.value = res.data;
      detailDialogVisible.value = true;
    }
  } catch {
    logger.error(t('supplierEvaluation.index.message.fetchDetailFailed'), String(row.id));
  }
};

// 评估指标管理
const indicatorList = ref<EvaluationIndicator[]>([]);
const indicatorLoading = ref(false);
const indicatorDialogVisible = ref(false);
const indicatorForm = reactive({
  indicatorCode: '',
  indicatorName: '',
  category: '',
  weight: 10,
  maxScore: 100,
  description: '',
});

const fetchIndicators = async () => {
  indicatorLoading.value = true;
  try {
    const res = await getEvaluationIndicatorList({ page: 1, pageSize: 100 });
    // 后端 list_indicators 返回 ApiResponse<Vec<Model>> ⇒ 裸数组
    indicatorList.value = res.data;
  } catch {
    ElMessage.error(t('supplierEvaluation.index.message.fetchIndicatorsFailed'));
  } finally {
    indicatorLoading.value = false;
  }
};

const handleSaveIndicator = async () => {
  if (!indicatorForm.indicatorCode || !indicatorForm.indicatorName || !indicatorForm.category) {
    ElMessage.warning(
      `${t('supplierEvaluation.index.indicators.label.code')}/${t('supplierEvaluation.index.indicators.label.name')}/${t('supplierEvaluation.index.indicators.label.category')}`
    );
    return;
  }
  try {
    await createIndicator(indicatorForm as CreateEvaluationIndicatorRequest);
    ElMessage.success(t('supplierEvaluation.index.message.saveSuccess'));
    indicatorDialogVisible.value = false;
    await fetchIndicators();
  } catch (e: unknown) {
    const errMsg = e instanceof Error ? e.message : String(e);
    ElMessage.error(errMsg || t('supplierEvaluation.index.message.saveFailed'));
  }
};

// 评估管理（正式评估 CRUD）
const evaluationList = ref<EvaluationRecord[]>([]);
const evaluationLoading = ref(false);
const editingEvaluationId = ref<number | null>(null);

const fetchEvaluations = async () => {
  evaluationLoading.value = true;
  try {
    const res = await getEvaluationList({ page: 1, pageSize: 50 });
    // 后端 list_evaluations 返回 ApiResponse<Vec<Model>> ⇒ 裸数组
    evaluationList.value = res.data;
  } catch {
    ElMessage.error(t('supplierEvaluation.index.message.fetchEvaluationsFailed'));
  } finally {
    evaluationLoading.value = false;
  }
};

const handleCreateEvaluation = () => {
  isEdit.value = false;
  editingEvaluationId.value = null;
  Object.assign(recordForm, { supplierId: undefined, period: '', remark: '' });
  recordDialogVisible.value = true;
};

const handleEditEvaluation = (row: EvaluationRecord) => {
  isEdit.value = true;
  editingEvaluationId.value = row.id ?? null;
  Object.assign(recordForm, {
    supplierId: row.supplierId,
    period: row.period || '',
    remark: row.remark || '',
  });
  recordDialogVisible.value = true;
};

const handleViewEvaluation = async (row: EvaluationRecord) => {
  currentRecord.value = row;
  detailDialogVisible.value = true;
  if (!row.id) return;
  try {
    const res = await getEvaluation(row.id);
    if (res.data) {
      currentRecord.value = res.data;
      detailDialogVisible.value = true;
    }
  } catch {
    logger.error(t('supplierEvaluation.index.message.fetchDetailFailed'), String(row.id));
  }
};

const handleDeleteEvaluation = async (row: EvaluationRecord) => {
  if (!row.id) return;
  try {
    await ElMessageBox.confirm(
      t('supplierEvaluation.index.evaluations.message.deleteConfirm'),
      t('common.warning'),
      { type: 'warning' }
    );
  } catch {
    return;
  }
  try {
    await deleteEvaluation(row.id);
    ElMessage.success(t('supplierEvaluation.index.evaluations.message.deleteSuccess'));
    await fetchEvaluations();
  } catch {
    ElMessage.error(t('supplierEvaluation.index.message.deleteFailed'));
  }
};

// 供应商评分查询
const scoreSupplierId = ref('');
const scoreResult = ref<SupplierScore | null>(null);

const handleQueryScore = async () => {
  const supplierId = Number(scoreSupplierId.value);
  if (!supplierId) {
    ElMessage.warning(t('supplierEvaluation.index.score.placeholder'));
    return;
  }
  try {
    const res = await getSupplierScore(supplierId);
    scoreResult.value = res.data ?? null;
    if (!scoreResult.value) {
      ElMessage.info(t('supplierEvaluation.index.message.fetchScoreFailed'));
    }
  } catch {
    ElMessage.error(t('supplierEvaluation.index.message.fetchScoreFailed'));
  }
};

const handleSaveRecord = async () => {
  if (!recordFormRef.value) return;

  await recordFormRef.value.validate(async (valid: boolean) => {
    if (!valid) return;

    submitLoading.value = true;
    try {
      if (isEdit.value && editingEvaluationId.value) {
        await updateEvaluation(editingEvaluationId.value, {
          period: recordForm.period,
          remark: recordForm.remark,
        });
      } else {
        await createEvaluationRecord(recordForm as CreateEvaluationRequest);
      }
      ElMessage.success(t('supplierEvaluation.index.message.saveSuccess'));
      recordDialogVisible.value = false;
      fetchRecords();
      if (activeTab.value === 'evaluations') await fetchEvaluations();
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
  loadIfNot('indicators', fetchIndicators, hasLoaded);
  loadIfNot('evaluations', fetchEvaluations, hasLoaded);
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

.supplier-evaluation .score-label {
  margin-right: 8px;
  font-size: 14px;
}

.supplier-evaluation .score-result {
  margin-top: 12px;
  width: 100%;
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

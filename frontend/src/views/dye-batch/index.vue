<template>
  <div class="dye-batch-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">{{ t('dyeBatch.index.pageTitle') }}</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">{{
            t('dyeBatch.index.breadcrumbHome')
          }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ t('dyeBatch.index.breadcrumbFabric') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ t('dyeBatch.index.breadcrumbDyeBatch') }}</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          {{ t('dyeBatch.index.buttonCreate') }}
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          {{ t('dyeBatch.index.buttonExport') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form
        :inline="true"
        :model="queryParams"
        class="filter-form"
        :aria-label="t('dyeBatch.index.ariaFilterForm')"
      >
        <el-form-item :label="t('dyeBatch.index.filterColorNo')">
          <el-input
            v-model="queryParams.color_no"
            :placeholder="t('dyeBatch.index.placeholderColorNo')"
            clearable
            @clear="handleQuery"
          />
        </el-form-item>
        <el-form-item :label="t('dyeBatch.index.filterBatchNo')">
          <el-input
            v-model="queryParams.batch_no"
            :placeholder="t('dyeBatch.index.placeholderBatchNo')"
            clearable
            @clear="handleQuery"
          />
        </el-form-item>
        <el-form-item :label="t('dyeBatch.index.filterDyeLotNo')">
          <el-input
            v-model="queryParams.dye_lot_no"
            :placeholder="t('dyeBatch.index.placeholderDyeLotNo')"
            clearable
            @clear="handleQuery"
          />
        </el-form-item>
        <el-form-item :label="t('dyeBatch.index.filterStatus')">
          <el-select
            v-model="queryParams.status"
            :placeholder="t('dyeBatch.index.placeholderStatus')"
            clearable
            @change="handleQuery"
          >
            <el-option
              v-for="s in DYE_BATCH_LIFECYCLE_STATUSES"
              :key="s"
              :label="statusLabelFor(s)"
              :value="s"
            />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">
            <el-icon><Search /></el-icon>
            {{ t('dyeBatch.index.buttonQuery') }}
          </el-button>
          <el-button @click="handleReset">
            <el-icon><Refresh /></el-icon>
            {{ t('dyeBatch.index.buttonReset') }}
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table
        v-loading="loading"
        :data="dyeBatchList"
        border
        stripe
        :aria-label="t('dyeBatch.index.ariaTable')"
      >
        <el-table-column
          type="index"
          :label="t('dyeBatch.index.colIndex')"
          width="60"
          align="center"
        />
        <el-table-column
          prop="batch_no"
          :label="t('dyeBatch.index.colBatchNo')"
          width="120"
          show-overflow-tooltip
        />
        <el-table-column
          prop="greige_fabric_name"
          :label="t('dyeBatch.index.colProduct')"
          width="150"
          show-overflow-tooltip
        />
        <el-table-column
          prop="color_no"
          :label="t('dyeBatch.index.colColorNo')"
          width="100"
          show-overflow-tooltip
        />
        <el-table-column
          prop="color_code"
          :label="t('dyeBatch.index.colColorCode')"
          width="100"
          show-overflow-tooltip
        />
        <el-table-column
          prop="started_at"
          :label="t('dyeBatch.index.colDyeDate')"
          width="120"
          align="center"
        >
          <template #default="{ row }">{{ formatDate(row.started_at) }}</template>
        </el-table-column>
        <el-table-column
          prop="planned_quantity"
          :label="t('dyeBatch.index.colQuantity')"
          width="100"
          align="right"
        />
        <el-table-column
          prop="status"
          :label="t('dyeBatch.index.colStatus')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="remarks"
          :label="t('dyeBatch.index.colRemarks')"
          min-width="150"
          show-overflow-tooltip
        />
        <el-table-column
          :label="t('dyeBatch.index.colOperation')"
          width="200"
          align="center"
          fixed="right"
        >
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleView(row as DyeBatch)">{{
              t('dyeBatch.index.buttonView')
            }}</el-button>
            <el-button
              v-if="canEdit(row as DyeBatch)"
              type="primary"
              link
              size="small"
              @click="handleEdit(row as DyeBatch)"
              >{{ t('dyeBatch.index.buttonEdit') }}</el-button
            >
            <el-button
              v-if="canComplete(row as DyeBatch)"
              type="success"
              link
              size="small"
              @click="handleComplete(row as DyeBatch)"
              >{{ t('dyeBatch.index.buttonComplete') }}</el-button
            >
            <el-button
              v-if="canDelete(row as DyeBatch)"
              type="danger"
              link
              size="small"
              @click="handleDelete(row as DyeBatch)"
              >{{ t('dyeBatch.index.buttonDelete') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-container">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          :aria-label="t('dyeBatch.index.ariaPagination')"
          @size-change="handleSizeChange"
          @current-change="handleCurrentChange"
        />
      </div>
    </el-card>

    <!-- 新建/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="dialogTitle"
      width="700px"
      :close-on-click-modal="false"
      :aria-label="t('dyeBatch.index.ariaEditDialog')"
    >
      <el-form
        ref="formRef"
        :model="formData"
        :rules="formRules"
        :disabled="isView"
        label-width="100px"
        :aria-label="t('dyeBatch.index.ariaForm')"
      >
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('dyeBatch.index.colBatchNo')" prop="batch_no">
              <el-input
                v-model="formData.batch_no"
                :placeholder="t('dyeBatch.index.placeholderBatchNo')"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('dyeBatch.index.colProduct')" prop="greige_fabric_id">
              <el-select
                v-model="formData.greige_fabric_id"
                :placeholder="t('dyeBatch.index.placeholderSelectProduct')"
                filterable
              >
                <el-option
                  v-for="gf in greigeFabrics"
                  :key="gf.id"
                  :label="gf.fabric_name || gf.fabric_no"
                  :value="gf.id"
                />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('dyeBatch.index.colColorNo')" prop="color_no">
              <el-input
                v-model="formData.color_no"
                :placeholder="t('dyeBatch.index.placeholderColorNo')"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('dyeBatch.index.colColorCode')" prop="color_code">
              <el-input
                v-model="formData.color_code"
                :placeholder="t('dyeBatch.index.placeholderColorCode')"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('dyeBatch.index.colDyeDate')" prop="dye_date">
              <el-date-picker
                v-model="formData.dye_date"
                type="date"
                value-format="YYYY-MM-DD"
                :placeholder="t('dyeBatch.index.placeholderSelectDyeDate')"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('dyeBatch.index.colQuantity')" prop="planned_quantity">
              <el-input-number
                v-model="formData.planned_quantity"
                :precision="2"
                :min="0"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item :label="t('dyeBatch.index.colRemarks')" prop="remarks">
          <el-input
            v-model="formData.remarks"
            type="textarea"
            :rows="3"
            :placeholder="t('dyeBatch.index.placeholderRemarks')"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{
          isView ? t('dyeBatch.index.buttonClose') : t('dyeBatch.index.buttonCancel')
        }}</el-button>
        <el-button v-if="!isView" type="primary" @click="handleSubmitForm">{{
          t('dyeBatch.index.buttonConfirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus, Download, Search, Refresh } from '@element-plus/icons-vue';
import {
  createDyeBatch,
  updateDyeBatch,
  deleteDyeBatch,
  completeDyeBatch,
  exportDyeBatches,
} from '@/api/dye-batch';
import type { DyeBatch } from '@/api/dye-batch';
import {
  DYE_BATCH_LIFECYCLE_STATUS,
  DYE_BATCH_LIFECYCLE_STATUSES,
  dyeBatchStatusLabelKey,
  dyeBatchStatusTagType,
  normalizeDyeBatchStatus,
  type DyeBatchLifecycleStatus,
} from '@/utils/dye-batch-status';
import { getGreigeFabricList, type GreigeFabric } from '@/api/greige-fabric';
import { logger } from '@/utils/logger';
import { useTableApi } from '@/composables/useTableApi';

const { t } = useI18n({ useScope: 'global' });

// 查询参数（仅保留后端 DyeBatchListQuery 真实读取的筛选：色号、批次号、缸号、状态；分页由 useTableApi 管理）
const queryParams = reactive({
  color_no: '',
  batch_no: '',
  dye_lot_no: '',
  status: '',
});

// 批次 271：接入 useTableApi，消除手写 page/pageSize/total/loading + getList 重复
// useTableApi 自动管理分页状态、loading、数据加载，自动 watch page/pageSize 变化触发重载
const {
  data: dyeBatchList,
  loading,
  page,
  pageSize,
  total,
  refresh,
  setQueryParam,
} = useTableApi<DyeBatch>({
  url: '/production/dye-batches',
  onError: (e: unknown) => logger.error(t('dyeBatch.index.messageFetchFailed'), String(e)),
});

// 坯布列表（染色批次消耗坯布，以 greige_fabric_id 关联）
const greigeFabrics = ref<GreigeFabric[]>([]);

// 对话框
const dialogVisible = ref(false);
const dialogTitle = ref('');
const formRef = ref();
const isView = ref(false);

// 表单数据
const formData = reactive({
  id: undefined as number | undefined,
  batch_no: '',
  greige_fabric_id: undefined as number | undefined,
  color_no: '',
  color_code: '',
  dye_date: '',
  planned_quantity: 0,
  remarks: '',
});

// 表单验证规则
const formRules = {
  batch_no: [{ required: true, message: t('dyeBatch.index.ruleBatchNoRequired'), trigger: 'blur' }],
  greige_fabric_id: [
    { required: true, message: t('dyeBatch.index.ruleProductRequired'), trigger: 'change' },
  ],
  color_no: [{ required: true, message: t('dyeBatch.index.ruleColorNoRequired'), trigger: 'blur' }],
  dye_date: [
    { required: true, message: t('dyeBatch.index.ruleDyeDateRequired'), trigger: 'change' },
  ],
  planned_quantity: [
    { required: true, message: t('dyeBatch.index.ruleQuantityRequired'), trigger: 'blur' },
  ],
};

// 批次 271：同步筛选条件到 useTableApi.queryParams 并刷新
// useTableApi 自动 watch page/pageSize 变化触发重载，无需手动 getList
const syncQueryParams = () => {
  setQueryParam('color_no', queryParams.color_no || undefined);
  setQueryParam('batch_no', queryParams.batch_no || undefined);
  setQueryParam('dye_lot_no', queryParams.dye_lot_no || undefined);
  setQueryParam('status', queryParams.status || undefined);
};

// 获取坯布列表（列表页 selector 数据源）
const getGreigeFabrics = async () => {
  try {
    const res = await getGreigeFabricList({ page: 1, page_size: 1000 });
    greigeFabrics.value = res.data?.items ?? [];
  } catch (error) {
    logger.error(t('dyeBatch.index.messageFetchProductsFailed'), error);
  }
};

// 查询
const handleQuery = () => {
  syncQueryParams();
  page.value = 1;
  refresh();
};

// 重置
const handleReset = () => {
  queryParams.color_no = '';
  queryParams.batch_no = '';
  queryParams.dye_lot_no = '';
  queryParams.status = '';
  syncQueryParams();
  page.value = 1;
  refresh();
};

// 新建
const handleCreate = () => {
  dialogTitle.value = t('dyeBatch.index.titleCreate');
  isView.value = false;
  Object.assign(formData, {
    id: undefined,
    batch_no: '',
    greige_fabric_id: undefined,
    color_no: '',
    color_code: '',
    dye_date: '',
    planned_quantity: 0,
    remarks: '',
  });
  dialogVisible.value = true;
};

// 查看（v14 P0-3 修复：实现只读查看功能，原 handler 为空导致业务失效）
const handleView = (row: DyeBatch) => {
  dialogTitle.value = t('dyeBatch.index.titleView');
  isView.value = true;
  Object.assign(formData, row);
  dialogVisible.value = true;
};

// 编辑
const handleEdit = (row: DyeBatch) => {
  dialogTitle.value = t('dyeBatch.index.titleEdit');
  isView.value = false;
  Object.assign(formData, row);
  dialogVisible.value = true;
};

// 完成
const handleComplete = async (row: DyeBatch) => {
  try {
    await ElMessageBox.confirm(
      t('dyeBatch.index.messageConfirmComplete'),
      t('dyeBatch.index.titlePrompt'),
      { type: 'warning' }
    );
    await completeDyeBatch(row.id);
    ElMessage.success(t('dyeBatch.index.messageOperationSuccess'));
    refresh();
  } catch (error) {
    logger.error(t('dyeBatch.index.messageOperationFailed'), error);
  }
};

// 删除
const handleDelete = async (row: DyeBatch) => {
  try {
    await ElMessageBox.confirm(
      t('dyeBatch.index.messageConfirmDelete'),
      t('dyeBatch.index.titlePrompt'),
      { type: 'warning' }
    );
    await deleteDyeBatch(row.id);
    ElMessage.success(t('dyeBatch.index.messageDeleteSuccess'));
    refresh();
  } catch (error) {
    logger.error(t('dyeBatch.index.messageDeleteFailed'), error);
  }
};

// 导出
const handleExport = async () => {
  try {
    const res = await exportDyeBatches(queryParams);
    const url = window.URL.createObjectURL(new Blob([res]));
    const link = document.createElement('a');
    link.href = url;
    link.setAttribute('download', t('dyeBatch.index.exportFileName'));
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    window.URL.revokeObjectURL(url);
    ElMessage.success(t('dyeBatch.index.messageExportSuccess'));
  } catch (error) {
    logger.error(t('dyeBatch.index.messageExportFailed'), error);
  }
};

// 提交表单
const handleSubmitForm = async () => {
  try {
    await formRef.value?.validate();
    if (formData.id) {
      await updateDyeBatch(formData.id, formData);
    } else {
      await createDyeBatch(formData);
    }
    ElMessage.success(t('dyeBatch.index.messageSaveSuccess'));
    dialogVisible.value = false;
    refresh();
  } catch (error) {
    logger.error(t('dyeBatch.index.messageFormValidateFailed'), error);
  }
};

// 分页（useTableApi 自动 watch page/pageSize 变化触发重载）
const handleSizeChange = (val: number) => {
  pageSize.value = val;
  page.value = 1;
};

const handleCurrentChange = (val: number) => {
  page.value = val;
};

// 状态标签/配色：词表取自后端 dye_batch_lifecycle_status（16 态小写）。
// dye_batch.status 是可空列，null 为合法缺省（无标签、默认配色）；
// 携带词表外取值属脏数据，由 normalizeDyeBatchStatus 记错误日志并抛错，不再回退掩盖。
const getStatusType = (status: string | null): string =>
  status === null ? 'info' : dyeBatchStatusTagType(normalizeDyeBatchStatus(status));

const getStatusLabel = (status: string | null): string =>
  status === null ? '' : t(dyeBatchStatusLabelKey(normalizeDyeBatchStatus(status)));

// 状态筛选下拉文案（入参恒为词表内的合法状态值）
const statusLabelFor = (status: DyeBatchLifecycleStatus): string =>
  t(dyeBatchStatusLabelKey(status));

// 备布之前的可编辑/可删除态：pending_schedule / scheduled
// （与后端 delete_dye_batch 的"生产中不可删"守卫集合互补）
const PRE_PRODUCTION_STATUSES: readonly string[] = [
  DYE_BATCH_LIFECYCLE_STATUS.PENDING_SCHEDULE,
  DYE_BATCH_LIFECYCLE_STATUS.SCHEDULED,
];
const canEdit = (row: DyeBatch): boolean => PRE_PRODUCTION_STATUSES.includes(row.status ?? '');
const canDelete = (row: DyeBatch): boolean => PRE_PRODUCTION_STATUSES.includes(row.status ?? '');
// "完成"对应后端 complete_dye_batch（inspecting → stored），仅此态可点
const canComplete = (row: DyeBatch): boolean =>
  row.status === DYE_BATCH_LIFECYCLE_STATUS.INSPECTING;

// started_at 为时间戳列，列表仅展示日期部分
const formatDate = (value: string | null): string => (value ? value.slice(0, 10) : '');

const hasLoaded = createLazyLoader();

// 批次 271：useTableApi 构造时自动初始加载，无需 onMounted 调用 getList
onMounted(() => {
  loadIfNot('greigeFabrics', getGreigeFabrics, hasLoaded);
});
</script>

<style scoped>
.dye-batch-page {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.header-left {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.page-title {
  margin: 0;
  font-size: 24px;
  font-weight: 600;
}

.header-actions {
  display: flex;
  gap: 10px;
}

.filter-card {
  margin-bottom: 20px;
}

.filter-form {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.table-card {
  margin-bottom: 20px;
}

.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 20px;
}
</style>

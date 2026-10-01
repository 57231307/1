<!--
  CostCollectionTab.vue - 成本归集 Tab
  来源：原 cost/index.vue 主体内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="cost-collection-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('cost.collectionList.title') }}</h2>
      <div>
        <el-button type="primary" @click="openDialog()">
          <el-icon><Plus /></el-icon>{{ t('cost.collectionList.button.create') }}
        </el-button>
        <el-button v-permission="'cost.export'" @click="handleExport">
          <el-icon><Download /></el-icon>{{ t('cost.collectionList.button.exportBtn') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form
        :inline="true"
        :model="queryForm"
        :aria-label="t('cost.collectionList.filter.ariaLabel')"
      >
        <el-form-item :label="t('cost.collectionList.filter.collectionNo')">
          <el-input
            v-model="queryForm.collection_no"
            :placeholder="t('cost.collectionList.filter.collectionNoPlaceholder')"
            clearable
          />
        </el-form-item>
        <el-form-item :label="t('cost.collectionList.filter.batchNo')">
          <el-input
            v-model="queryForm.batch_no"
            :placeholder="t('cost.collectionList.filter.batchNoPlaceholder')"
            clearable
          />
        </el-form-item>
        <el-form-item :label="t('cost.collectionList.filter.status')">
          <el-select
            v-model="queryForm.status"
            :placeholder="t('cost.collectionList.filter.statusPlaceholder')"
            clearable
          >
            <el-option :label="t('cost.collectionList.status.draft')" value="draft" />
            <el-option :label="t('cost.collectionList.status.pending')" value="pending" />
            <el-option :label="t('cost.collectionList.status.approved')" value="approved" />
            <el-option :label="t('cost.collectionList.status.rejected')" value="rejected" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleSearch">{{
            t('cost.collectionList.button.query')
          }}</el-button>
          <el-button @click="handleReset">{{ t('cost.collectionList.button.reset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover">
      <el-table
        v-loading="loading"
        :data="collectionList"
        stripe
        :aria-label="t('cost.collectionList.table.ariaLabel')"
      >
        <el-table-column
          prop="collection_no"
          :label="t('cost.collectionList.table.collectionNo')"
          width="140"
        />
        <el-table-column
          prop="collection_date"
          :label="t('cost.collectionList.table.collectionDate')"
          width="120"
        />
        <el-table-column
          prop="batch_no"
          :label="t('cost.collectionList.table.batchNo')"
          width="120"
        />
        <el-table-column
          prop="color_no"
          :label="t('cost.collectionList.table.colorNo')"
          width="100"
        />
        <el-table-column
          :label="t('cost.collectionList.table.directMaterial')"
          width="120"
          align="right"
        >
          <template #default="{ row }">¥{{ Number(row.direct_material ?? 0).toFixed(2) }}</template>
        </el-table-column>
        <el-table-column
          :label="t('cost.collectionList.table.directLabor')"
          width="120"
          align="right"
        >
          <template #default="{ row }">¥{{ Number(row.direct_labor ?? 0).toFixed(2) }}</template>
        </el-table-column>
        <el-table-column
          :label="t('cost.collectionList.table.manufacturingOverhead')"
          width="120"
          align="right"
        >
          <template #default="{ row }"
            >¥{{ Number(row.manufacturing_overhead ?? 0).toFixed(2) }}</template
          >
        </el-table-column>
        <el-table-column
          :label="t('cost.collectionList.table.totalCost')"
          width="120"
          align="right"
        >
          <template #default="{ row }">
            <span class="text-bold">¥{{ Number(row.total_cost ?? 0).toFixed(2) }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="status" :label="t('cost.collectionList.table.status')" width="100">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="remark"
          :label="t('cost.collectionList.table.remark')"
          min-width="150"
          show-overflow-tooltip
        />
        <el-table-column
          :label="t('cost.collectionList.table.operation')"
          width="180"
          fixed="right"
        >
          <template #default="{ row }">
            <el-button
              v-permission="'cost_collection:update'"
              type="primary"
              link
              size="small"
              @click="openDialog(row)"
              >{{ t('cost.collectionList.button.edit') }}</el-button
            >
            <el-button
              v-if="row.status === 'draft' || row.status === 'pending'"
              v-permission="'cost_collection:approve'"
              type="success"
              link
              size="small"
              @click="auditCollection(row, true)"
              >{{ t('cost.collectionList.button.audit') }}</el-button
            >
            <el-button
              v-if="row.status === 'pending'"
              v-permission="'cost_collection:approve'"
              type="warning"
              link
              size="small"
              @click="auditCollection(row, false)"
              >{{ t('cost.collectionList.button.reject') }}</el-button
            >
            <el-button
              v-permission="'cost_collection:delete'"
              type="danger"
              link
              size="small"
              @click="handleDelete(row)"
              >{{ t('cost.collectionList.button.delete') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          :aria-label="t('cost.collectionList.table.paginationAriaLabel')"
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
        />
      </div>
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="
        form.id
          ? t('cost.collectionList.dialog.editTitle')
          : t('cost.collectionList.dialog.createTitle')
      "
      width="600px"
      :aria-label="t('cost.collectionList.dialog.ariaLabel')"
    >
      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        label-width="100px"
        :aria-label="t('cost.collectionList.dialog.formAriaLabel')"
      >
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item
              :label="t('cost.collectionList.dialog.collectionDate')"
              prop="collection_date"
            >
              <el-date-picker
                v-model="form.collection_date"
                type="date"
                :placeholder="t('cost.collectionList.dialog.collectionDatePlaceholder')"
                value-format="YYYY-MM-DD"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('cost.collectionList.dialog.batchNo')">
              <el-input
                v-model="form.batch_no"
                :placeholder="t('cost.collectionList.dialog.batchNoPlaceholder')"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('cost.collectionList.dialog.colorNo')">
              <el-input
                v-model="form.color_no"
                :placeholder="t('cost.collectionList.dialog.colorNoPlaceholder')"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('cost.collectionList.dialog.period')">
              <el-input
                v-model="form.period"
                :placeholder="t('cost.collectionList.dialog.periodPlaceholder')"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item
              :label="t('cost.collectionList.dialog.directMaterial')"
              prop="direct_material"
            >
              <el-input-number
                v-model="form.direct_material"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('cost.collectionList.dialog.directLabor')" prop="direct_labor">
              <el-input-number
                v-model="form.direct_labor"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item
              :label="t('cost.collectionList.dialog.manufacturingOverhead')"
              prop="manufacturing_overhead"
            >
              <el-input-number
                v-model="form.manufacturing_overhead"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('cost.collectionList.dialog.totalCost')">
              <span class="text-bold">¥{{ totalCost.toFixed(2) }}</span>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <!-- 后端创建 DTO 必填（CreateCostCollectionInput.processing_fee/dyeing_fee，
               NOT NULL 列且参与 total_cost 求和），表单必须采集 -->
          <el-col :span="12">
            <el-form-item
              :label="t('cost.collectionList.dialog.processingFee')"
              prop="processing_fee"
            >
              <el-input-number
                v-model="form.processing_fee"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('cost.collectionList.dialog.dyeingFee')" prop="dyeing_fee">
              <el-input-number
                v-model="form.dyeing_fee"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item :label="t('cost.collectionList.dialog.remark')">
          <el-input v-model="form.remark" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{
          t('cost.collectionList.button.cancel')
        }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
          t('cost.collectionList.button.confirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';
import { Plus, Download } from '@element-plus/icons-vue';
import {
  createCostCollection,
  updateCostCollection,
  deleteCollection as deleteCollectionApi,
  auditCollection as auditCollectionApi,
  COST_STATUS,
  type CostCollection,
} from '@/api/cost';
import { logger } from '@/utils/logger';
import { exportFromBackend } from '@/utils/export';
// 批次 278：迁移到 useTableApi composable，自动管理分页与 loading
import { useTableApi } from '@/composables/useTableApi';

// 批次 34 v9 P1：接入 i18n，替换硬编码中文 ElMessage
const { t } = useI18n({ useScope: 'global' });

const submitLoading = ref(false);
const dialogVisible = ref(false);
const formRef = ref<FormInstance>();

// 批次 278：筛选条件（仅保留业务字段，page/page_size 由 useTableApi 管理）
const queryForm = reactive({
  collection_no: '',
  batch_no: '',
  status: '',
});

// 批次 278：使用 useTableApi 管理成本归集列表分页
const {
  data: collectionList,
  total,
  loading,
  page,
  pageSize,
  queryParams,
  setQueryParam,
  refresh: fetchCollections,
} = useTableApi<CostCollection>({
  url: '/production/cost-collections',
  defaultPageSize: 20,
  onError: (err: unknown) => {
    const msg =
      err instanceof Error
        ? err.message || t('cost.collectionList.message.loadListFailed')
        : t('cost.collectionList.message.loadListFailed');
    ElMessage.error(msg);
  },
});

// 批次 278：将筛选字段同步到 queryParams
const syncQueryParams = () => {
  setQueryParam('collection_no', queryForm.collection_no);
  setQueryParam('batch_no', queryForm.batch_no);
  setQueryParam('status', queryForm.status);
};

// 批次 278：分页变化处理函数
const handlePageChange = (_p: number) => {
  // useTableApi 内部 watch page 自动触发刷新
};
const handleSizeChange = (_s: number) => {
  // useTableApi 内部 watch pageSize 自动触发刷新
  page.value = 1;
};

// 表单模型 = 归集单编辑所需实体字段 + 创建必填的两项费用。
// processing_fee/dyeing_fee 为后端 CreateCostCollectionRequestDto 非 Option 必填
// （见 api/cost.ts::CreateCostCollectionInput），此前表单未采集 ⇒ 创建请求体缺键、后端 400。
interface CollectionFormModel {
  id?: number;
  collection_date: string;
  batch_no?: string;
  color_no?: string;
  period?: string;
  direct_material: number;
  direct_labor: number;
  manufacturing_overhead: number;
  processing_fee: number;
  dyeing_fee: number;
  remark?: string;
}

const form = reactive<CollectionFormModel>({
  id: undefined,
  collection_date: new Date().toISOString().split('T')[0],
  batch_no: '',
  color_no: '',
  period: new Date().toISOString().slice(0, 7),
  direct_material: 0,
  direct_labor: 0,
  manufacturing_overhead: 0,
  processing_fee: 0,
  dyeing_fee: 0,
  remark: '',
});

const rules: FormRules = {
  collection_date: [
    { required: true, message: t('cost.validation.collectionDateRequired'), trigger: 'change' },
  ],
  direct_material: [
    { required: true, message: t('cost.validation.directMaterialRequired'), trigger: 'blur' },
  ],
  direct_labor: [
    { required: true, message: t('cost.validation.directLaborRequired'), trigger: 'blur' },
  ],
  manufacturing_overhead: [
    {
      required: true,
      message: t('cost.validation.manufacturingOverheadRequired'),
      trigger: 'blur',
    },
  ],
  // 后端创建 DTO 必填（CreateCostCollectionInput）
  processing_fee: [
    { required: true, message: t('cost.validation.processingFeeRequired'), trigger: 'blur' },
  ],
  dyeing_fee: [
    { required: true, message: t('cost.validation.dyeingFeeRequired'), trigger: 'blur' },
  ],
};

// 与后端 total_cost 重算口径逐项一致：五项费用求和
// （create：cost_collection_service.rs:82-86；update：:265-272）。
// form 金额恒为 number（初始化/el-input-number/openDialog 归一三路保证），直接 + 求和；
// 原先的 `|| 0` 会把字符串金额放行（"100.50"||0 仍是非空串）导致 + 变字符串拼接——归一后不再需要。
const totalCost = computed(
  () =>
    form.direct_material +
    form.direct_labor +
    form.manufacturing_overhead +
    form.processing_fee +
    form.dyeing_fee
);

const getStatusLabel = (status: string) => t(`cost.collectionList.status.${status}`);

const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    [COST_STATUS.DRAFT]: 'info',
    [COST_STATUS.PENDING]: 'warning',
    [COST_STATUS.APPROVED]: 'success',
    [COST_STATUS.REJECTED]: 'danger',
  };
  return map[status] || 'info';
};

const handleSearch = () => {
  // 批次 278：同步筛选条件并重置到第一页
  syncQueryParams();
  page.value = 1;
  fetchCollections();
};

const handleReset = () => {
  queryForm.collection_no = '';
  queryForm.batch_no = '';
  queryForm.status = '';
  handleSearch();
};

const openDialog = (row?: CostCollection) => {
  formRef.value?.resetFields();
  if (row) {
    // 后端 rust_decimal 仅 serde 特性 → 金额序列化为十进制字符串（models/cost_collection.rs:24-30，
    // 本文件列表列 :93 的 Number() 归一即佐证）。金额直灌进 form 会让 totalCost 的 + 变字符串拼接、
    // el-input-number 收到 string，故进 form 即逐键归一为 number；
    // 词表外脏值 Number() 得 NaN 会显式暴露，不静默吞。
    // 提交方向：后端 Decimal 反序列化 string/number 均可（service CreateCostCollectionRequest:48-52
    // 为非 Option Decimal 直收），按表单模型送 number。
    Object.assign(form, row);
    const amountKeys = [
      'direct_material',
      'direct_labor',
      'manufacturing_overhead',
      'processing_fee',
      'dyeing_fee',
    ] as const;
    amountKeys.forEach(key => {
      form[key] = Number(form[key]);
    });
  } else {
    form.id = undefined;
    form.collection_date = new Date().toISOString().split('T')[0];
    form.batch_no = '';
    form.color_no = '';
    form.period = new Date().toISOString().slice(0, 7);
    form.direct_material = 0;
    form.direct_labor = 0;
    form.manufacturing_overhead = 0;
    form.processing_fee = 0;
    form.dyeing_fee = 0;
    form.remark = '';
  }
  dialogVisible.value = true;
};

const handleSubmit = async () => {
  if (!formRef.value) return;
  await formRef.value.validate(async valid => {
    if (!valid) return;
    submitLoading.value = true;
    try {
      if (form.id) {
        // UpdateCostCollectionRequest（cost_collection_service.rs:21-32）无 total_cost 键——
        // total_cost 由服务端按五项求和重算（:265-272），前端不代劳、不提交将被 serde 丢弃的键值
        const data: Partial<CostCollection> = { ...form };
        await updateCostCollection(form.id, data);
        ElMessage.success(t('message.updateSuccess'));
      } else {
        // 创建载荷按后端 DTO 逐键构造（api/cost.ts::CreateCostCollectionInput）：
        // id/period/remark/total_cost 等非 CreateCostCollectionRequestDto 键不提交，
        // total_cost 由服务端五项求和计算（cost_collection_service.rs:82-86），前端不代劳。
        await createCostCollection({
          collection_date: form.collection_date,
          batch_no: form.batch_no,
          color_no: form.color_no,
          direct_material: form.direct_material,
          direct_labor: form.direct_labor,
          manufacturing_overhead: form.manufacturing_overhead,
          processing_fee: form.processing_fee,
          dyeing_fee: form.dyeing_fee,
        });
        ElMessage.success(t('message.createSuccess'));
      }
      dialogVisible.value = false;
      fetchCollections();
    } catch (e) {
      const err = e as Error;
      ElMessage.error(err.message || t('cost.collectionList.message.operationFailed'));
    } finally {
      submitLoading.value = false;
    }
  });
};

const handleDelete = async (row: CostCollection) => {
  if (!row.id) return;
  try {
    await ElMessageBox.confirm(
      t('cost.confirmDelete', { name: row.collection_no }),
      t('message.deleteConfirmTitle'),
      {
        type: 'warning',
      }
    );
    await deleteCollectionApi(row.id);
    ElMessage.success(t('message.deleteSuccess'));
    fetchCollections();
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error;
      ElMessage.error(err.message || t('cost.collectionList.message.deleteFailed'));
    }
  }
};

const auditCollection = async (row: CostCollection, approved: boolean) => {
  if (!row.id) return;
  try {
    const text = approved
      ? t('cost.collectionList.message.auditPassed')
      : t('cost.collectionList.message.auditRejected');
    await ElMessageBox.confirm(
      t('cost.confirmAction', { action: text }),
      t('cost.actionConfirmTitle', { action: text }),
      { type: 'info' }
    );
    await auditCollectionApi(row.id, approved);
    ElMessage.success(t('cost.collectionList.message.auditSuccess', { action: text }));
    fetchCollections();
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error;
      ElMessage.error(err.message || t('cost.collectionList.message.operationFailed'));
    }
  }
};

// V15 P0-S12 修复（Batch 475e）：迁移到后端导出，注入水印 + 审计日志
const handleExport = async () => {
  const params: Record<string, unknown> = {
    batch_no: queryParams.value.batch_no as string | undefined,
    color_no: queryParams.value.color_no as string | undefined,
  };
  await exportFromBackend('/production/cost-collections/export', params, 'cost_collections_export');
  logger.info(t('cost.collectionList.message.exportSuccess'));
};
</script>

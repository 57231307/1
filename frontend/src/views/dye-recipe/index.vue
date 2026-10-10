<template>
  <div class="dye-recipe-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">{{ t('dyeRecipe.index.pageTitle') }}</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">{{
            t('dyeRecipe.index.breadcrumbHome')
          }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ t('dyeRecipe.index.breadcrumbFabric') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ t('dyeRecipe.index.breadcrumbDyeRecipe') }}</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          {{ t('dyeRecipe.index.buttonCreate') }}
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          {{ t('dyeRecipe.index.buttonExport') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form
        :inline="true"
        :model="queryParams"
        class="filter-form"
        :aria-label="t('dyeRecipe.index.ariaFilterForm')"
      >
        <el-form-item :label="t('dyeRecipe.index.filterRecipeNo')">
          <el-input
            v-model="queryParams.recipe_no"
            :placeholder="t('dyeRecipe.index.placeholderRecipeNo')"
            clearable
            @clear="handleQuery"
          />
        </el-form-item>
        <el-form-item :label="t('dyeRecipe.index.filterColorNo')">
          <el-input
            v-model="queryParams.color_code"
            :placeholder="t('dyeRecipe.index.placeholderColorNo')"
            clearable
            @clear="handleQuery"
          />
        </el-form-item>
        <el-form-item :label="t('dyeRecipe.index.filterStatus')">
          <el-select
            v-model="queryParams.status"
            :placeholder="t('dyeRecipe.index.placeholderStatus')"
            clearable
            @change="handleQuery"
          >
            <el-option :label="t('dyeRecipe.index.optionDraft')" :value="DYE_RECIPE_STATUS.DRAFT" />
            <el-option
              :label="t('dyeRecipe.index.optionPending')"
              :value="DYE_RECIPE_STATUS.PENDING_APPROVAL"
            />
            <el-option
              :label="t('dyeRecipe.index.optionApproved')"
              :value="DYE_RECIPE_STATUS.APPROVED"
            />
            <el-option
              :label="t('dyeRecipe.index.optionRejected')"
              :value="DYE_RECIPE_STATUS.REJECTED"
            />
            <el-option
              :label="t('dyeRecipe.index.optionInactive')"
              :value="DYE_RECIPE_STATUS.DISABLED"
            />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">
            <el-icon><Search /></el-icon>
            {{ t('dyeRecipe.index.buttonQuery') }}
          </el-button>
          <el-button @click="handleReset">
            <el-icon><Refresh /></el-icon>
            {{ t('dyeRecipe.index.buttonReset') }}
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table
        v-loading="loading"
        :data="recipeList"
        border
        stripe
        :aria-label="t('dyeRecipe.index.ariaTable')"
      >
        <el-table-column
          type="index"
          :label="t('dyeRecipe.index.colIndex')"
          width="60"
          align="center"
        />
        <el-table-column
          prop="recipe_no"
          :label="t('dyeRecipe.index.colRecipeNo')"
          width="120"
          show-overflow-tooltip
        />
        <el-table-column
          prop="recipe_name"
          :label="t('dyeRecipe.index.colRecipeName')"
          min-width="150"
          show-overflow-tooltip
        />
        <el-table-column
          prop="color_no"
          :label="t('dyeRecipe.index.colColorNo')"
          width="100"
          show-overflow-tooltip
        />
        <el-table-column
          prop="color_name"
          :label="t('dyeRecipe.index.colColorName')"
          width="120"
          show-overflow-tooltip
        />
        <el-table-column
          prop="version"
          :label="t('dyeRecipe.index.colVersion')"
          width="80"
          align="center"
        />
        <el-table-column
          prop="status"
          :label="t('dyeRecipe.index.colStatus')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="created_at"
          :label="t('dyeRecipe.index.colCreatedAt')"
          width="180"
          align="center"
        />
        <el-table-column
          :label="t('dyeRecipe.index.colOperation')"
          width="250"
          align="center"
          fixed="right"
        >
          <template #default="{ row }">
            <!-- v11 批次 168 P2-1 修复：row as any 改为 row as DyeRecipe -->
            <el-button type="primary" link size="small" @click="handleView(row as DyeRecipe)">{{
              t('dyeRecipe.index.buttonView')
            }}</el-button>
            <el-button
              v-if="row.status === DYE_RECIPE_STATUS.DRAFT"
              type="primary"
              link
              size="small"
              @click="handleEdit(row as DyeRecipe)"
              >{{ t('dyeRecipe.index.buttonEdit') }}</el-button
            >
            <el-button
              v-if="row.status === DYE_RECIPE_STATUS.DRAFT"
              type="success"
              link
              size="small"
              @click="handleSubmit(row as DyeRecipe)"
              >{{ t('dyeRecipe.index.buttonSubmit') }}</el-button
            >
            <el-button
              v-if="row.status === DYE_RECIPE_STATUS.DRAFT"
              type="danger"
              link
              size="small"
              @click="handleDelete(row as DyeRecipe)"
              >{{ t('common.delete') }}</el-button
            >
            <el-button
              v-if="canApprove((row as DyeRecipe).status)"
              type="success"
              link
              size="small"
              @click="handleApprove(row as DyeRecipe)"
              >{{ t('dyeRecipe.index.buttonApprove') }}</el-button
            >
            <!-- 拒绝仅对待审核开放：后端 service.reject 的状态门是 pending_approval→rejected，
                 草稿态按钮若可达只会换来一次业务拒绝 -->
            <el-button
              v-if="canReject((row as DyeRecipe).status)"
              type="danger"
              link
              size="small"
              @click="handleReject(row as DyeRecipe)"
              >{{ t('dyeRecipe.index.buttonReject') }}</el-button
            >
            <el-button type="info" link size="small" @click="handleVersion(row as DyeRecipe)">{{
              t('dyeRecipe.index.buttonVersion')
            }}</el-button>
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
          :aria-label="t('dyeRecipe.index.ariaPagination')"
          @size-change="handleSizeChange"
          @current-change="handleCurrentChange"
        />
      </div>
    </el-card>

    <!-- 新建/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="dialogTitle"
      width="800px"
      :close-on-click-modal="false"
      :aria-label="t('dyeRecipe.index.ariaEditDialog')"
    >
      <el-form
        ref="formRef"
        :model="formData"
        :rules="formRules"
        :disabled="isView"
        label-width="100px"
        :aria-label="t('dyeRecipe.index.ariaForm')"
      >
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('dyeRecipe.index.colRecipeNo')" prop="recipe_no">
              <el-input
                v-model="formData.recipe_no"
                :placeholder="t('dyeRecipe.index.placeholderRecipeNo')"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('dyeRecipe.index.colRecipeName')" prop="recipe_name">
              <el-input
                v-model="formData.recipe_name"
                :placeholder="t('dyeRecipe.index.placeholderRecipeName')"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('dyeRecipe.index.colColorNo')" prop="color_no">
              <el-input
                v-model="formData.color_no"
                :placeholder="t('dyeRecipe.index.placeholderColorNo')"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('dyeRecipe.index.colColorName')" prop="color_name">
              <el-input
                v-model="formData.color_name"
                :placeholder="t('dyeRecipe.index.placeholderColorName')"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item :label="t('dyeRecipe.index.labelContent')" prop="content">
          <el-input
            v-model="formData.content"
            type="textarea"
            :rows="10"
            :placeholder="t('dyeRecipe.index.placeholderContent')"
          />
        </el-form-item>
        <el-form-item :label="t('dyeRecipe.index.labelRemarks')" prop="remarks">
          <el-input
            v-model="formData.remarks"
            type="textarea"
            :rows="3"
            :placeholder="t('dyeRecipe.index.placeholderRemarks')"
          />
        </el-form-item>
        <!-- 拒绝理由是已裁定的审批结果，只在查看态只读展示；编辑态不出现该控件，
             避免把裁量文本重新纳入可提交载荷 -->
        <el-form-item
          v-if="isView && formData.rejected_reason"
          :label="t('dyeRecipe.index.labelRejectReason')"
        >
          <el-input :model-value="formData.rejected_reason" type="textarea" :rows="2" readonly />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{
          isView ? t('dyeRecipe.index.buttonClose') : t('dyeRecipe.index.buttonCancel')
        }}</el-button>
        <el-button v-if="!isView" type="primary" @click="handleSubmitForm">{{
          t('dyeRecipe.index.buttonConfirm')
        }}</el-button>
      </template>
    </el-dialog>

    <!-- 版本历史对话框 -->
    <el-dialog
      v-model="versionVisible"
      :title="t('dyeRecipe.index.titleVersionHistory')"
      width="800px"
      :aria-label="t('dyeRecipe.index.ariaVersionHistory')"
    >
      <el-table
        :data="versionList"
        border
        stripe
        :aria-label="t('dyeRecipe.index.ariaVersionList')"
      >
        <el-table-column
          prop="version"
          :label="t('dyeRecipe.index.colVersion')"
          width="80"
          align="center"
        />
        <el-table-column
          prop="recipe_name"
          :label="t('dyeRecipe.index.colRecipeName')"
          min-width="150"
          show-overflow-tooltip
        />
        <el-table-column
          prop="status"
          :label="t('dyeRecipe.index.colStatus')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="created_at"
          :label="t('dyeRecipe.index.colCreatedAt')"
          width="180"
          align="center"
        />
        <el-table-column :label="t('dyeRecipe.index.colOperation')" width="100" align="center">
          <template #default="{ row }">
            <el-button
              type="primary"
              link
              size="small"
              @click="handleViewVersion(row as DyeRecipe)"
              >{{ t('dyeRecipe.index.buttonView') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus, Download, Search, Refresh } from '@element-plus/icons-vue';
import {
  createDyeRecipe,
  updateDyeRecipe,
  approveDyeRecipe,
  rejectDyeRecipe,
  submitDyeRecipe,
  deleteDyeRecipe,
  getRecipeVersions,
  exportDyeRecipes,
} from '@/api/dye-recipe';
import { DYE_RECIPE_STATUS } from '@/api/dye-recipe';
import type { DyeRecipe, DyeRecipeStatus, DyeRecipeUpdatePayload } from '@/api/dye-recipe';
import { logger } from '@/utils/logger';
import { isDialogDismissal } from '@/utils/monitor';
import { useTableApi } from '@/composables/useTableApi';

const { t } = useI18n({ useScope: 'global' });

// 查询参数（仅保留后端 DyeRecipeListQuery 真实读取的筛选：配方编号、色号 color_code、状态；分页由 useTableApi 管理）
const queryParams = reactive({
  recipe_no: '',
  color_code: '',
  status: '',
});

// 批次 271：接入 useTableApi，消除手写 page/pageSize/total/loading + getList 重复
// useTableApi 自动管理分页状态、loading、数据加载，自动 watch page/pageSize 变化触发重载
const {
  data: recipeList,
  loading,
  page,
  pageSize,
  total,
  refresh,
  setQueryParam,
} = useTableApi<DyeRecipe>({
  url: '/production/dye-recipes',
  onError: (e: unknown) => logger.error(t('dyeRecipe.index.messageFetchFailed'), String(e)),
});

// 对话框
const dialogVisible = ref(false);
const dialogTitle = ref('');
const formRef = ref();
const isView = ref(false);

// 版本历史
const versionVisible = ref(false);
const versionList = ref<DyeRecipe[]>([]);

// 表单数据
const formData = reactive({
  id: undefined as number | undefined,
  recipe_no: '',
  recipe_name: '',
  color_no: '',
  color_name: '',
  content: '',
  remarks: '',
  rejected_reason: null as string | null,
});

// 表单验证规则
const formRules = {
  recipe_no: [
    { required: true, message: t('dyeRecipe.index.ruleRecipeNoRequired'), trigger: 'blur' },
  ],
  recipe_name: [
    { required: true, message: t('dyeRecipe.index.ruleRecipeNameRequired'), trigger: 'blur' },
  ],
  color_no: [
    { required: true, message: t('dyeRecipe.index.ruleColorNoRequired'), trigger: 'blur' },
  ],
  color_name: [
    { required: true, message: t('dyeRecipe.index.ruleColorNameRequired'), trigger: 'blur' },
  ],
  content: [{ required: true, message: t('dyeRecipe.index.ruleContentRequired'), trigger: 'blur' }],
};

// 批次 271：同步筛选条件到 useTableApi.queryParams 并刷新
// useTableApi 自动 watch page/pageSize 变化触发重载，无需手动 getList
const syncQueryParams = () => {
  setQueryParam('recipe_no', queryParams.recipe_no || undefined);
  setQueryParam('color_code', queryParams.color_code || undefined);
  setQueryParam('status', queryParams.status || undefined);
};

// 查询
const handleQuery = () => {
  syncQueryParams();
  page.value = 1;
  refresh();
};

// 重置
const handleReset = () => {
  queryParams.recipe_no = '';
  queryParams.color_code = '';
  queryParams.status = '';
  syncQueryParams();
  page.value = 1;
  refresh();
};

// 新建
const handleCreate = () => {
  dialogTitle.value = t('dyeRecipe.index.titleCreate');
  isView.value = false;
  Object.assign(formData, {
    id: undefined,
    recipe_no: '',
    recipe_name: '',
    color_no: '',
    color_name: '',
    content: '',
    remarks: '',
    rejected_reason: null,
  });
  dialogVisible.value = true;
};

// 查看（v14 P0-3 修复：实现只读查看功能，原 handler 为空导致业务失效）
const handleView = (row: DyeRecipe) => {
  dialogTitle.value = t('dyeRecipe.index.titleView');
  isView.value = true;
  // 正文回显：后端出参键为 chemical_formula，对话框绑定 content——不显式映射则编辑/查看正文恒空
  Object.assign(formData, row, { content: row.chemical_formula ?? '' });
  dialogVisible.value = true;
};

// 编辑
const handleEdit = (row: DyeRecipe) => {
  dialogTitle.value = t('dyeRecipe.index.titleEdit');
  isView.value = false;
  Object.assign(formData, row, { content: row.chemical_formula ?? '' });
  dialogVisible.value = true;
};

// 提交审批
const handleSubmit = async (row: DyeRecipe) => {
  try {
    await ElMessageBox.confirm(
      t('dyeRecipe.index.messageConfirmSubmit'),
      t('dyeRecipe.index.titlePrompt'),
      { type: 'warning' }
    );
    await submitDyeRecipe(row.id);
    ElMessage.success(t('dyeRecipe.index.messageSubmitSuccess'));
    refresh();
  } catch (error) {
    logger.error(t('dyeRecipe.index.messageSubmitFailed'), error);
  }
};

// 删除（草稿态可删、与编辑提交同状态门、非草稿不给删除入口、后端 service 另有状态门时前端保持 DRAFT 门不放宽）
const handleDelete = async (row: DyeRecipe) => {
  try {
    await ElMessageBox.confirm(
      '删除后该配方不可恢复，确认删除吗？',
      t('dyeRecipe.index.titlePrompt'),
      {
        type: 'warning',
      }
    );
    await deleteDyeRecipe(row.id);
    ElMessage.success(t('common.message.deleteSuccess'));
    refresh();
  } catch (error) {
    if (!isDialogDismissal(error)) {
      logger.error(t('common.message.operationFailed'), error);
    }
  }
};

// 审批
const handleApprove = async (row: DyeRecipe) => {
  try {
    await ElMessageBox.confirm(
      t('dyeRecipe.index.messageConfirmApprove'),
      t('dyeRecipe.index.titlePrompt'),
      { type: 'warning' }
    );
    // 审批人身份由后端按会话（AuthContext.user_id）派生，请求体不承载；
    // 前端不得再取登录 ID 组装载荷或对缺失身份做前置锁死。
    await approveDyeRecipe(row.id);
    ElMessage.success(t('dyeRecipe.index.messageApproveSuccess'));
    refresh();
  } catch (error) {
    logger.error(t('dyeRecipe.index.messageApproveFailed'), error);
  }
};

// 审批拒绝：理由经输入校验非空后提交，操作人身份由后端按会话派生，前端不承载
const handleReject = async (row: DyeRecipe) => {
  try {
    const { value } = await ElMessageBox.prompt(
      t('dyeRecipe.index.rejectReasonPrompt'),
      t('dyeRecipe.index.titlePrompt'),
      {
        inputValidator: (input: string) =>
          (input ?? '').trim().length > 0 || t('dyeRecipe.index.rejectReasonRequired'),
      }
    );
    await rejectDyeRecipe(row.id, value.trim());
    ElMessage.success(t('dyeRecipe.index.rejectSuccess'));
    refresh();
  } catch (error) {
    if (!isDialogDismissal(error)) {
      logger.error(t('dyeRecipe.index.rejectFailed'), error);
    }
  }
};

// 版本历史
const handleVersion = async (row: DyeRecipe) => {
  try {
    const res = await getRecipeVersions(row.id);
    versionList.value = res.data || [];
    versionVisible.value = true;
  } catch (error) {
    logger.error(t('dyeRecipe.index.messageVersionFailed'), error);
  }
};

// 查看版本（v14 中风险修复：实现版本详情查看逻辑，原 handler 为空导致功能失效）
const handleViewVersion = (row: DyeRecipe) => {
  versionVisible.value = false;
  dialogTitle.value = t('dyeRecipe.index.titleViewVersion', { version: row.version });
  isView.value = true;
  Object.assign(formData, row, { content: row.chemical_formula ?? '' });
  dialogVisible.value = true;
};

// 导出
const handleExport = async () => {
  try {
    const res = await exportDyeRecipes(queryParams);
    const url = window.URL.createObjectURL(new Blob([res]));
    const link = document.createElement('a');
    link.href = url;
    link.setAttribute('download', t('dyeRecipe.index.exportFileName'));
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    window.URL.revokeObjectURL(url);
    ElMessage.success(t('dyeRecipe.index.messageExportSuccess'));
  } catch (error) {
    logger.error(t('dyeRecipe.index.messageExportFailed'), error);
  }
};

// 提交表单
const handleSubmitForm = async () => {
  try {
    await formRef.value?.validate();
    // 后端 CreateDyeRecipeRequest 字段为 color_code（表单绑定 color_no），
    // 提交时映射，否则 serde 忽略 color_no → color_code=None → DB NOT NULL 违反 500。
    // 正文列后端键为 chemical_formula（表单绑定 content）——此前整表单直传时 content
    // 被 serde 丢弃、正文从未落库，现显式映射（契约修复，需联调验证编辑回写）。
    if (formData.id) {
      // 三态语义（后端 UpdateDyeRecipeRequest DoubleOption，RFC 7386）：
      // 对话框已回显原值；可空列 UI 清空 ⇒ 送显式 null（=清空为 NULL）、未改动 ⇒ 原值回传；
      // color_code 为 NOT NULL（m0003:30）恒送值（送 null 会被后端拒绝）；
      // 未采集键（fabric_type/dye_type/temperature/time_minutes/ph_value/
      // liquor_ratio/auxiliaries/status）省略 = 保持原值
      const updatePayload: DyeRecipeUpdatePayload = {
        color_no: formData.color_no || null,
        color_code: formData.color_no,
        color_name: formData.color_name || null,
        chemical_formula: formData.content || null,
        remarks: formData.remarks || null,
      };
      await updateDyeRecipe(formData.id, updatePayload);
    } else {
      const payload = {
        ...formData,
        color_code: formData.color_no,
        chemical_formula: formData.content || null,
      };
      await createDyeRecipe(payload);
    }
    ElMessage.success(t('dyeRecipe.index.messageSaveSuccess'));
    dialogVisible.value = false;
    refresh();
  } catch (error) {
    logger.error(t('dyeRecipe.index.messageFormValidateFailed'), error);
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

// 状态颜色以闭合词表为键：Record<DyeRecipeStatus, …> 由 TS 强制穷举，
// 因此不再需要 `|| 'info'` 兜底，也不允许再塞中文或大写历史死键
// （迁移前该列同时存在 中文 / 大写 两套取值，词表收口后都已不可达）。
const getStatusType = (status: DyeRecipeStatus): 'info' | 'warning' | 'success' | 'danger' => {
  const map: Record<DyeRecipeStatus, 'info' | 'warning' | 'success' | 'danger'> = {
    [DYE_RECIPE_STATUS.DRAFT]: 'info',
    [DYE_RECIPE_STATUS.PENDING_APPROVAL]: 'warning',
    [DYE_RECIPE_STATUS.APPROVED]: 'success',
    [DYE_RECIPE_STATUS.REJECTED]: 'danger',
    [DYE_RECIPE_STATUS.DISABLED]: 'danger',
  };
  return map[status];
};

const getStatusLabel = (status: DyeRecipeStatus): string => {
  const map: Record<DyeRecipeStatus, string> = {
    [DYE_RECIPE_STATUS.DRAFT]: t('dyeRecipe.index.optionDraft'),
    [DYE_RECIPE_STATUS.PENDING_APPROVAL]: t('dyeRecipe.index.optionPending'),
    [DYE_RECIPE_STATUS.APPROVED]: t('dyeRecipe.index.optionApproved'),
    [DYE_RECIPE_STATUS.REJECTED]: t('dyeRecipe.index.optionRejected'),
    [DYE_RECIPE_STATUS.DISABLED]: t('dyeRecipe.index.optionInactive'),
  };
  return map[status];
};

// 审批门槛与后端 validate_can_approve 同源：草稿或待审核均可审批
const canApprove = (status: DyeRecipeStatus) =>
  status === DYE_RECIPE_STATUS.DRAFT || status === DYE_RECIPE_STATUS.PENDING_APPROVAL;

// 拒绝门槛与后端 validate_can_reject 同源：仅待审核可拒（草稿态后端会直接业务拒绝）
const canReject = (status: DyeRecipeStatus) => status === DYE_RECIPE_STATUS.PENDING_APPROVAL;

// 批次 271：useTableApi 构造时自动初始加载，无需 onMounted 调用 getList
</script>

<style scoped>
.dye-recipe-page {
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

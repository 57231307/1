<!--
  report-templates/index.vue - 报表中心
  D05 Batch 3：接入 useI18n，所有硬编码中文迁移到 locales/zh-CN.ts + en-US.ts
-->
<template>
  <div class="report-templates-page">
    <div class="page-header">
      <h2 class="page-title">{{ $t('reportTemplates.title') }}</h2>
      <div class="header-actions">
        <el-button type="primary" @click="openDialog()">
          <el-icon><Plus /></el-icon>
          {{ $t('reportTemplates.create') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover">
      <div class="filter-container">
        <el-input
          v-model="queryParams.keyword"
          :placeholder="$t('reportTemplates.searchPlaceholder')"
          style="width: 200px"
          clearable
          @clear="handleSearch"
          @keyup.enter="handleSearch"
        />
        <el-select
          v-model="queryParams.report_type"
          :placeholder="$t('reportTemplates.category.placeholder')"
          clearable
          style="width: 120px"
        >
          <el-option :label="$t('reportTemplates.category.sales')" value="sales" />
          <el-option :label="$t('reportTemplates.category.inventory')" value="inventory" />
          <el-option :label="$t('reportTemplates.category.finance')" value="finance" />
          <el-option :label="$t('reportTemplates.category.production')" value="production" />
          <el-option :label="$t('reportTemplates.category.custom')" value="custom" />
        </el-select>
        <el-select
          v-model="queryParams.status"
          :placeholder="$t('reportTemplates.status.placeholder')"
          clearable
          style="width: 120px"
        >
          <!-- 状态词表写入方为大写 ACTIVE/INACTIVE（report_template_service.rs），比较值须逐字符一致 -->
          <el-option :label="$t('reportTemplates.status.active')" value="ACTIVE" />
          <el-option :label="$t('reportTemplates.status.inactive')" value="INACTIVE" />
        </el-select>
        <el-button type="primary" @click="handleSearch">
          <el-icon><Search /></el-icon>
          {{ $t('reportTemplates.search') }}
        </el-button>
      </div>

      <el-table
        v-loading="loading"
        :data="list"
        stripe
        :aria-label="$t('reportTemplates.table.ariaLabel')"
      >
        <el-table-column
          prop="code"
          :label="$t('reportTemplates.table.templateCode')"
          width="140"
        />
        <el-table-column
          prop="name"
          :label="$t('reportTemplates.table.templateName')"
          min-width="180"
        />
        <el-table-column
          prop="report_type"
          :label="$t('reportTemplates.table.category')"
          width="100"
        >
          <template #default="{ row }">
            {{ getCategoryLabel(row.report_type) }}
          </template>
        </el-table-column>
        <el-table-column
          prop="status"
          :label="$t('reportTemplates.table.status')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="row.status === 'ACTIVE' ? 'success' : 'info'" size="small">
              {{
                row.status === 'ACTIVE'
                  ? $t('reportTemplates.status.active')
                  : $t('reportTemplates.status.inactive')
              }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="created_at"
          :label="$t('reportTemplates.table.createdAt')"
          width="160"
        />
        <el-table-column :label="$t('reportTemplates.table.operation')" width="250" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handlePreview(row)">{{
              $t('reportTemplates.table.preview')
            }}</el-button>
            <el-button type="primary" link size="small" @click="handleGenerate(row)">{{
              $t('reportTemplates.table.generate')
            }}</el-button>
            <el-button type="primary" link size="small" @click="openDialog(row)">{{
              $t('reportTemplates.table.edit')
            }}</el-button>
            <el-button type="danger" link size="small" @click="handleDelete(row)">{{
              $t('reportTemplates.table.delete')
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
          :aria-label="$t('reportTemplates.paginationAriaLabel')"
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
        />
      </div>
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="
        form.id ? $t('reportTemplates.dialog.editTitle') : $t('reportTemplates.dialog.createTitle')
      "
      width="800px"
      :aria-label="$t('reportTemplates.dialog.ariaLabel')"
    >
      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        label-width="100px"
        :aria-label="$t('reportTemplates.dialog.formAriaLabel')"
      >
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('reportTemplates.dialog.templateCode')" prop="code">
              <el-input
                v-model="form.code"
                :disabled="!!form.id"
                :placeholder="$t('reportTemplates.dialog.templateCodePlaceholder')"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="$t('reportTemplates.dialog.templateName')" prop="name">
              <el-input
                v-model="form.name"
                :placeholder="$t('reportTemplates.dialog.templateNamePlaceholder')"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('reportTemplates.dialog.category')" prop="report_type">
              <el-select
                v-model="form.report_type"
                :placeholder="$t('reportTemplates.dialog.categoryPlaceholder')"
                style="width: 100%"
              >
                <el-option :label="$t('reportTemplates.category.sales')" value="sales" />
                <el-option :label="$t('reportTemplates.category.inventory')" value="inventory" />
                <el-option :label="$t('reportTemplates.category.finance')" value="finance" />
                <el-option :label="$t('reportTemplates.category.production')" value="production" />
                <el-option :label="$t('reportTemplates.category.custom')" value="custom" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item :label="$t('reportTemplates.dialog.description')" prop="description">
          <el-input
            v-model="form.description"
            type="textarea"
            :rows="3"
            :placeholder="$t('reportTemplates.dialog.descriptionPlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="$t('reportTemplates.dialog.parameters')" prop="parameters">
          <el-input
            v-model="parametersText"
            type="textarea"
            :rows="4"
            :placeholder="$t('reportTemplates.dialog.parametersPlaceholder')"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{
          $t('reportTemplates.dialog.cancel')
        }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
          $t('reportTemplates.dialog.confirm')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="previewVisible"
      :title="$t('reportTemplates.previewDialog.title')"
      width="900px"
      :aria-label="$t('reportTemplates.previewDialog.ariaLabel')"
    >
      <div v-loading="previewLoading" class="preview-container">
        <!-- Wave B-2 修复（B3-1）：使用 DOMPurify 净化后端返回的 HTML，防止 XSS 注入 -->
        <!-- eslint-disable-next-line vue/no-v-html -- 安全：已通过 DOMPurify 净化 -->
        <div v-if="previewData" v-html="sanitizedPreview"></div>
        <div v-else class="no-preview">{{ $t('reportTemplates.previewDialog.noData') }}</div>
      </div>
      <template #footer>
        <el-button @click="previewVisible = false">{{
          $t('reportTemplates.previewDialog.close')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';
import { Plus, Search } from '@element-plus/icons-vue';
// Wave B-2 修复（B3-1）：引入 DOMPurify 用于净化后端返回的 HTML 模板，防止 XSS
import DOMPurify from 'dompurify';
// v14 中风险安全修复（批次 243）：引入 escapeHtml 对单元格值做 HTML 转义，
// 防止后端数据中包含的恶意 HTML/脚本经 v-html 渲染后误导用户（DOMPurify 默认允许 <img>/<a> 等标签）
import { escapeHtml } from '@/utils/print';
import {
  createReportTemplate,
  updateReportTemplate,
  deleteReportTemplate,
  previewReportTemplate,
  exportReportTemplate,
  type ReportTemplate,
} from '@/api/report-templates';
// 批次 277：接入 useTableApi composable，统一表格分页/加载/查询逻辑
import { useTableApi } from '@/composables/useTableApi';

const { t } = useI18n({ useScope: 'global' });

const queryParams = reactive({
  keyword: '',
  report_type: '',
  status: '',
});

// 批次 277：接入 useTableApi，消除手写 list/total/listLoading/fetchData 重复
// useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
const {
  data: list,
  loading,
  page,
  pageSize,
  total,
  refresh: fetchData,
  setQueryParam,
} = useTableApi<ReportTemplate>({
  url: '/reports/enhanced/templates',
  onError: (err: unknown) =>
    // 批次 98 P2-D 修复（v5 复审）：类型守卫提取错误信息
    ElMessage.error(
      (err instanceof Error ? err.message : String(err)) || t('reportTemplates.message.loadFailed')
    ),
});

// 批次 277：同步筛选条件到 useTableApi.queryParams 并刷新
const syncQueryParams = () => {
  setQueryParam('keyword', queryParams.keyword || undefined);
  setQueryParam('report_type', queryParams.report_type || undefined);
  setQueryParam('status', queryParams.status || undefined);
};

// D05 Batch 3：categoryMap 改为函数返回，使 t() 在每次渲染时响应式求值（参照 print-templates/index.vue 的 getModuleLabel）
// report_type 词表为 sales/inventory/finance/production/custom（后端不强制枚举，落库存原值）
const getCategoryLabel = (reportType: string) => {
  const map: Record<string, string> = {
    sales: t('reportTemplates.category.sales'),
    inventory: t('reportTemplates.category.inventory'),
    finance: t('reportTemplates.category.finance'),
    production: t('reportTemplates.category.production'),
    custom: t('reportTemplates.category.custom'),
  };
  return map[reportType] || reportType;
};

const dialogVisible = ref(false);
const formRef = ref<FormInstance>();
const submitLoading = ref(false);
const parametersText = ref('');

/**
 * 表单本地模型：只收集真实进入 create/update DTO 的字段。
 * columns 后端是无类型 `Json NOT NULL` 列，页面尚无列设计器（能力缺口已登记），
 * 故新建时以空数组如实表示"未配置列"——不是伪造数据，也不静默丢弃用户输入。
 */
interface TemplateForm {
  id?: number;
  name: string;
  code: string;
  report_type: string;
  description: string;
}

const emptyForm = (): TemplateForm => ({
  id: undefined,
  name: '',
  code: '',
  report_type: 'custom',
  description: '',
});

const form = reactive<TemplateForm>(emptyForm());

const rules: FormRules = {
  code: [
    {
      required: true,
      message: t('reportTemplates.validation.templateCodeRequired'),
      trigger: 'blur',
    },
  ],
  name: [
    {
      required: true,
      message: t('reportTemplates.validation.templateNameRequired'),
      trigger: 'blur',
    },
  ],
  report_type: [
    {
      required: true,
      message: t('reportTemplates.validation.categoryRequired'),
      trigger: 'change',
    },
  ],
};

const openDialog = (row?: ReportTemplate) => {
  if (row) {
    form.id = row.id;
    form.name = row.name;
    form.code = row.code;
    form.report_type = row.report_type;
    form.description = row.description ?? '';
    parametersText.value = row.parameters ? JSON.stringify(row.parameters, null, 2) : '';
  } else {
    Object.assign(form, emptyForm());
    parametersText.value = '';
  }
  dialogVisible.value = true;
};

const handleSubmit = async () => {
  if (!formRef.value) return;
  await formRef.value.validate(async valid => {
    if (!valid) return;

    submitLoading.value = true;
    try {
      let parameters: Record<string, unknown> | undefined;
      if (parametersText.value) {
        try {
          parameters = JSON.parse(parametersText.value) as Record<string, unknown>;
        } catch {
          ElMessage.error(t('reportTemplates.message.parametersFormatError'));
          return;
        }
      }
      if (form.id) {
        await updateReportTemplate(form.id, {
          name: form.name,
          report_type: form.report_type,
          description: form.description || undefined,
        });
      } else {
        await createReportTemplate({
          name: form.name,
          code: form.code,
          report_type: form.report_type,
          columns: [],
          description: form.description || undefined,
          parameters,
        });
      }
      ElMessage.success(t('reportTemplates.message.operationSuccess'));
      dialogVisible.value = false;
      fetchData();
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      ElMessage.error(
        (error instanceof Error ? error.message : String(error)) ||
          t('reportTemplates.message.operationFailed')
      );
    } finally {
      submitLoading.value = false;
    }
  });
};

const handleDelete = async (row: ReportTemplate) => {
  try {
    await ElMessageBox.confirm(
      t('reportTemplates.message.deleteConfirm'),
      t('reportTemplates.message.deleteConfirmTitle'),
      { type: 'warning' }
    );
    await deleteReportTemplate(row.id);
    ElMessage.success(t('reportTemplates.message.deleteSuccess'));
    fetchData();
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    if (error !== 'cancel')
      ElMessage.error(
        (error instanceof Error ? error.message : String(error)) ||
          t('reportTemplates.message.deleteFailed')
      );
  }
};

const previewVisible = ref(false);
const previewLoading = ref(false);
const previewData = ref('');

// Wave B-2 修复（B3-1）：使用 DOMPurify.sanitize 净化预览 HTML 内容
// 安全原因：v-html 默认不转义，后端返回的模板内容若包含恶意脚本（<script>、onerror 等），
// 会在浏览器中执行导致 XSS 攻击。DOMPurify 通过白名单过滤危险标签和属性。
const sanitizedPreview = computed(() => {
  if (!previewData.value) return '';
  return DOMPurify.sanitize(previewData.value, {
    USE_PROFILES: { html: true },
    // 禁止危险标签（脚本/iframe/object/embed），即使 DOMPurify 默认也会过滤，作为双保险
    FORBID_TAGS: ['script', 'iframe', 'object', 'embed', 'form'],
    FORBID_ATTR: ['onerror', 'onload', 'onclick', 'onmouseover'],
  });
});

const handlePreview = async (row: ReportTemplate) => {
  previewLoading.value = true;
  previewVisible.value = true;
  try {
    const res = await previewReportTemplate(row.id);
    // 后端 preview_template 出参：{template_id, columns: string[], data: string[][], total, preview_rows}
    // columns 为表头名，data 为行数组（每行是与 columns 等长的字符串数组）
    const { columns, data } = res.data;
    if (columns.length && data.length) {
      // v14 中风险安全修复（批次 243）：表头与单元格值均经 escapeHtml 转义，
      // 防止后端返回的字段名/数据中包含 <img onerror> 等危险标签误导用户
      const headerHtml = columns.map(c => `<th>${escapeHtml(c)}</th>`).join('');
      const bodyHtml = data
        .map(
          rowCells => `<tr>${rowCells.map(cell => `<td>${escapeHtml(cell)}</td>`).join('')}</tr>`
        )
        .join('');
      previewData.value = `<table border="1" cellpadding="4" cellspacing="0" style="border-collapse:collapse;width:100%"><thead><tr>${headerHtml}</tr></thead><tbody>${bodyHtml}</tbody></table>`;
    } else {
      previewData.value = '';
    }
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('reportTemplates.message.previewFailed')
    );
    previewData.value = '';
  } finally {
    previewLoading.value = false;
  }
};

const handleGenerate = async (row: ReportTemplate) => {
  try {
    // 真实端点 POST /templates/{id}/export 返回 JSON 信封（content 为 base64），非二进制流；
    // 仓库 utils/export.ts 无 base64 落盘 helper（downloadFile 私有且前置 BOM 仅适用文本），
    // 故此处按 content_type 解码 base64 → Blob 后触发下载。
    const res = await exportReportTemplate(row.id, {});
    const { content, filename, content_type: contentType } = res.data;
    const binary = atob(content);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
    const blob = new Blob([bytes], { type: contentType });
    const link = document.createElement('a');
    link.href = URL.createObjectURL(blob);
    link.download = filename;
    link.click();
    URL.revokeObjectURL(link.href);
    ElMessage.success(t('reportTemplates.message.generateSuccess'));
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('reportTemplates.message.generateFailed')
    );
  }
};

// 批次 277：搜索/重置（同步筛选条件到 useTableApi 后重置到第一页并刷新）
const handleSearch = () => {
  syncQueryParams();
  page.value = 1;
  fetchData();
};

// 批次 277：分页（useTableApi 自动 watch page/pageSize 变化触发重载）
const handlePageChange = (p: number) => {
  page.value = p;
};

const handleSizeChange = (s: number) => {
  pageSize.value = s;
  page.value = 1;
};
</script>

<style scoped>
.report-templates-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.page-title {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
.filter-container {
  display: flex;
  gap: 12px;
  margin-bottom: 16px;
}
.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
.preview-container {
  min-height: 300px;
  max-height: 500px;
  overflow-y: auto;
  border: 1px solid #ebeef5;
  border-radius: 4px;
  padding: 16px;
}
.no-preview {
  text-align: center;
  color: #909399;
  padding: 40px;
}
</style>

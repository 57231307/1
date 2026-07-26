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
        <el-form-item :label="t('dyeRecipe.index.filterKeyword')">
          <el-input
            v-model="queryParams.keyword"
            :placeholder="t('dyeRecipe.index.placeholderKeyword')"
            clearable
            @clear="handleQuery"
          />
        </el-form-item>
        <el-form-item :label="t('dyeRecipe.index.filterColorNo')">
          <el-input
            v-model="queryParams.color_no"
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
            <el-option :label="t('dyeRecipe.index.optionDraft')" value="DRAFT" />
            <el-option :label="t('dyeRecipe.index.optionPending')" value="PENDING" />
            <el-option :label="t('dyeRecipe.index.optionApproved')" value="APPROVED" />
            <el-option :label="t('dyeRecipe.index.optionInactive')" value="INACTIVE" />
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
              v-if="row.status === 'DRAFT'"
              type="primary"
              link
              size="small"
              @click="handleEdit(row as DyeRecipe)"
              >{{ t('dyeRecipe.index.buttonEdit') }}</el-button
            >
            <el-button
              v-if="row.status === 'DRAFT'"
              type="success"
              link
              size="small"
              @click="handleSubmit(row as DyeRecipe)"
              >{{ t('dyeRecipe.index.buttonSubmit') }}</el-button
            >
            <el-button
              v-if="row.status === 'PENDING'"
              type="success"
              link
              size="small"
              @click="handleApprove(row as DyeRecipe)"
              >{{ t('dyeRecipe.index.buttonApprove') }}</el-button
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
import { ref, reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Download, Search, Refresh } from '@element-plus/icons-vue'
import {
  createDyeRecipe,
  updateDyeRecipe,
  approveDyeRecipe,
  submitDyeRecipe,
  getRecipeVersions,
  exportDyeRecipes,
} from '@/api/dye-recipe'
import type { DyeRecipe } from '@/api/dye-recipe'
import { logger } from '@/utils/logger'
import { useTableApi } from '@/composables/useTableApi'

const { t } = useI18n({ useScope: 'global' })

// 查询参数（筛选条件，分页由 useTableApi 管理）
const queryParams = reactive({
  keyword: '',
  color_no: '',
  status: '',
})

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
})

// 对话框
const dialogVisible = ref(false)
const dialogTitle = ref('')
const formRef = ref()
const isView = ref(false)

// 版本历史
const versionVisible = ref(false)
const versionList = ref<DyeRecipe[]>([])

// 表单数据
const formData = reactive({
  id: undefined as number | undefined,
  recipe_no: '',
  recipe_name: '',
  color_no: '',
  color_name: '',
  content: '',
  remarks: '',
})

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
}

// 批次 271：同步筛选条件到 useTableApi.queryParams 并刷新
// useTableApi 自动 watch page/pageSize 变化触发重载，无需手动 getList
const syncQueryParams = () => {
  setQueryParam('keyword', queryParams.keyword || undefined)
  setQueryParam('color_no', queryParams.color_no || undefined)
  setQueryParam('status', queryParams.status || undefined)
}

// 查询
const handleQuery = () => {
  syncQueryParams()
  page.value = 1
  refresh()
}

// 重置
const handleReset = () => {
  queryParams.keyword = ''
  queryParams.color_no = ''
  queryParams.status = ''
  syncQueryParams()
  page.value = 1
  refresh()
}

// 新建
const handleCreate = () => {
  dialogTitle.value = t('dyeRecipe.index.titleCreate')
  isView.value = false
  Object.assign(formData, {
    id: undefined,
    recipe_no: '',
    recipe_name: '',
    color_no: '',
    color_name: '',
    content: '',
    remarks: '',
  })
  dialogVisible.value = true
}

// 查看（v14 P0-3 修复：实现只读查看功能，原 handler 为空导致业务失效）
const handleView = (row: DyeRecipe) => {
  dialogTitle.value = t('dyeRecipe.index.titleView')
  isView.value = true
  Object.assign(formData, row)
  dialogVisible.value = true
}

// 编辑
const handleEdit = (row: DyeRecipe) => {
  dialogTitle.value = t('dyeRecipe.index.titleEdit')
  isView.value = false
  Object.assign(formData, row)
  dialogVisible.value = true
}

// 提交审批
const handleSubmit = async (row: DyeRecipe) => {
  try {
    await ElMessageBox.confirm(
      t('dyeRecipe.index.messageConfirmSubmit'),
      t('dyeRecipe.index.titlePrompt'),
      { type: 'warning' }
    )
    await submitDyeRecipe(row.id)
    ElMessage.success(t('dyeRecipe.index.messageSubmitSuccess'))
    refresh()
  } catch (error) {
    logger.error(t('dyeRecipe.index.messageSubmitFailed'), error)
  }
}

// 审批
const handleApprove = async (row: DyeRecipe) => {
  try {
    await ElMessageBox.confirm(
      t('dyeRecipe.index.messageConfirmApprove'),
      t('dyeRecipe.index.titlePrompt'),
      { type: 'warning' }
    )
    await approveDyeRecipe(row.id)
    ElMessage.success(t('dyeRecipe.index.messageApproveSuccess'))
    refresh()
  } catch (error) {
    logger.error(t('dyeRecipe.index.messageApproveFailed'), error)
  }
}

// 版本历史
const handleVersion = async (row: DyeRecipe) => {
  try {
    const res = await getRecipeVersions(row.id)
    versionList.value = res.data || []
    versionVisible.value = true
  } catch (error) {
    logger.error(t('dyeRecipe.index.messageVersionFailed'), error)
  }
}

// 查看版本（v14 中风险修复：实现版本详情查看逻辑，原 handler 为空导致功能失效）
const handleViewVersion = (row: DyeRecipe) => {
  versionVisible.value = false
  dialogTitle.value = t('dyeRecipe.index.titleViewVersion', { version: row.version })
  isView.value = true
  Object.assign(formData, row)
  dialogVisible.value = true
}

// 导出
const handleExport = async () => {
  try {
    const res = await exportDyeRecipes(queryParams)
    const url = window.URL.createObjectURL(new Blob([res]))
    const link = document.createElement('a')
    link.href = url
    link.setAttribute('download', t('dyeRecipe.index.exportFileName'))
    document.body.appendChild(link)
    link.click()
    document.body.removeChild(link)
    window.URL.revokeObjectURL(url)
    ElMessage.success(t('dyeRecipe.index.messageExportSuccess'))
  } catch (error) {
    logger.error(t('dyeRecipe.index.messageExportFailed'), error)
  }
}

// 提交表单
const handleSubmitForm = async () => {
  try {
    await formRef.value?.validate()
    if (formData.id) {
      await updateDyeRecipe(formData.id, formData)
    } else {
      await createDyeRecipe(formData)
    }
    ElMessage.success(t('dyeRecipe.index.messageSaveSuccess'))
    dialogVisible.value = false
    refresh()
  } catch (error) {
    logger.error(t('dyeRecipe.index.messageFormValidateFailed'), error)
  }
}

// 分页（useTableApi 自动 watch page/pageSize 变化触发重载）
const handleSizeChange = (val: number) => {
  pageSize.value = val
  page.value = 1
}

const handleCurrentChange = (val: number) => {
  page.value = val
}

// 获取状态类型
const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    DRAFT: 'info',
    PENDING: 'warning',
    APPROVED: 'success',
    INACTIVE: 'danger',
  }
  return map[status] || 'info'
}

// 获取状态标签（响应式求值）
const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    DRAFT: t('dyeRecipe.index.optionDraft'),
    PENDING: t('dyeRecipe.index.optionPending'),
    APPROVED: t('dyeRecipe.index.optionApproved'),
    INACTIVE: t('dyeRecipe.index.optionInactive'),
  }
  return map[status] || status
}

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

<template>
  <div class="quality-standards-page">
    <div class="page-header">
      <h2 class="page-title">质量标准管理</h2>
      <div class="header-actions">
        <el-button type="primary" @click="openDialog()">
          <el-icon><Plus /></el-icon>
          新建标准
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          导出
        </el-button>
      </div>
    </div>

    <el-card shadow="hover">
      <div class="filter-container">
        <el-input
          v-model="listQuery.keyword"
          placeholder="搜索标准编号/名称"
          style="width: 200px"
          clearable
          @clear="handleSearch"
          @keyup.enter="handleSearch"
        />
        <el-select v-model="listQuery.status" placeholder="状态" clearable style="width: 120px">
          <el-option label="草稿" value="draft" />
          <el-option label="已审批" value="approved" />
          <el-option label="已发布" value="published" />
          <el-option label="已归档" value="archived" />
        </el-select>
        <el-select v-model="listQuery.type" placeholder="类型" clearable style="width: 120px">
          <el-option label="产品标准" value="product" />
          <el-option label="工艺标准" value="process" />
          <el-option label="安全标准" value="safety" />
          <el-option label="环保标准" value="environmental" />
        </el-select>
        <el-button type="primary" @click="handleSearch">
          <el-icon><Search /></el-icon>
          搜索
        </el-button>
      </div>

      <el-table v-loading="loading" :data="list" stripe>
        <el-table-column prop="standard_code" label="标准编号" width="140" />
        <el-table-column prop="standard_name" label="标准名称" min-width="180" />
        <el-table-column prop="type" label="类型" width="100">
          <template #default="{ row }">
            {{ typeMap[row.type] }}
          </template>
        </el-table-column>
        <el-table-column prop="version" label="版本" width="80" />
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="statusTypeMap[row.status]" size="small">
              {{ statusMap[row.status] }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_by_name" label="创建人" width="100" />
        <el-table-column prop="approved_by_name" label="审批人" width="100">
          <template #default="{ row }">
            {{ row.approved_by_name || '-' }}
          </template>
        </el-table-column>
        <el-table-column prop="created_at" label="创建时间" width="160" />
        <el-table-column label="操作" width="280" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="openDialog(row)">编辑</el-button>
            <el-button
              v-if="row.status === 'draft'"
              type="success"
              link
              size="small"
              @click="handleApprove(row)"
              >审批</el-button
            >
            <el-button
              v-if="row.status === 'approved'"
              type="warning"
              link
              size="small"
              @click="handlePublish(row)"
              >发布</el-button
            >
            <el-button
              v-if="row.status === 'published'"
              type="info"
              link
              size="small"
              @click="handleArchive(row)"
              >归档</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              type="danger"
              link
              size="small"
              @click="handleDelete(row)"
              >删除</el-button
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
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
        />
      </div>
    </el-card>

    <el-dialog v-model="dialogVisible" :title="form.id ? '编辑标准' : '新建标准'" width="700px">
      <el-form ref="formRef" :model="form" :rules="rules" label-width="100px">
        <el-form-item label="标准编号" prop="standard_code">
          <el-input
            v-model="form.standard_code"
            :disabled="!!form.id"
            placeholder="请输入标准编号"
          />
        </el-form-item>
        <el-form-item label="标准名称" prop="standard_name">
          <el-input v-model="form.standard_name" placeholder="请输入标准名称" />
        </el-form-item>
        <el-form-item label="类型" prop="type">
          <el-select v-model="form.type" placeholder="请选择类型" style="width: 100%">
            <el-option label="产品标准" value="product" />
            <el-option label="工艺标准" value="process" />
            <el-option label="安全标准" value="safety" />
            <el-option label="环保标准" value="environmental" />
          </el-select>
        </el-form-item>
        <el-form-item label="版本" prop="version">
          <el-input v-model="form.version" placeholder="例如：1.0" />
        </el-form-item>
        <el-form-item label="标准内容" prop="content">
          <el-input v-model="form.content" type="textarea" :rows="6" placeholder="请输入标准内容" />
        </el-form-item>
        <el-form-item label="附件" prop="attachments">
          <el-input
            v-model="attachmentsText"
            type="textarea"
            placeholder='JSON格式数组，例如：["附件1.pdf", "附件2.docx"]'
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Download, Search } from '@element-plus/icons-vue'
import {
  createQualityStandard,
  updateQualityStandard,
  deleteQualityStandard,
  approveQualityStandard,
  publishQualityStandard,
  archiveQualityStandard,
  type QualityStandard,
} from '@/api/quality-standards'
// V15 P0-S12 修复（Batch 475d）：导出改用后端带水印 xlsx 接口
// 后端 GET /quality-standards/export 已就绪（含异步审计日志 + 水印）
import { exportFromBackend } from '@/utils/export'
import { useTableApi } from '@/composables/useTableApi'

// 批次 277：listQuery 仅保留筛选字段，page/page_size 交给 useTableApi 管理
const listQuery = reactive({
  keyword: '',
  status: '',
  type: '',
})

// 批次 277：接入 useTableApi，消除手写 list/total/listLoading/fetchData 重复
// useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
// listQualityStandards 返回 ApiResponse<QualityStandard[]>（{ data: T[], total: number }），
// useTableApi detectList 支持 data 字段、detectTotal 支持 res 外层 total，已兼容
const {
  data: list,
  total,
  loading,
  page,
  pageSize,
  refresh: fetchData,
  setQueryParam,
} = useTableApi<QualityStandard>({
  url: '/quality-standards',
  onError: (err: unknown) =>
    // 批次 98 P2-D 修复（v5 复审）：unknown 类型守卫
    ElMessage.error((err instanceof Error ? err.message : String(err)) || '获取数据失败'),
})

// 批次 277：同步 listQuery 筛选条件到 useTableApi.queryParams
const syncQueryParams = () => {
  setQueryParam('keyword', listQuery.keyword || undefined)
  setQueryParam('status', listQuery.status || undefined)
  setQueryParam('type', listQuery.type || undefined)
}

// 批次 277：搜索/重置统一入口：同步筛选条件 + 回到首页 + 拉取
const handleSearch = () => {
  syncQueryParams()
  page.value = 1
  fetchData()
}

// 批次 277：分页（useTableApi 自动 watch page/pageSize 变化触发重载）
const handlePageChange = (p: number) => {
  page.value = p
}

const handleSizeChange = (s: number) => {
  pageSize.value = s
  page.value = 1
}

const typeMap: Record<string, string> = {
  product: '产品标准',
  process: '工艺标准',
  safety: '安全标准',
  environmental: '环保标准',
}

const statusMap: Record<string, string> = {
  draft: '草稿',
  approved: '已审批',
  published: '已发布',
  archived: '已归档',
}

const statusTypeMap: Record<string, string> = {
  draft: 'info',
  approved: 'warning',
  published: 'success',
  archived: 'info',
}

const dialogVisible = ref(false)
const formRef = ref<FormInstance>()
const submitLoading = ref(false)
const attachmentsText = ref('')
const form = reactive<Partial<QualityStandard>>({
  id: undefined,
  standard_code: '',
  standard_name: '',
  version: '1.0',
  type: 'product',
  content: '',
  attachments: [],
})

const rules: FormRules = {
  standard_code: [{ required: true, message: '请输入标准编号', trigger: 'blur' }],
  standard_name: [{ required: true, message: '请输入标准名称', trigger: 'blur' }],
  type: [{ required: true, message: '请选择类型', trigger: 'change' }],
  version: [{ required: true, message: '请输入版本号', trigger: 'blur' }],
  content: [{ required: true, message: '请输入标准内容', trigger: 'blur' }],
}

const openDialog = (row?: QualityStandard) => {
  if (row) {
    Object.assign(form, row)
    attachmentsText.value = JSON.stringify(row.attachments || [], null, 2)
  } else {
    Object.assign(form, {
      id: undefined,
      standard_code: '',
      standard_name: '',
      version: '1.0',
      type: 'product',
      content: '',
      attachments: [],
    })
    attachmentsText.value = ''
  }
  dialogVisible.value = true
}

const handleSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async valid => {
    if (!valid) return

    submitLoading.value = true
    try {
      if (attachmentsText.value) {
        try {
          form.attachments = JSON.parse(attachmentsText.value)
        } catch (e) {
          ElMessage.error('附件格式错误，请检查JSON格式')
          return
        }
      }
      if (form.id) {
        await updateQualityStandard(form.id, form)
      } else {
        await createQualityStandard(form)
      }
      ElMessage.success('操作成功')
      dialogVisible.value = false
      fetchData()
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      ElMessage.error((error instanceof Error ? error.message : String(error)) || '操作失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const handleDelete = async (row: QualityStandard) => {
  try {
    await ElMessageBox.confirm('确定要删除此标准吗？', '确认删除', { type: 'warning' })
    await deleteQualityStandard(row.id)
    ElMessage.success('删除成功')
    fetchData()
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    if (error !== 'cancel') ElMessage.error((error instanceof Error ? error.message : String(error)) || '删除失败')
  }
}

const handleApprove = async (row: QualityStandard) => {
  try {
    await ElMessageBox.confirm('确定要审批通过此标准吗？', '确认审批', { type: 'warning' })
    await approveQualityStandard(row.id)
    ElMessage.success('审批成功')
    fetchData()
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    if (error !== 'cancel') ElMessage.error((error instanceof Error ? error.message : String(error)) || '审批失败')
  }
}

const handlePublish = async (row: QualityStandard) => {
  try {
    await ElMessageBox.confirm('确定要发布此标准吗？发布后将无法编辑。', '确认发布', {
      type: 'warning',
    })
    await publishQualityStandard(row.id)
    ElMessage.success('发布成功')
    fetchData()
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    if (error !== 'cancel') ElMessage.error((error instanceof Error ? error.message : String(error)) || '发布失败')
  }
}

const handleArchive = async (row: QualityStandard) => {
  try {
    await ElMessageBox.confirm('确定要归档此标准吗？', '确认归档', { type: 'warning' })
    await archiveQualityStandard(row.id)
    ElMessage.success('归档成功')
    fetchData()
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    if (error !== 'cancel') ElMessage.error((error instanceof Error ? error.message : String(error)) || '归档失败')
  }
}

// 导出 Excel（V15 P0-S12 修复 Batch 475d）
// 规则 3：导出统一使用 xlsx 格式（禁止 CSV 作为最终交付格式）
// 改为调用后端 GET /quality-standards/export，后端注入水印 + 异步审计日志
// 传入当前筛选条件：listQuery.type 映射为后端 standard_type 字段，status 与后端一致
const handleExport = async () => {
  await exportFromBackend(
    '/quality-standards/export',
    {
      standard_type: listQuery.type || undefined,
      status: listQuery.status || undefined,
    },
    'quality_standards_export'
  )
}
</script>

<style scoped>
.quality-standards-page {
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
</style>

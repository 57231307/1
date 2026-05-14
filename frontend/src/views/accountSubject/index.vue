<script setup lang="ts">
import { ref, computed } from 'vue'
import { ElTable, ElTableColumn, ElButton, ElDialog, ElForm, ElFormItem, ElInput, ElSelect, ElSwitch, ElTree, ElMessageBox, ElMessage, ElRow, ElCol } from 'element-plus'
import { Plus, Edit, Trash2, Eye, Refresh, ChevronRight } from '@element-plus/icons-vue'
import { listAccountSubjects, getAccountSubject, createAccountSubject, updateAccountSubject, deleteAccountSubject, enableAccountSubject, disableAccountSubject, getAccountSubjectTree, type AccountSubjectEntity } from '@/api/accountSubject'

const tableData = ref<AccountSubjectEntity[]>([])
const treeData = ref<any[]>([])
const total = ref(0)
const loading = ref(false)
const searchForm = ref({
  code: '',
  name: '',
  category: '',
  type: '',
  is_enabled: ''
})
const pagination = ref({
  page: 1,
  pageSize: 20
})

const dialogVisible = ref(false)
const dialogTitle = ref('新增会计科目')
const form = ref<Partial<AccountSubjectEntity>>({
  code: '',
  name: '',
  parent_id: undefined,
  level: 1,
  category: '',
  type: '',
  balance_type: 'debit',
  description: '',
  is_enabled: true
})

const viewDialogVisible = ref(false)
const viewData = ref<AccountSubjectEntity | null>(null)

const categories = [
  { label: '资产', value: 'asset' },
  { label: '负债', value: 'liability' },
  { label: '权益', value: 'equity' },
  { label: '成本', value: 'cost' },
  { label: '损益', value: 'income' }
]

const types = [
  { label: '流动资产', value: 'current_asset' },
  { label: '非流动资产', value: 'non_current_asset' },
  { label: '流动负债', value: 'current_liability' },
  { label: '非流动负债', value: 'non_current_liability' },
  { label: '所有者权益', value: 'owner_equity' },
  { label: '成本', value: 'cost' },
  { label: '收入', value: 'income' },
  { label: '费用', value: 'expense' }
]

const balanceTypes = [
  { label: '借方', value: 'debit' },
  { label: '贷方', value: 'credit' }
]

const getCategoryLabel = (value: string) => {
  return categories.find(c => c.value === value)?.label || value
}

const getTypeLabel = (value: string) => {
  return types.find(t => t.value === value)?.label || value
}

const getBalanceTypeLabel = (value: string) => {
  return balanceTypes.find(b => b.value === value)?.label || value
}

const loadData = async () => {
  loading.value = true
  try {
    const res = await listAccountSubjects({
      page: pagination.value.page,
      pageSize: pagination.value.pageSize,
      ...searchForm.value
    })
    tableData.value = res.data.list
    total.value = res.data.total
  } catch (error) {
    ElMessage.error('加载失败')
  } finally {
    loading.value = false
  }
}

const loadTree = async () => {
  try {
    const res = await getAccountSubjectTree()
    treeData.value = res.data
  } catch (error) {
    ElMessage.error('加载树结构失败')
  }
}

const handleSearch = () => {
  pagination.value.page = 1
  loadData()
}

const handleReset = () => {
  searchForm.value = {
    code: '',
    name: '',
    category: '',
    type: '',
    is_enabled: ''
  }
  handleSearch()
}

const handlePageChange = (page: number) => {
  pagination.value.page = page
  loadData()
}

const handlePageSizeChange = (pageSize: number) => {
  pagination.value.pageSize = pageSize
  loadData()
}

const openAddDialog = () => {
  dialogTitle.value = '新增会计科目'
  form.value = {
    code: '',
    name: '',
    parent_id: undefined,
    level: 1,
    category: '',
    type: '',
    balance_type: 'debit',
    description: '',
    is_enabled: true
  }
  dialogVisible.value = true
}

const openEditDialog = (row: AccountSubjectEntity) => {
  dialogTitle.value = '编辑会计科目'
  form.value = { ...row }
  dialogVisible.value = true
}

const openViewDialog = async (row: AccountSubjectEntity) => {
  try {
    const res = await getAccountSubject(row.id!)
    viewData.value = res.data
    viewDialogVisible.value = true
  } catch (error) {
    ElMessage.error('获取详情失败')
  }
}

const handleSubmit = async () => {
  if (!form.value.code || !form.value.name || !form.value.category || !form.value.type) {
    ElMessage.warning('请填写必填字段')
    return
  }
  try {
    if (form.value.id) {
      await updateAccountSubject(form.value.id, form.value)
      ElMessage.success('更新成功')
    } else {
      await createAccountSubject(form.value)
      ElMessage.success('新增成功')
    }
    dialogVisible.value = false
    loadData()
    loadTree()
  } catch (error) {
    ElMessage.error('操作失败')
  }
}

const handleDelete = async (row: AccountSubjectEntity) => {
  try {
    await ElMessageBox.confirm('确定要删除这个会计科目吗？', '提示', {
      type: 'warning'
    })
    await deleteAccountSubject(row.id!)
    ElMessage.success('删除成功')
    loadData()
    loadTree()
  } catch (error) {
    ElMessage.info('取消删除')
  }
}

const handleEnable = async (row: AccountSubjectEntity) => {
  try {
    await enableAccountSubject(row.id!)
    ElMessage.success('启用成功')
    loadData()
  } catch (error) {
    ElMessage.error('操作失败')
  }
}

const handleDisable = async (row: AccountSubjectEntity) => {
  try {
    await disableAccountSubject(row.id!)
    ElMessage.success('禁用成功')
    loadData()
  } catch (error) {
    ElMessage.error('操作失败')
  }
}

const treeProps = {
  label: 'name',
  children: 'children',
  isLeaf: 'is_leaf'
}

loadData()
loadTree()
</script>

<template>
  <div class="app-container">
    <div class="filter-container">
      <ElRow :gutter="20">
        <ElCol :span="6">
          <ElInput
            v-model="searchForm.code"
            placeholder="科目编码"
            class="filter-item"
            @keyup.enter="handleSearch"
          />
        </ElCol>
        <ElCol :span="6">
          <ElInput
            v-model="searchForm.name"
            placeholder="科目名称"
            class="filter-item"
            @keyup.enter="handleSearch"
          />
        </ElCol>
        <ElCol :span="6">
          <ElSelect
            v-model="searchForm.category"
            placeholder="科目类别"
            class="filter-item"
          >
            <ElOption label="全部" value="" />
            <ElOption v-for="c in categories" :key="c.value" :label="c.label" :value="c.value" />
          </ElSelect>
        </ElCol>
        <ElCol :span="6">
          <ElSelect
            v-model="searchForm.type"
            placeholder="科目类型"
            class="filter-item"
          >
            <ElOption label="全部" value="" />
            <ElOption v-for="t in types" :key="t.value" :label="t.label" :value="t.value" />
          </ElSelect>
        </ElCol>
      </ElRow>
      <div class="filter-actions">
        <ElButton type="primary" @click="handleSearch">查询</ElButton>
        <ElButton @click="handleReset">重置</ElButton>
        <ElButton type="success" @click="openAddDialog">
          <Plus /> 新增科目
        </ElButton>
      </div>
    </div>

    <div class="main-content">
      <div class="tree-panel">
        <div class="panel-header">
          <span>科目树</span>
          <ElButton size="small" @click="loadTree">
            <Refresh />
          </ElButton>
        </div>
        <ElTree
          :data="treeData"
          :props="treeProps"
          default-expand-all
          :highlight-current="true"
          @node-click="(data: any) => searchForm.code = data.code; handleSearch()"
        >
          <template #default="{ node, data }">
            <span class="tree-node">
              <span>{{ data.code }} - {{ data.name }}</span>
            </span>
          </template>
        </ElTree>
      </div>

      <div class="table-panel">
        <ElTable
          :data="tableData"
          :total="total"
          :loading="loading"
          :page-size="pagination.pageSize"
          :current-page="pagination.page"
          @current-change="handlePageChange"
          @size-change="handlePageSizeChange"
          border
          fit
          highlight-current-row
          style="width: 100%"
        >
          <ElTableColumn prop="code" label="科目编码" width="120" />
          <ElTableColumn prop="name" label="科目名称" width="150" />
          <ElTableColumn prop="category" label="类别" width="80">
            <template #default="scope">
              {{ getCategoryLabel(scope.row.category) }}
            </template>
          </ElTableColumn>
          <ElTableColumn prop="type" label="类型" width="100">
            <template #default="scope">
              {{ getTypeLabel(scope.row.type) }}
            </template>
          </ElTableColumn>
          <ElTableColumn prop="balance_type" label="余额方向" width="100">
            <template #default="scope">
              {{ getBalanceTypeLabel(scope.row.balance_type) }}
            </template>
          </ElTableColumn>
          <ElTableColumn prop="level" label="级别" width="60" align="center" />
          <ElTableColumn prop="is_enabled" label="状态" width="80">
            <template #default="scope">
              <ElSwitch
                :value="scope.row.is_enabled"
                :disabled="true"
                active-text="启用"
                inactive-text="禁用"
              />
            </template>
          </ElTableColumn>
          <ElTableColumn prop="description" label="备注" />
          <ElTableColumn label="操作" width="200" align="center">
            <template #default="scope">
              <ElButton size="small" @click="openViewDialog(scope.row)">
                <Eye />
              </ElButton>
              <ElButton size="small" type="primary" @click="openEditDialog(scope.row)">
                <Edit />
              </ElButton>
              <ElButton
                v-if="scope.row.is_enabled"
                size="small"
                type="warning"
                @click="handleDisable(scope.row)"
              >
                禁用
              </ElButton>
              <ElButton
                v-else
                size="small"
                type="success"
                @click="handleEnable(scope.row)"
              >
                启用
              </ElButton>
              <ElButton size="small" type="danger" @click="handleDelete(scope.row)">
                <Trash2 />
              </ElButton>
            </template>
          </ElTableColumn>
        </ElTable>
      </div>
    </div>

    <ElDialog :title="dialogTitle" :visible="dialogVisible" width="600px" @close="dialogVisible = false">
      <ElForm :model="form" label-width="120px">
        <ElFormItem label="科目编码" prop="code">
          <ElInput v-model="form.code" placeholder="请输入科目编码" />
        </ElFormItem>
        <ElFormItem label="科目名称" prop="name">
          <ElInput v-model="form.name" placeholder="请输入科目名称" />
        </ElFormItem>
        <ElFormItem label="上级科目" prop="parent_id">
          <ElTree
            :data="treeData"
            :props="treeProps"
            show-checkbox
            check-strictly
            :default-checked-keys="form.parent_id ? [form.parent_id] : []"
            @check-change="(data: any, checked: boolean) => { form.parent_id = checked ? data.id : undefined; form.level = data.level + 1 }"
          />
        </ElFormItem>
        <ElFormItem label="级别" prop="level">
          <ElInput v-model.number="form.level" disabled />
        </ElFormItem>
        <ElFormItem label="科目类别" prop="category">
          <ElSelect v-model="form.category" placeholder="请选择科目类别">
            <ElOption v-for="c in categories" :key="c.value" :label="c.label" :value="c.value" />
          </ElSelect>
        </ElFormItem>
        <ElFormItem label="科目类型" prop="type">
          <ElSelect v-model="form.type" placeholder="请选择科目类型">
            <ElOption v-for="t in types" :key="t.value" :label="t.label" :value="t.value" />
          </ElSelect>
        </ElFormItem>
        <ElFormItem label="余额方向" prop="balance_type">
          <ElSelect v-model="form.balance_type" placeholder="请选择余额方向">
            <ElOption v-for="b in balanceTypes" :key="b.value" :label="b.label" :value="b.value" />
          </ElSelect>
        </ElFormItem>
        <ElFormItem label="状态" prop="is_enabled">
          <ElSwitch v-model="form.is_enabled" active-text="启用" inactive-text="禁用" />
        </ElFormItem>
        <ElFormItem label="备注" prop="description">
          <ElInput v-model="form.description" type="textarea" rows="3" />
        </ElFormItem>
      </ElForm>
      <template #footer>
        <ElButton @click="dialogVisible = false">取消</ElButton>
        <ElButton type="primary" @click="handleSubmit">确定</ElButton>
      </template>
    </ElDialog>

    <ElDialog title="科目详情" :visible="viewDialogVisible" width="600px" @close="viewDialogVisible = false">
      <ElDescriptions v-if="viewData" :column="2" border>
        <ElDescriptionsItem label="科目编码">{{ viewData.code }}</ElDescriptionsItem>
        <ElDescriptionsItem label="科目名称">{{ viewData.name }}</ElDescriptionsItem>
        <ElDescriptionsItem label="级别">{{ viewData.level }}</ElDescriptionsItem>
        <ElDescriptionsItem label="类别">{{ getCategoryLabel(viewData.category) }}</ElDescriptionsItem>
        <ElDescriptionsItem label="类型">{{ getTypeLabel(viewData.type) }}</ElDescriptionsItem>
        <ElDescriptionsItem label="余额方向">{{ getBalanceTypeLabel(viewData.balance_type) }}</ElDescriptionsItem>
        <ElDescriptionsItem label="状态">{{ viewData.is_enabled ? '启用' : '禁用' }}</ElDescriptionsItem>
        <ElDescriptionsItem label="备注">{{ viewData.description || '-' }}</ElDescriptionsItem>
        <ElDescriptionsItem label="创建时间" :span="2">{{ viewData.created_at }}</ElDescriptionsItem>
        <ElDescriptionsItem label="更新时间" :span="2">{{ viewData.updated_at }}</ElDescriptionsItem>
      </ElDescriptions>
    </ElDialog>
  </div>
</template>

<style scoped>
.app-container {
  padding: 20px;
}

.filter-container {
  margin-bottom: 20px;
}

.filter-item {
  width: 100%;
}

.filter-actions {
  margin-top: 10px;
}

.main-content {
  display: flex;
  gap: 20px;
}

.tree-panel {
  width: 300px;
  flex-shrink: 0;
  border: 1px solid #ebeef5;
  border-radius: 4px;
  background: #fff;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 15px;
  border-bottom: 1px solid #ebeef5;
  font-weight: bold;
}

.table-panel {
  flex: 1;
}

.tree-node {
  display: flex;
  align-items: center;
}
</style>
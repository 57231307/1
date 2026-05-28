<template>
  <div class="crm-leads-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">线索管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>CRM</el-breadcrumb-item>
          <el-breadcrumb-item>线索管理</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          新建线索
        </el-button>
        <el-button @click="handleImport">
          <el-icon><Upload /></el-icon>
          导入
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          导出
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form">
        <el-form-item label="关键词">
          <el-input
            v-model="queryParams.keyword"
            placeholder="线索编号/公司名称/联系人"
            clearable
            @clear="handleQuery"
          />
        </el-form-item>
        <el-form-item label="线索来源">
          <el-select
            v-model="queryParams.lead_source"
            placeholder="选择来源"
            clearable
            @change="handleQuery"
          >
            <el-option label="网站" value="WEBSITE" />
            <el-option label="电话" value="PHONE" />
            <el-option label="展会" value="EXHIBITION" />
            <el-option label="推荐" value="REFERRAL" />
            <el-option label="其他" value="OTHER" />
          </el-select>
        </el-form-item>
        <el-form-item label="线索状态">
          <el-select
            v-model="queryParams.lead_status"
            placeholder="选择状态"
            clearable
            @change="handleQuery"
          >
            <el-option label="新线索" value="NEW" />
            <el-option label="已联系" value="CONTACTED" />
            <el-option label="已qualified" value="QUALIFIED" />
            <el-option label="已转化" value="CONVERTED" />
            <el-option label="已流失" value="LOST" />
          </el-select>
        </el-form-item>
        <el-form-item label="负责人">
          <el-select
            v-model="queryParams.owner_id"
            placeholder="选择负责人"
            clearable
            filterable
            @change="handleQuery"
          >
            <el-option v-for="u in users" :key="u.id" :label="u.name" :value="u.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="优先级">
          <el-select
            v-model="queryParams.priority"
            placeholder="选择优先级"
            clearable
            @change="handleQuery"
          >
            <el-option label="低" value="LOW" />
            <el-option label="中" value="MEDIUM" />
            <el-option label="高" value="HIGH" />
            <el-option label="紧急" value="URGENT" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">
            <el-icon><Search /></el-icon>
            查询
          </el-button>
          <el-button @click="handleReset">
            <el-icon><Refresh /></el-icon>
            重置
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table
        v-loading="loading"
        :data="leadList"
        border
        stripe
        @selection-change="handleSelectionChange"
      >
        <el-table-column type="selection" width="55" align="center" />
        <el-table-column type="index" label="序号" width="60" align="center" />
        <el-table-column prop="lead_no" label="线索编号" width="120" show-overflow-tooltip />
        <el-table-column
          prop="company_name"
          label="公司名称"
          min-width="150"
          show-overflow-tooltip
        />
        <el-table-column prop="contact_name" label="联系人" width="100" show-overflow-tooltip />
        <el-table-column prop="mobile_phone" label="手机号" width="120" show-overflow-tooltip />
        <el-table-column prop="lead_source" label="线索来源" width="100" align="center">
          <template #default="{ row }">
            <el-tag>{{ getSourceLabel(row.lead_source) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="lead_status" label="线索状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.lead_status)">{{
              getStatusLabel(row.lead_status)
            }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="priority" label="优先级" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="getPriorityType(row.priority)">{{
              getPriorityLabel(row.priority)
            }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="owner_name" label="负责人" width="100" show-overflow-tooltip />
        <el-table-column prop="last_follow_up_date" label="最近跟进" width="120" align="center" />
        <el-table-column prop="next_follow_up_date" label="下次跟进" width="120" align="center" />
        <el-table-column label="操作" width="250" align="center" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleView(row)">查看</el-button>
            <el-button
              v-if="row.lead_status !== 'CONVERTED'"
              type="primary"
              link
              size="small"
              @click="handleEdit(row)"
              >编辑</el-button
            >
            <el-button
              v-if="row.lead_status === 'NEW'"
              type="warning"
              link
              size="small"
              @click="handleContact(row)"
              >联系</el-button
            >
            <el-button
              v-if="row.lead_status === 'QUALIFIED'"
              type="success"
              link
              size="small"
              @click="handleConvert(row)"
              >转化</el-button
            >
            <el-button
              v-if="row.lead_status !== 'CONVERTED'"
              type="danger"
              link
              size="small"
              @click="handleLost(row)"
              >流失</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-container">
        <el-pagination
          v-model:current-page="queryParams.page"
          v-model:page-size="queryParams.page_size"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
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
    >
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="线索来源" prop="lead_source">
              <el-select v-model="formData.lead_source" placeholder="请选择线索来源">
                <el-option label="网站" value="WEBSITE" />
                <el-option label="电话" value="PHONE" />
                <el-option label="展会" value="EXHIBITION" />
                <el-option label="推荐" value="REFERRAL" />
                <el-option label="其他" value="OTHER" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="优先级" prop="priority">
              <el-select v-model="formData.priority" placeholder="请选择优先级">
                <el-option label="低" value="LOW" />
                <el-option label="中" value="MEDIUM" />
                <el-option label="高" value="HIGH" />
                <el-option label="紧急" value="URGENT" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="公司名称" prop="company_name">
              <el-input v-model="formData.company_name" placeholder="请输入公司名称" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="联系人" prop="contact_name">
              <el-input v-model="formData.contact_name" placeholder="请输入联系人姓名" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="手机号" prop="mobile_phone">
              <el-input v-model="formData.mobile_phone" placeholder="请输入手机号" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="邮箱" prop="email">
              <el-input v-model="formData.email" placeholder="请输入邮箱" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="职位" prop="contact_title">
              <el-input v-model="formData.contact_title" placeholder="请输入职位" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="负责人" prop="owner_id">
              <el-select v-model="formData.owner_id" placeholder="请选择负责人" filterable>
                <el-option v-for="u in users" :key="u.id" :label="u.name" :value="u.id" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="需求描述" prop="requirement_desc">
          <el-input
            v-model="formData.requirement_desc"
            type="textarea"
            :rows="3"
            placeholder="请输入需求描述"
          />
        </el-form-item>
        <el-form-item label="备注" prop="remarks">
          <el-input v-model="formData.remarks" type="textarea" :rows="2" placeholder="请输入备注" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSubmitForm">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Upload, Download, Search, Refresh } from '@element-plus/icons-vue'

// 查询参数
const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  lead_source: '',
  lead_status: '',
  owner_id: '',
  priority: '',
})

// 列表数据
const loading = ref(false)
const leadList = ref([])
const total = ref(0)
const selectedRows = ref([])

// 用户列表
const users = ref([])

// 对话框
const dialogVisible = ref(false)
const dialogTitle = ref('')
const formRef = ref()

// 表单数据
const formData = reactive({
  id: null,
  lead_source: '',
  company_name: '',
  contact_name: '',
  contact_title: '',
  mobile_phone: '',
  email: '',
  priority: 'MEDIUM',
  owner_id: '',
  requirement_desc: '',
  remarks: '',
})

// 表单验证规则
const formRules = {
  lead_source: [{ required: true, message: '请选择线索来源', trigger: 'change' }],
  contact_name: [{ required: true, message: '请输入联系人姓名', trigger: 'blur' }],
  owner_id: [{ required: true, message: '请选择负责人', trigger: 'change' }],
}

// 获取列表数据
const getList = async () => {
  loading.value = true
  try {
    // TODO: 调用API获取数据
    leadList.value = []
    total.value = 0
  } catch (error) {
    console.error('获取线索列表失败:', error)
  } finally {
    loading.value = false
  }
}

// 获取用户列表
const getUsers = async () => {
  try {
    // TODO: 调用API获取用户列表
    users.value = []
  } catch (error) {
    console.error('获取用户列表失败:', error)
  }
}

// 查询
const handleQuery = () => {
  queryParams.page = 1
  getList()
}

// 重置
const handleReset = () => {
  queryParams.keyword = ''
  queryParams.lead_source = ''
  queryParams.lead_status = ''
  queryParams.owner_id = ''
  queryParams.priority = ''
  handleQuery()
}

// 新建
const handleCreate = () => {
  dialogTitle.value = '新建线索'
  Object.assign(formData, {
    id: null,
    lead_source: '',
    company_name: '',
    contact_name: '',
    contact_title: '',
    mobile_phone: '',
    email: '',
    priority: 'MEDIUM',
    owner_id: '',
    requirement_desc: '',
    remarks: '',
  })
  dialogVisible.value = true
}

// 查看
const handleView = (row: any) => {}

// 编辑
const handleEdit = (row: any) => {
  dialogTitle.value = '编辑线索'
  Object.assign(formData, row)
  dialogVisible.value = true
}

// 联系
const handleContact = async (row: any) => {
  try {
    await ElMessageBox.confirm('确认标记为已联系？', '提示', { type: 'warning' })
    ElMessage.success('操作成功')
    getList()
  } catch (error) {
    console.error('操作失败:', error)
  }
}

// 转化
const handleConvert = async (row: any) => {
  try {
    await ElMessageBox.confirm('确认将该线索转化为客户？', '提示', { type: 'warning' })
    ElMessage.success('转化成功')
    getList()
  } catch (error) {
    console.error('转化失败:', error)
  }
}

// 流失
const handleLost = async (row: any) => {
  try {
    await ElMessageBox.confirm('确认标记该线索为流失？', '提示', { type: 'warning' })
    ElMessage.success('操作成功')
    getList()
  } catch (error) {
    console.error('操作失败:', error)
  }
}

// 导入
const handleImport = () => {
  ElMessage.info('导入功能开发中')
}

// 导出
const handleExport = () => {
  ElMessage.success('导出成功')
}

// 选择变化
const handleSelectionChange = (selection: any[]) => {
  selectedRows.value = selection
}

// 提交表单
const handleSubmitForm = async () => {
  try {
    await formRef.value?.validate()
    ElMessage.success('保存成功')
    dialogVisible.value = false
    getList()
  } catch (error) {
    console.error('表单验证失败:', error)
  }
}

// 分页
const handleSizeChange = (val: number) => {
  queryParams.page_size = val
  getList()
}

const handleCurrentChange = (val: number) => {
  queryParams.page = val
  getList()
}

// 获取来源标签
const getSourceLabel = (source: string) => {
  const map: Record<string, string> = {
    WEBSITE: '网站',
    PHONE: '电话',
    EXHIBITION: '展会',
    REFERRAL: '推荐',
    OTHER: '其他',
  }
  return map[source] || source
}

// 获取状态类型
const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    NEW: 'info',
    CONTACTED: 'warning',
    QUALIFIED: 'primary',
    CONVERTED: 'success',
    LOST: 'danger',
  }
  return map[status] || 'info'
}

// 获取状态标签
const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    NEW: '新线索',
    CONTACTED: '已联系',
    QUALIFIED: '已qualified',
    CONVERTED: '已转化',
    LOST: '已流失',
  }
  return map[status] || status
}

// 获取优先级类型
const getPriorityType = (priority: string) => {
  const map: Record<string, string> = {
    LOW: 'info',
    MEDIUM: '',
    HIGH: 'warning',
    URGENT: 'danger',
  }
  return map[priority] || ''
}

// 获取优先级标签
const getPriorityLabel = (priority: string) => {
  const map: Record<string, string> = {
    LOW: '低',
    MEDIUM: '中',
    HIGH: '高',
    URGENT: '紧急',
  }
  return map[priority] || priority
}

onMounted(() => {
  getList()
  getUsers()
})
</script>

<style scoped>
.crm-leads-page {
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

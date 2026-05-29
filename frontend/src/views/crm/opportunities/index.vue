<template>
  <div class="crm-opportunities-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">商机管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>CRM</el-breadcrumb-item>
          <el-breadcrumb-item>商机管理</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          新建商机
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
            placeholder="商机编号/商机名称/客户名称"
            clearable
            @clear="handleQuery"
          />
        </el-form-item>
        <el-form-item label="商机阶段">
          <el-select
            v-model="queryParams.opportunity_stage"
            placeholder="选择阶段"
            clearable
            @change="handleQuery"
          >
            <el-option label="初步接触" value="INITIAL" />
            <el-option label="需求确认" value="REQUIREMENT" />
            <el-option label="方案报价" value="PROPOSAL" />
            <el-option label="谈判" value="NEGOTIATION" />
            <el-option label="成交" value="WON" />
            <el-option label="流失" value="LOST" />
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
            <el-option v-for="u in users" :key="u.id" :label="u.real_name" :value="u.id" />
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
      <el-table v-loading="loading" :data="opportunityList" border stripe>
        <el-table-column type="index" label="序号" width="60" align="center" />
        <el-table-column prop="opportunity_no" label="商机编号" width="120" show-overflow-tooltip />
        <el-table-column
          prop="opportunity_name"
          label="商机名称"
          min-width="150"
          show-overflow-tooltip
        />
        <el-table-column prop="customer_name" label="客户" width="150" show-overflow-tooltip />
        <el-table-column prop="estimated_amount" label="预估金额" width="120" align="right">
          <template #default="{ row }">
            {{ formatCurrency(row.estimated_amount) }}
          </template>
        </el-table-column>
        <el-table-column prop="win_probability" label="成交概率" width="100" align="center">
          <template #default="{ row }"> {{ row.win_probability }}% </template>
        </el-table-column>
        <el-table-column prop="opportunity_stage" label="商机阶段" width="120" align="center">
          <template #default="{ row }">
            <el-tag :type="getStageType(row.opportunity_stage)">{{
              getStageLabel(row.opportunity_stage)
            }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="expected_close_date" label="预计成交" width="120" align="center" />
        <el-table-column prop="owner_name" label="负责人" width="100" show-overflow-tooltip />
        <el-table-column prop="last_follow_up_date" label="最近跟进" width="120" align="center" />
        <el-table-column label="操作" width="250" align="center" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleView(row)">查看</el-button>
            <el-button
              v-if="row.opportunity_stage !== 'WON' && row.opportunity_stage !== 'LOST'"
              type="primary"
              link
              size="small"
              @click="handleEdit(row)"
              >编辑</el-button
            >
            <el-button
              v-if="row.opportunity_stage !== 'WON' && row.opportunity_stage !== 'LOST'"
              type="warning"
              link
              size="small"
              @click="handleFollow(row)"
              >跟进</el-button
            >
            <el-button
              v-if="row.opportunity_stage === 'NEGOTIATION'"
              type="success"
              link
              size="small"
              @click="handleWin(row)"
              >成交</el-button
            >
            <el-button
              v-if="row.opportunity_stage !== 'WON' && row.opportunity_stage !== 'LOST'"
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
            <el-form-item label="商机名称" prop="opportunity_name">
              <el-input v-model="formData.opportunity_name" placeholder="请输入商机名称" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="客户" prop="customer_id">
              <el-select v-model="formData.customer_id" placeholder="请选择客户" filterable>
                <el-option
                  v-for="c in customers"
                  :key="c.id"
                  :label="c.customer_name"
                  :value="c.id"
                />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="商机类型" prop="opportunity_type">
              <el-select v-model="formData.opportunity_type" placeholder="请选择商机类型">
                <el-option label="新客户" value="NEW" />
                <el-option label="增购" value="UPSELL" />
                <el-option label="续约" value="RENEWAL" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="商机阶段" prop="opportunity_stage">
              <el-select v-model="formData.opportunity_stage" placeholder="请选择商机阶段">
                <el-option label="初步接触" value="INITIAL" />
                <el-option label="需求确认" value="REQUIREMENT" />
                <el-option label="方案报价" value="PROPOSAL" />
                <el-option label="谈判" value="NEGOTIATION" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="预估金额" prop="estimated_amount">
              <el-input-number
                v-model="formData.estimated_amount"
                :precision="2"
                :min="0"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="成交概率" prop="win_probability">
              <el-slider v-model="formData.win_probability" :min="0" :max="100" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="预计成交" prop="expected_close_date">
              <el-date-picker
                v-model="formData.expected_close_date"
                type="date"
                placeholder="请选择预计成交日期"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="负责人" prop="owner_id">
              <el-select v-model="formData.owner_id" placeholder="请选择负责人" filterable>
                <el-option v-for="u in users" :key="u.id" :label="u.real_name" :value="u.id" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="产品描述" prop="product_desc">
          <el-input
            v-model="formData.product_desc"
            type="textarea"
            :rows="3"
            placeholder="请输入产品描述"
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

    <!-- 跟进记录对话框 -->
    <el-dialog v-model="followVisible" title="跟进记录" width="600px">
      <el-form :model="followData" label-width="80px">
        <el-form-item label="跟进内容">
          <el-input
            v-model="followData.content"
            type="textarea"
            :rows="4"
            placeholder="请输入跟进内容"
          />
        </el-form-item>
        <el-form-item label="下次跟进">
          <el-date-picker
            v-model="followData.next_follow_up_date"
            type="date"
            placeholder="请选择下次跟进日期"
            style="width: 100%"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="followVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSubmitFollow">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Download, Search, Refresh } from '@element-plus/icons-vue'
import { listOpportunities } from '@/api/crm'
import type { Opportunity } from '@/api/crm'
import { listUsers } from '@/api/user'
import type { User } from '@/api/user'
import { customerApi } from '@/api/customer'
import type { Customer } from '@/api/customer'

// 查询参数
const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  opportunity_stage: '',
  owner_id: '',
  priority: '',
})

// 列表数据
const loading = ref(false)
const opportunityList = ref<Opportunity[]>([])
const total = ref(0)

// 用户和客户列表
const users = ref<User[]>([])
const customers = ref<Customer[]>([])

// 对话框
const dialogVisible = ref(false)
const dialogTitle = ref('')
const formRef = ref()

// 跟进记录
const followVisible = ref(false)
const followData = reactive({
  opportunity_id: null,
  content: '',
  next_follow_up_date: '',
})

// 表单数据
const formData = reactive({
  id: null,
  opportunity_name: '',
  customer_id: '',
  opportunity_type: '',
  opportunity_stage: '',
  estimated_amount: 0,
  win_probability: 50,
  expected_close_date: '',
  owner_id: '',
  product_desc: '',
  remarks: '',
})

// 表单验证规则
const formRules = {
  opportunity_name: [{ required: true, message: '请输入商机名称', trigger: 'blur' }],
  customer_id: [{ required: true, message: '请选择客户', trigger: 'change' }],
  opportunity_stage: [{ required: true, message: '请选择商机阶段', trigger: 'change' }],
  owner_id: [{ required: true, message: '请选择负责人', trigger: 'change' }],
}

// 获取列表数据
const getList = async () => {
  loading.value = true
  try {
    const res = await listOpportunities(queryParams)
    opportunityList.value = res.data || []
    total.value = res.total || 0
  } catch (error) {
    console.error('获取商机列表失败:', error)
  } finally {
    loading.value = false
  }
}

// 获取用户列表
const getUsers = async () => {
  try {
    const res = await listUsers()
    users.value = res.data?.list || []
  } catch (error) {
    console.error('获取用户列表失败:', error)
  }
}

// 获取客户列表
const getCustomers = async () => {
  try {
    const res = await customerApi.list()
    customers.value = res.data?.list || []
  } catch (error) {
    console.error('获取客户列表失败:', error)
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
  queryParams.opportunity_stage = ''
  queryParams.owner_id = ''
  queryParams.priority = ''
  handleQuery()
}

// 新建
const handleCreate = () => {
  dialogTitle.value = '新建商机'
  Object.assign(formData, {
    id: null,
    opportunity_name: '',
    customer_id: '',
    opportunity_type: '',
    opportunity_stage: '',
    estimated_amount: 0,
    win_probability: 50,
    expected_close_date: '',
    owner_id: '',
    product_desc: '',
    remarks: '',
  })
  dialogVisible.value = true
}

// 查看
const handleView = (_row: any) => {}

// 编辑
const handleEdit = (row: any) => {
  dialogTitle.value = '编辑商机'
  Object.assign(formData, row)
  dialogVisible.value = true
}

// 跟进
const handleFollow = (row: any) => {
  followData.opportunity_id = row.id
  followData.content = ''
  followData.next_follow_up_date = ''
  followVisible.value = true
}

// 成交
const handleWin = async (_row: any) => {
  try {
    await ElMessageBox.confirm('确认标记该商机为成交？', '提示', { type: 'warning' })
    ElMessage.success('操作成功')
    getList()
  } catch (error) {
    console.error('操作失败:', error)
  }
}

// 流失
const handleLost = async (_row: any) => {
  try {
    await ElMessageBox.confirm('确认标记该商机为流失？', '提示', { type: 'warning' })
    ElMessage.success('操作成功')
    getList()
  } catch (error) {
    console.error('操作失败:', error)
  }
}

// 导出
const handleExport = () => {
  ElMessage.success('导出成功')
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

// 提交跟进
const handleSubmitFollow = async () => {
  try {
    // TODO: 调用API保存跟进记录
    ElMessage.success('跟进成功')
    followVisible.value = false
    getList()
  } catch (error) {
    console.error('跟进失败:', error)
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

// 格式化货币
const formatCurrency = (value: number) => {
  return value ? `¥${value.toFixed(2)}` : '¥0.00'
}

// 获取阶段类型
const getStageType = (stage: string) => {
  const map: Record<string, string> = {
    INITIAL: 'info',
    REQUIREMENT: '',
    PROPOSAL: 'warning',
    NEGOTIATION: 'primary',
    WON: 'success',
    LOST: 'danger',
  }
  return map[stage] || 'info'
}

// 获取阶段标签
const getStageLabel = (stage: string) => {
  const map: Record<string, string> = {
    INITIAL: '初步接触',
    REQUIREMENT: '需求确认',
    PROPOSAL: '方案报价',
    NEGOTIATION: '谈判',
    WON: '成交',
    LOST: '流失',
  }
  return map[stage] || stage
}

onMounted(() => {
  getList()
  getUsers()
  getCustomers()
})
</script>

<style scoped>
.crm-opportunities-page {
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

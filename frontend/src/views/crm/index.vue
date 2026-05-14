<template>
  <div class="crm-page">
    <el-tabs v-model="activeTab">
      <el-tab-pane label="线索管理" name="lead">
        <div class="page-header">
          <h2 class="page-title">线索管理</h2>
          <el-button type="primary" @click="openLeadDialog">
            <el-icon><Plus /></el-icon>
            新建线索
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table :data="leads" v-loading="leadLoading" stripe>
            <el-table-column prop="lead_no" label="线索编号" width="120" />
            <el-table-column prop="name" label="联系人" width="120" />
            <el-table-column prop="company" label="公司名称" min-width="150" />
            <el-table-column prop="phone" label="电话" width="130" />
            <el-table-column prop="email" label="邮箱" min-width="180" />
            <el-table-column prop="source" label="来源" width="100" />
            <el-table-column prop="rating" label="评分" width="80" align="center">
              <template #default="{ row }">
                <el-tag :type="row.rating >= 4 ? 'success' : row.rating >= 2 ? 'warning' : 'info'" size="small">
                  {{ '★'.repeat(row.rating) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="status" label="状态" width="100" align="center">
              <template #default="{ row }">
                <el-tag :type="getLeadStatusType(row.status)" size="small">{{ getLeadStatusLabel(row.status) }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="assigned_to_name" label="负责人" width="100" />
            <el-table-column label="操作" width="200" fixed="right">
              <template #default>
                <el-button type="primary" link size="small" @click="viewLead">查看</el-button>
                <el-button type="success" link size="small" @click="convertLead">转化</el-button>
                <el-button type="warning" link size="small" @click="updateLeadStatus">更新状态</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="商机管理" name="opportunity">
        <div class="page-header">
          <h2 class="page-title">商机管理</h2>
          <el-button type="primary" @click="openOpportunityDialog">
            <el-icon><Plus /></el-icon>
            新建商机
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table :data="opportunities" v-loading="opportunityLoading" stripe>
            <el-table-column prop="opportunity_no" label="商机编号" width="120" />
            <el-table-column prop="name" label="商机名称" width="180" />
            <el-table-column prop="customer_name" label="客户" width="150" />
            <el-table-column prop="estimated_amount" label="预估金额" width="120" align="right">
              <template #default="{ row }">{{ formatMoney(row.estimated_amount) }}</template>
            </el-table-column>
            <el-table-column prop="probability" label="成功率" width="100" align="center">
              <template #default="{ row }">{{ row.probability }}%</template>
            </el-table-column>
            <el-table-column prop="stage" label="阶段" width="140">
              <template #default="{ row }">{{ getStageLabel(row.stage) }}</template>
            </el-table-column>
            <el-table-column prop="expected_close_date" label="预计成交" width="120" />
            <el-table-column label="操作" width="120" fixed="right">
              <template #default>
                <el-button type="primary" link size="small" @click="viewOpportunity">查看</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="客户画像" name="customer">
        <div class="page-header">
          <h2 class="page-title">客户画像</h2>
        </div>

        <el-row :gutter="20">
          <el-col :span="8">
            <el-card shadow="hover">
              <template #header><div class="card-header">客户概览</div></template>
              <el-statistic title="客户总数" :value="128" />
              <el-divider />
              <el-statistic title="活跃客户" :value="86" />
            </el-card>
          </el-col>
          <el-col :span="8">
            <el-card shadow="hover">
              <template #header><div class="card-header">销售数据</div></template>
              <el-statistic title="本月成交额" :value="128500" prefix="¥" />
              <el-divider />
              <el-statistic title="待跟进商机" :value="24" />
            </el-card>
          </el-col>
          <el-col :span="8">
            <el-card shadow="hover">
              <template #header><div class="card-header">线索数据</div></template>
              <el-statistic title="本月新增线索" :value="36" />
              <el-divider />
              <el-statistic title="线索转化率" :value="28" suffix="%" />
            </el-card>
          </el-col>
        </el-row>

        <el-card shadow="hover" class="mt-20">
          <template #header><div class="card-header">客户分析</div></template>
          <el-empty description="客户分析图表区域" />
        </el-card>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import { listLeads, listOpportunities, type Lead, type Opportunity } from '@/api/crm'

const activeTab = ref('lead')
const leads = ref<Lead[]>([])
const opportunities = ref<Opportunity[]>([])
const leadLoading = ref(false)
const opportunityLoading = ref(false)

const fetchLeads = async () => {
  leadLoading.value = true
  try {
    const res = await listLeads()
    leads.value = res.data || []
  } finally {
    leadLoading.value = false
  }
}

const fetchOpportunities = async () => {
  opportunityLoading.value = true
  try {
    const res = await listOpportunities()
    opportunities.value = res.data || []
  } finally {
    opportunityLoading.value = false
  }
}

const formatMoney = (amount: number) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const getLeadStatusType = (status: string) => {
  const map: Record<string, any> = { new: 'info', contacted: 'primary', qualified: 'success', proposal: 'warning', converted: 'success', lost: 'danger' }
  return map[status] || 'info'
}

const getLeadStatusLabel = (status: string) => {
  const map: Record<string, string> = { new: '新线索', contacted: '已联系', qualified: '已转化', proposal: '提案中', converted: '已成交', lost: '已流失' }
  return map[status] || status
}

const getStageLabel = (stage: string) => {
  const map: Record<string, string> = { qualification: '需求评估', needs_analysis: '需求分析', value_proposition: '价值呈现', proposal: '方案阶段', negotiation: '商务谈判', closed_won: '成交', closed_lost: '流标' }
  return map[stage] || stage
}

const openLeadDialog = () => ElMessage.info('新建线索功能开发中')
const viewLead = () => ElMessage.info('查看线索功能开发中')
const convertLead = async () => {
  try {
    await ElMessageBox.confirm('确定转化此线索为客户和商机吗？', '确认转化', { type: 'info' })
    ElMessage.success('转化成功')
    fetchLeads()
  } catch (e) { if (e !== 'cancel') console.error(e) }
}
const updateLeadStatus = () => ElMessage.info('更新线索状态功能开发中')
const openOpportunityDialog = () => ElMessage.info('新建商机功能开发中')
const viewOpportunity = () => ElMessage.info('查看商机功能开发中')

onMounted(() => {
  fetchLeads()
  fetchOpportunities()
})
</script>

<style scoped>
.crm-page { padding: 24px; background-color: #f5f7fa; min-height: 100%; }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; }
.page-title { font-size: 20px; font-weight: 600; color: #303133; margin: 0; }
.card-header { font-weight: 600; }
.mt-20 { margin-top: 20px; }
</style>

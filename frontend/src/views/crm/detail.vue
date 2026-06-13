<template>
  <div class="detail-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">客户 360 视图</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>CRM</el-breadcrumb-item>
          <el-breadcrumb-item>客户详情</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button @click="handleBack">
          <el-icon><Back /></el-icon>
          返回
        </el-button>
      </div>
    </div>

    <div v-loading="loading" class="detail-content">
      <template v-if="customer">
        <el-row :gutter="20">
          <el-col :span="16">
            <el-card shadow="hover" class="section-card">
              <template #header>
                <div class="card-header">
                  <span>基本信息</span>
                  <el-tag :type="customer.status === 'active' ? 'success' : 'info'" size="small">
                    {{ customer.status === 'active' ? '启用' : '禁用' }}
                  </el-tag>
                </div>
              </template>

              <el-descriptions :column="2" border>
                <el-descriptions-item label="客户编码">{{
                  customer.customer_code
                }}</el-descriptions-item>
                <el-descriptions-item label="客户名称">{{
                  customer.customer_name
                }}</el-descriptions-item>
                <el-descriptions-item label="联系人">{{
                  customer.contact_person
                }}</el-descriptions-item>
                <el-descriptions-item label="电话">{{ customer.phone }}</el-descriptions-item>
                <el-descriptions-item label="邮箱" :span="2">{{
                  customer.email
                }}</el-descriptions-item>
                <el-descriptions-item label="地址" :span="2">{{
                  customer.address
                }}</el-descriptions-item>
                <el-descriptions-item label="客户类型">
                  <el-tag :type="getTypeTag(customer.customer_type)" size="small">
                    {{ getTypeLabel(customer.customer_type) }}
                  </el-tag>
                </el-descriptions-item>
                <el-descriptions-item label="负责人">{{
                  customer.owner_name
                }}</el-descriptions-item>
                <el-descriptions-item label="信用额度">
                  {{ customer.credit_limit ? formatCurrency(customer.credit_limit) : '-' }}
                </el-descriptions-item>
                <el-descriptions-item label="订单总数">{{
                  customer.total_orders
                }}</el-descriptions-item>
                <el-descriptions-item label="累计金额">
                  {{ customer.total_amount ? formatCurrency(customer.total_amount) : '-' }}
                </el-descriptions-item>
                <el-descriptions-item label="最近下单">{{
                  customer.last_order_date || '-'
                }}</el-descriptions-item>
              </el-descriptions>
            </el-card>

            <el-card shadow="hover" class="section-card mt-20">
              <template #header>
                <div class="card-header">
                  <span>开票信息</span>
                </div>
              </template>

              <el-descriptions :column="2" border>
                <el-descriptions-item label="税号" :span="2">{{
                  customer.tax_number || '-'
                }}</el-descriptions-item>
                <el-descriptions-item label="开户银行">{{
                  customer.bank_name || '-'
                }}</el-descriptions-item>
                <el-descriptions-item label="银行账号">{{
                  customer.bank_account || '-'
                }}</el-descriptions-item>
              </el-descriptions>
            </el-card>

            <el-card shadow="hover" class="section-card mt-20">
              <template #header>
                <div class="card-header">
                  <span>联系人列表</span>
                  <el-button type="primary" size="small" @click="handleAddContact">
                    <el-icon><Plus /></el-icon>
                    新增联系人
                  </el-button>
                </div>
              </template>

              <el-table :data="customer.contacts" stripe>
                <el-table-column prop="name" label="姓名" width="120" />
                <el-table-column prop="title" label="职务" width="150" />
                <el-table-column prop="phone" label="电话" width="140" />
                <el-table-column prop="email" label="邮箱" min-width="180" />
                <el-table-column prop="is_primary" label="主联系人" width="100" align="center">
                  <template #default="{ row }">
                    <el-tag v-if="row.is_primary" type="warning" size="small">主</el-tag>
                  </template>
                </el-table-column>
              </el-table>
            </el-card>
          </el-col>

          <el-col :span="8">
            <el-card shadow="hover">
              <template #header><div class="card-header">RFM 评分</div></template>
              <div v-if="customer.rfm_score" class="rfm-display">
                <div class="rfm-level">
                  <span class="level-badge">{{ customer.rfm_score.level }}</span>
                  <span class="level-label">{{ customer.rfm_score.label }}</span>
                </div>
                <div class="rfm-scores">
                  <div class="rfm-item">
                    <span class="rfm-label">R (最近消费)</span>
                    <span class="rfm-value">{{ customer.rfm_score.recency }}</span>
                  </div>
                  <div class="rfm-item">
                    <span class="rfm-label">F (消费频率)</span>
                    <span class="rfm-value">{{ customer.rfm_score.frequency }}</span>
                  </div>
                  <div class="rfm-item">
                    <span class="rfm-label">M (消费金额)</span>
                    <span class="rfm-value">{{ customer.rfm_score.monetary }}</span>
                  </div>
                </div>
              </div>
              <el-empty v-else description="暂无 RFM 数据" />
            </el-card>

            <el-card shadow="hover" class="mt-20">
              <template #header>
                <div class="card-header">
                  <span>标签管理</span>
                  <el-button type="primary" size="small" @click="tagDialogVisible = true">
                    <el-icon><Plus /></el-icon>
                    添加标签
                  </el-button>
                </div>
              </template>

              <div class="tags-container">
                <el-tag
                  v-for="tag in customer.tags"
                  :key="tag.id"
                  :color="tag.color"
                  class="tag-item"
                  closable
                  @close="handleRemoveTag(tag.id)"
                >
                  {{ tag.name }}
                </el-tag>
                <span v-if="!customer.tags.length" class="no-tags">暂无标签</span>
              </div>
            </el-card>

            <el-card shadow="hover" class="mt-20">
              <template #header><div class="card-header">收货地址</div></template>
              <div class="address-list">
                <div
                  v-for="addr in customer.shipping_addresses"
                  :key="addr.id"
                  class="address-item"
                >
                  <div class="address-header">
                    <span class="addr-name">{{ addr.name }}</span>
                    <el-tag v-if="addr.is_default" type="warning" size="small">默认</el-tag>
                  </div>
                  <div class="addr-phone">{{ addr.phone }}</div>
                  <div class="addr-detail">
                    {{ addr.province }} {{ addr.city }} {{ addr.district }} {{ addr.detail }}
                  </div>
                </div>
                <el-empty v-if="!customer.shipping_addresses.length" description="暂无收货地址" />
              </div>
            </el-card>
          </el-col>
        </el-row>

        <el-card shadow="hover" class="section-card mt-20">
          <template #header>
            <div class="card-header">
              <span>跟进记录</span>
              <el-button type="primary" size="small" @click="handleAddFollowUp">
                <el-icon><Plus /></el-icon>
                新增跟进
              </el-button>
            </div>
          </template>

          <el-timeline>
            <el-timeline-item
              v-for="record in followUps"
              :key="record.id"
              :timestamp="record.created_at"
              placement="top"
              :type="getFollowUpType(record.type)"
            >
              <el-card>
                <div class="follow-up-header">
                  <span class="follow-up-type">{{ getFollowUpTypeLabel(record.type) }}</span>
                  <span class="follow-up-operator">跟进人：{{ record.operator_name }}</span>
                </div>
                <p class="follow-up-content">{{ record.content }}</p>
                <div v-if="record.next_follow_date" class="follow-up-next">
                  <el-icon><Clock /></el-icon>
                  下次跟进：{{ record.next_follow_date }}
                </div>
              </el-card>
            </el-timeline-item>
          </el-timeline>

          <div class="pagination-wrapper">
            <el-pagination
              v-model:current-page="followUpQuery.page"
              v-model:page-size="followUpQuery.page_size"
              :page-sizes="[10, 20, 50]"
              :total="followUpTotal"
              layout="total, sizes, prev, pager, next, jumper"
              @size-change="fetchFollowUps"
              @current-change="fetchFollowUps"
            />
          </div>
        </el-card>
      </template>
    </div>

    <!-- 新增跟进对话框 -->
    <el-dialog
      v-model="followUpDialogVisible"
      title="新增跟进记录"
      width="500px"
      :close-on-click-modal="false"
    >
      <el-form ref="followUpFormRef" :model="followUpForm" label-width="100px">
        <el-form-item label="跟进方式" prop="type">
          <el-select v-model="followUpForm.type" placeholder="请选择跟进方式" style="width: 100%">
            <el-option label="电话" value="phone" />
            <el-option label="面谈" value="meeting" />
            <el-option label="邮件" value="email" />
            <el-option label="微信" value="wechat" />
            <el-option label="拜访" value="visit" />
          </el-select>
        </el-form-item>
        <el-form-item label="跟进内容" prop="content">
          <el-input
            v-model="followUpForm.content"
            type="textarea"
            :rows="4"
            placeholder="请输入跟进内容"
          />
        </el-form-item>
        <el-form-item label="下次跟进">
          <el-date-picker
            v-model="followUpForm.next_follow_date"
            type="date"
            placeholder="选择日期"
            style="width: 100%"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="followUpDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="followUpSubmitLoading" @click="submitFollowUp"
          >保存</el-button
        >
      </template>
    </el-dialog>

    <!-- 添加标签对话框 -->
    <el-dialog v-model="tagDialogVisible" title="添加标签" width="400px">
      <el-form ref="tagFormRef" :model="tagForm" label-width="80px">
        <el-form-item label="标签名称" prop="name">
          <el-select v-model="tagForm.name" placeholder="选择已有标签" style="width: 100%">
            <el-option
              v-for="tag in availableTags"
              :key="tag.id"
              :label="tag.name"
              :value="tag.name"
            />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="tagDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleAddTag">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import type { FormInstance } from 'element-plus'
import { Back, Plus, Clock } from '@element-plus/icons-vue'
import crmEnhancedApi, {
  type Customer360,
  type FollowUpRecord,
  type CustomerTag,
} from '@/api/crm-enhanced'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'

const hasLoaded = createLazyLoader()

const route = useRoute()
const router = useRouter()

const loading = ref(false)
const followUpSubmitLoading = ref(false)
const customer = ref<Customer360 | null>(null)
const followUps = ref<FollowUpRecord[]>([])
const followUpTotal = ref(0)
const availableTags = ref<CustomerTag[]>([])
const tagDialogVisible = ref(false)
const followUpDialogVisible = ref(false)
const followUpFormRef = ref<FormInstance>()
const tagFormRef = ref<FormInstance>()

const followUpQuery = reactive({
  page: 1,
  page_size: 10,
})

const followUpForm = reactive({
  type: 'phone',
  content: '',
  next_follow_date: '',
})

const tagForm = reactive({
  name: '',
})

const customerId = Number(route.params.id)

const formatCurrency = (amount: number) => `¥${(amount || 0).toFixed(2)}`

const getTypeLabel = (type: string) => {
  const labels: Record<string, string> = {
    normal: '普通客户',
    vip: 'VIP客户',
    wholesale: '批发客户',
  }
  return labels[type] || type
}

const getTypeTag = (type: string) => {
  const tags: Record<string, string> = { normal: '', vip: 'warning', wholesale: 'success' }
  return tags[type] || ''
}

const getFollowUpType = (type: string) => {
  const types: Record<string, string> = {
    phone: 'primary',
    meeting: 'success',
    email: 'info',
    wechat: 'warning',
    visit: 'danger',
  }
  return types[type] || ''
}

const getFollowUpTypeLabel = (type: string) => {
  const labels: Record<string, string> = {
    phone: '电话',
    meeting: '面谈',
    email: '邮件',
    wechat: '微信',
    visit: '拜访',
  }
  return labels[type] || type
}

const fetchCustomer360 = async () => {
  loading.value = true
  try {
    const res = await crmEnhancedApi.getCustomer360(customerId)
    customer.value = res.data
  } catch (error: any) {
    ElMessage.error(error.message || '获取客户详情失败')
  } finally {
    loading.value = false
  }
}

const fetchFollowUps = async () => {
  try {
    const res = await crmEnhancedApi.getFollowUps(customerId, followUpQuery)
    followUps.value = res.data?.list || []
    followUpTotal.value = res.data?.total || 0
  } catch (error: any) {
    followUps.value = []
    followUpTotal.value = 0
  }
}

const fetchTags = async () => {
  try {
    const res = await crmEnhancedApi.getTags()
    availableTags.value = res.data || []
  } catch (error: any) {
    availableTags.value = []
  }
}

const handleBack = () => {
  router.back()
}

const handleAddFollowUp = () => {
  followUpForm.type = 'phone'
  followUpForm.content = ''
  followUpForm.next_follow_date = ''
  followUpDialogVisible.value = true
}

const submitFollowUp = async () => {
  if (!followUpForm.content.trim()) {
    ElMessage.warning('请输入跟进内容')
    return
  }

  followUpSubmitLoading.value = true
  try {
    await crmEnhancedApi.createFollowUp(customerId, {
      type: followUpForm.type,
      content: followUpForm.content,
      next_follow_date: followUpForm.next_follow_date || undefined,
    })
    ElMessage.success('跟进记录已保存')
    followUpDialogVisible.value = false
    fetchFollowUps()
    fetchCustomer360()
  } catch (error: any) {
    ElMessage.error(error.message || '保存失败')
  } finally {
    followUpSubmitLoading.value = false
  }
}

const handleAddTag = async () => {
  if (!tagForm.name) {
    ElMessage.warning('请选择标签')
    return
  }

  const selectedTag = availableTags.value.find(t => t.name === tagForm.name)
  if (!selectedTag) return

  try {
    await crmEnhancedApi.addTagToCustomer(customerId, selectedTag.id)
    ElMessage.success('标签已添加')
    tagDialogVisible.value = false
    tagForm.name = ''
    fetchCustomer360()
  } catch (error: any) {
    ElMessage.error(error.message || '添加标签失败')
  }
}

const handleRemoveTag = async (tagId: number) => {
  try {
    await crmEnhancedApi.removeTagFromCustomer(customerId, tagId)
    ElMessage.success('标签已移除')
    fetchCustomer360()
  } catch (error: any) {
    ElMessage.error(error.message || '移除标签失败')
  }
}

const handleAddContact = () => {
  ElMessage.info('新增联系人功能待实现')
}

onMounted(() => {
  if (!customerId) {
    ElMessage.error('缺少客户 ID 参数')
    router.back()
    return
  }
  loadIfNot('fetchCustomer360', fetchCustomer360, hasLoaded)
  loadIfNot('fetchFollowUps', fetchFollowUps, hasLoaded)
  loadIfNot('fetchTags', fetchTags, hasLoaded)
})
</script>

<style scoped>
.detail-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 24px;
}
.header-left .page-title {
  font-size: 28px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 12px 0;
}
.header-actions {
  display: flex;
  gap: 12px;
}
.detail-content {
  min-height: 400px;
}
.section-card {
  margin-bottom: 0;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-weight: 600;
}
.mt-20 {
  margin-top: 20px;
}
.pagination-wrapper {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}

.rfm-display {
  padding: 12px 0;
}
.rfm-level {
  text-align: center;
  margin-bottom: 20px;
}
.level-badge {
  display: inline-block;
  width: 60px;
  height: 60px;
  line-height: 60px;
  border-radius: 50%;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: #fff;
  font-size: 28px;
  font-weight: 700;
}
.level-label {
  display: block;
  margin-top: 8px;
  font-size: 14px;
  color: #606266;
}
.rfm-scores {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.rfm-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  background: #fafafa;
  border-radius: 6px;
}
.rfm-label {
  font-size: 13px;
  color: #606266;
}
.rfm-value {
  font-size: 18px;
  font-weight: 600;
  color: #303133;
}

.tags-container {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  min-height: 40px;
}
.tag-item {
  border: none;
}
.no-tags {
  color: #909399;
  font-size: 13px;
}

.address-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.address-item {
  padding: 12px;
  background: #fafafa;
  border-radius: 6px;
}
.address-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}
.addr-name {
  font-weight: 600;
  color: #303133;
}
.addr-phone {
  font-size: 13px;
  color: #606266;
  margin-bottom: 4px;
}
.addr-detail {
  font-size: 13px;
  color: #909399;
}

.follow-up-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}
.follow-up-type {
  font-weight: 600;
  color: #303133;
}
.follow-up-operator {
  font-size: 12px;
  color: #909399;
}
.follow-up-content {
  color: #606266;
  margin: 0 0 8px 0;
  line-height: 1.6;
}
.follow-up-next {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  color: #409eff;
}
</style>

<template>
  <div class="page">
    <el-tabs v-model="activeTab" type="border-card">
      <!-- 权限委托 -->
      <el-tab-pane label="权限委托" name="delegation">
        <div class="toolbar mb">
          <el-button type="primary" @click="delegationDialogVisible = true">新建委托</el-button>
          <el-button plain @click="loadDelegations">全部委托</el-button>
          <el-button plain @click="onLoadActiveDelegations">生效中委托</el-button>
          <el-button plain @click="onExpireOverdue">清理过期委托</el-button>
        </div>
        <el-table v-loading="loading" :data="delegations" border>
          <el-table-column prop="id" label="ID" width="70" />
          <template v-for="col in delegationCols" :key="col">
            <el-table-column
              :prop="col"
              :label="col.replace(/_/g, ' ')"
              min-width="140"
              show-overflow-tooltip
            />
          </template>
          <el-table-column label="操作" width="180" fixed="right">
            <template #default="{ row }">
              <el-button size="small" @click="onDelegationDetail(row)">详情</el-button>
              <el-button size="small" type="danger" plain @click="onRevoke(row)">撤销</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <!-- 角色关系 -->
      <el-tab-pane label="角色关系" name="role-relation">
        <div class="toolbar mb">
          <el-button type="primary" @click="relationDialogVisible = true">新建角色关系</el-button>
        </div>
        <el-table v-loading="loadingRelations" :data="relations" border>
          <el-table-column prop="id" label="ID" width="70" />
          <template v-for="col in relationCols" :key="col">
            <el-table-column
              :prop="col"
              :label="col.replace(/_/g, ' ')"
              min-width="140"
              show-overflow-tooltip
            />
          </template>
          <el-table-column label="操作" width="120" fixed="right">
            <template #default="{ row }">
              <el-button size="small" type="danger" plain @click="onDeleteRelation(row)"
                >删除</el-button
              >
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <!-- AI 模型治理 -->
      <el-tab-pane label="AI 模型治理" name="ai">
        <div class="toolbar mb">
          <el-button type="primary" @click="versionDialogVisible = true">注册模型版本</el-button>
          <el-button plain @click="onReconcile">月度对账</el-button>
          <el-button plain @click="onAccuracyReports">准确率报表</el-button>
          <el-button plain @click="onModelEvaluation">模型评估</el-button>
          <el-button plain @click="onDecisionLogs">决策日志</el-button>
        </div>
        <el-table v-loading="loadingModels" :data="modelVersions" border>
          <el-table-column prop="id" label="ID" width="70" />
          <template v-for="col in modelCols" :key="col">
            <el-table-column
              :prop="col"
              :label="col.replace(/_/g, ' ')"
              min-width="140"
              show-overflow-tooltip
            />
          </template>
          <el-table-column label="操作" width="340" fixed="right">
            <template #default="{ row }">
              <el-button size="small" type="success" @click="onApprove(row)">审批</el-button>
              <el-button size="small" type="primary" plain @click="onDrift(row)"
                >漂移检测</el-button
              >
              <el-button size="small" @click="onEvaluations(row)">评估记录</el-button>
              <el-button size="small" type="warning" plain @click="onModelStatus(row, 'active')"
                >激活</el-button
              >
              <el-button size="small" type="info" plain @click="onModelStatus(row, 'deprecated')"
                >弃用</el-button
              >
            </template>
          </el-table-column>
        </el-table>
        <pre v-if="aiResult" class="result-box">{{ aiResult }}</pre>
      </el-tab-pane>

      <el-tab-pane label="角色变更审批" name="role-change">
        <div class="toolbar mb">
          <el-button type="primary" :loading="roleChangeLoading" @click="loadRoleChanges">
            刷新
          </el-button>
          <el-button type="primary" plain @click="roleChangeDialogVisible = true"
            >新建审批</el-button
          >
        </div>
        <el-table v-loading="roleChangeLoading" :data="roleChanges" border>
          <el-table-column prop="id" label="ID" width="70" />
          <template v-for="col in roleChangeCols" :key="col">
            <el-table-column
              :prop="col"
              :label="col.replace(/_/g, ' ')"
              min-width="130"
              show-overflow-tooltip
            />
          </template>
          <el-table-column label="操作" width="360" fixed="right">
            <template #default="{ row }">
              <el-button size="small" @click="onRoleChangeDetail(row)">详情</el-button>
              <el-button
                v-if="row.status === 'pending_l1'"
                size="small"
                type="success"
                @click="onRoleChangeAction(row, 'approve-l1')"
                >一级审批</el-button
              >
              <el-button
                v-if="row.status === 'pending_l2'"
                size="small"
                type="success"
                @click="onRoleChangeAction(row, 'approve-l2')"
                >二级审批</el-button
              >
              <el-button
                v-if="row.status === 'pending_l1' || row.status === 'pending_l2'"
                size="small"
                type="danger"
                @click="onRoleChangeAction(row, 'reject')"
                >驳回</el-button
              >
              <el-button size="small" @click="onRoleChangeAction(row, 'cancel')">撤销</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <!-- 设备连接 -->
      <el-tab-pane label="设备连接" name="device">
        <div class="toolbar mb">
          <el-tag>在线设备：{{ onlineCount }}</el-tag>
          <el-button type="primary" @click="deviceDialogVisible = true">注册设备</el-button>
          <el-button plain @click="onCleanupTimeoutDevices">清理超时设备</el-button>
        </div>
        <el-table v-loading="loadingDevices" :data="devices" border>
          <el-table-column prop="id" label="ID" width="70" />
          <template v-for="col in deviceCols" :key="col">
            <el-table-column
              :prop="col"
              :label="col.replace(/_/g, ' ')"
              min-width="140"
              show-overflow-tooltip
            />
          </template>
          <el-table-column label="操作" width="220" fixed="right">
            <template #default="{ row }">
              <el-button size="small" @click="onDeviceDetail(row)">详情</el-button>
              <el-button size="small" type="success" plain @click="onDeviceHeartbeat(row)">
                心跳
              </el-button>
              <el-button size="small" type="warning" plain @click="onDisconnectDevice(row)">
                断开
              </el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>
    </el-tabs>

    <el-dialog v-model="relationDialogVisible" title="新建角色关系" width="460">
      <el-form :model="relationForm" label-width="110px">
        <el-form-item label="主角色" required
          ><el-input v-model="relationForm.role_a"
        /></el-form-item>
        <el-form-item label="关联角色" required
          ><el-input v-model="relationForm.role_b"
        /></el-form-item>
        <el-form-item label="关系类型" required>
          <el-select v-model="relationForm.relation_type" class="w-full">
            <el-option label="互斥" value="mutual_exclusive" />
            <el-option label="继承" value="inheritance" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="relationDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="onCreateRelation">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="versionDialogVisible" title="注册模型版本" width="480">
      <el-form :model="versionForm" label-width="110px">
        <el-form-item label="模型名" required
          ><el-input v-model="versionForm.model_name"
        /></el-form-item>
        <el-form-item label="版本号" required
          ><el-input v-model="versionForm.version"
        /></el-form-item>
        <el-form-item label="说明"
          ><el-input v-model="versionForm.description" type="textarea" :rows="2"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="versionDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="onCreateVersion">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="deviceDialogVisible" title="注册设备" width="460">
      <el-form :model="deviceForm" label-width="110px">
        <el-form-item label="设备编码" required
          ><el-input v-model="deviceForm.device_code"
        /></el-form-item>
        <el-form-item label="设备名称"><el-input v-model="deviceForm.device_name" /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="deviceDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="onRegisterDevice">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="delegationDialogVisible" title="新建权限委托" width="480">
      <el-form :model="delegationForm" label-width="110px">
        <!-- 后端 CreateDelegationRequest：delegator 取当前登录用户（谁委托自己的权限），
             一条委托仅一个 permission_code（String 列），时间必填 RFC3339 -->
        <el-form-item label="委托人">
          <span>当前登录用户（ID：{{ userStore.userInfo?.id || '-' }}）</span>
        </el-form-item>
        <el-form-item label="受托人ID" required
          ><el-input-number v-model="delegationForm.delegatee_id" :min="1"
        /></el-form-item>
        <el-form-item label="权限码" required
          ><el-input
            v-model="delegationForm.permission_code"
            placeholder="单个权限码，如 user:read"
          />
        </el-form-item>
        <el-form-item label="开始时间" required>
          <el-date-picker
            v-model="delegationForm.valid_from"
            type="datetime"
            value-format="YYYY-MM-DDTHH:mm:ss"
            class="w-full"
          />
        </el-form-item>
        <el-form-item label="结束时间" required>
          <el-date-picker
            v-model="delegationForm.valid_until"
            type="datetime"
            value-format="YYYY-MM-DDTHH:mm:ss"
            class="w-full"
          />
        </el-form-item>
        <el-form-item label="委托原因"><el-input v-model="delegationForm.reason" /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="delegationDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="onSaveDelegation">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="roleChangeDialogVisible" title="新建角色变更审批" width="500">
      <el-form :model="roleChangeForm" label-width="120px">
        <!-- change_type 取值域=后端服务层白名单（越界 422）；target_role_code 为非 Option 必填；
             后端 DTO/审批表均无 reason 字段，变更原因输入被丢弃（缺失已登记串行清单），故移除 -->
        <el-form-item label="变更类型" required>
          <el-select v-model="roleChangeForm.change_type" style="width: 100%">
            <el-option label="分配角色" value="assign_role" />
            <el-option label="授予权限" value="assign_permission" />
            <el-option label="移除权限" value="remove_permission" />
          </el-select>
        </el-form-item>
        <el-form-item label="目标角色ID" required
          ><el-input-number v-model="roleChangeForm.target_role_id" :min="1"
        /></el-form-item>
        <el-form-item label="目标角色编码" required
          ><el-input v-model="roleChangeForm.target_role_code" placeholder="如 warehouse_keeper"
        /></el-form-item>
        <el-form-item label="目标用户ID"
          ><el-input-number v-model="roleChangeForm.target_user_id" :min="1"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="roleChangeDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="onSaveRoleChange">提交</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="detailDialogVisible" title="详情" width="560">
      <pre class="detail-json">{{ detailJson }}</pre>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import type { ApiResponse } from '@/types/api';
import { useUserStore } from '@/store/user';
import type { CreatePermissionDelegationPayload, RoleChangeType } from '@/api/system-governance';
import {
  approveModelVersion,
  reconcileMonthly,
  createModelVersion,
  createRoleRelation,
  deleteRoleRelation,
  detectModelDrift,
  expireOverdueDelegations,
  getAccuracyReports,
  getDelegationList,
  getDeviceList,
  getModelEvaluations,
  getModelVersionList,
  getOnlineDeviceCount,
  getRoleRelations,
  registerDevice,
  revokeDelegation,
  changeModelStatus,
  createModelEvaluation,
  getDecisionLogs,
  getRoleChangeApprovals,
  getRoleChangeApproval,
  createRoleChangeApproval,
  approveRoleChangeL1,
  approveRoleChangeL2,
  rejectRoleChangeApproval,
  cancelRoleChangeApproval,
  createPermissionDelegation,
  getDelegation,
  getActiveDelegatedPermissions,
  getDeviceConnection,
  sendDeviceHeartbeat,
  disconnectDevice,
  cleanupTimeoutDevices,
} from '@/api/system-governance';

const { t } = useI18n({ useScope: 'global' });
// 权限委托委托人=当前登录用户（后端 delegator_id 必填，真实取值不手填）
const userStore = useUserStore();

const activeTab = ref('delegation');
// 权限委托 / 角色关系 / AI 模型版本 / 生效中委托：后端 ApiResponse::success(Vec) → data 为裸数组
const unwrapDataArray = <T,>(p: unknown): T[] => (p as { data: T[] }).data;
// 设备连接：后端 ApiResponse::success(PaginatedResponse) → data.items
const unwrapPagedItems = <T,>(p: unknown): T[] => (p as { data: { items: T[] } }).data.items;
// 决策日志 / 角色变更审批：调用方已传入 res.data（数组或 {items}），沿用既有解包
const unwrapList = <T,>(p: unknown): T[] =>
  Array.isArray(p) ? p : ((p as { items?: T[] })?.items ?? []);

const cols = (rows: Array<Record<string, unknown>>, skip: string[], n: number) =>
  rows.length
    ? Object.keys(rows[0])
        .filter(k => !skip.includes(k) && typeof rows[0][k] !== 'object')
        .slice(0, n)
    : [];

// 委托
const delegations = ref<Array<Record<string, unknown>>>([]);
const delegationCols = ref<string[]>([]);
const loading = ref(false);

async function loadDelegations() {
  loading.value = true;
  try {
    delegations.value = unwrapDataArray(await getDelegationList());
    delegationCols.value = cols(delegations.value, ['id'], 6);
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  } finally {
    loading.value = false;
  }
}

async function onExpireOverdue() {
  try {
    const res = await expireOverdueDelegations();
    ElMessage.success(`已清理过期委托：${JSON.stringify(res).slice(0, 60)}`);
    await loadDelegations();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  }
}

async function onRevoke(row: Record<string, unknown>) {
  try {
    await ElMessageBox.confirm('确认撤销该委托？', '确认', { type: 'warning' });
  } catch {
    return; // 用户取消不是失败，静默返回（rejection 已捕获）
  }
  try {
    await revokeDelegation(row.id as number);
    ElMessage.success('已撤销');
    await loadDelegations();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  }
}

// 委托详情弹窗（复用 detail-json 展示）
const detailDialogVisible = ref(false);
const detailJson = ref('');

async function showDetail(data: unknown) {
  detailJson.value = JSON.stringify(data, null, 2);
  detailDialogVisible.value = true;
}

async function onDelegationDetail(row: Record<string, unknown>) {
  try {
    const res = (await getDelegation(Number(row.id))) as ApiResponse<unknown>;
    await showDetail(res.data ?? res);
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  }
}

// 生效中委托：按受托人查询 active 状态委托
async function onLoadActiveDelegations() {
  let delegateeId = 0;
  try {
    const { value } = await ElMessageBox.prompt('请输入受托人用户ID', '生效中委托', {
      inputPattern: /^\d+$/,
      inputErrorMessage: '请输入数字ID',
    });
    delegateeId = Number(value);
  } catch {
    return;
  }
  loading.value = true;
  try {
    const res = (await getActiveDelegatedPermissions(delegateeId)) as ApiResponse<unknown>;
    delegations.value = unwrapDataArray(res);
    ElMessage.success(`受托人 ${delegateeId} 的生效中委托已加载`);
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  } finally {
    loading.value = false;
  }
}

// 新建委托
const delegationDialogVisible = ref(false);
// 后端 CreateDelegationRequest 键集：delegator_id/delegatee_id/permission_code/
// valid_from/valid_until/reason；委托人=当前登录用户（真实取值，非手填）
const delegationForm = reactive({
  delegatee_id: 1,
  permission_code: '',
  valid_from: '',
  valid_until: '',
  reason: '',
});

async function onSaveDelegation() {
  if (!delegationForm.delegatee_id || !delegationForm.permission_code.trim()) {
    ElMessage.warning(t('systemGovernance.delegation.fillDelegateePermission'));
    return;
  }
  if (!delegationForm.valid_from || !delegationForm.valid_until) {
    ElMessage.warning(t('systemGovernance.delegation.fillTimeRange'));
    return;
  }
  // 后端 DateTime<Utc> 反序列化要求 RFC3339：本地时间字符串显式转 UTC ISO
  const validFrom = new Date(delegationForm.valid_from);
  const validUntil = new Date(delegationForm.valid_until);
  if (Number.isNaN(validFrom.getTime()) || Number.isNaN(validUntil.getTime())) {
    ElMessage.warning(t('systemGovernance.delegation.invalidTimeFormat'));
    return;
  }
  if (validUntil <= validFrom) {
    ElMessage.warning(t('systemGovernance.delegation.endBeforeStart'));
    return;
  }
  const delegatorId = userStore.userInfo?.id;
  if (!delegatorId) {
    ElMessage.warning(t('systemGovernance.delegation.noCurrentUser'));
    return;
  }
  try {
    // 载荷键集与后端 CreateDelegationRequest 逐字一致；可选 reason 空则省略键
    // （条件赋值而非 `|| undefined` 兜底，见 Option 空值省略键范式）
    const payload: CreatePermissionDelegationPayload = {
      delegator_id: delegatorId,
      delegatee_id: delegationForm.delegatee_id,
      permission_code: delegationForm.permission_code.trim(),
      valid_from: validFrom.toISOString(),
      valid_until: validUntil.toISOString(),
    };
    if (delegationForm.reason.trim()) payload.reason = delegationForm.reason.trim();
    await createPermissionDelegation(payload);
    ElMessage.success(t('systemGovernance.delegation.created'));
    delegationDialogVisible.value = false;
    await loadDelegations();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  }
}

// 角色关系
const relations = ref<Array<Record<string, unknown>>>([]);
const relationCols = ref<string[]>([]);
const loadingRelations = ref(false);
const relationDialogVisible = ref(false);
const relationForm = reactive({ role_a: '', role_b: '', relation_type: 'mutual_exclusive' });

async function loadRelations() {
  loadingRelations.value = true;
  try {
    relations.value = unwrapDataArray(await getRoleRelations());
    relationCols.value = cols(relations.value, ['id'], 6);
  } finally {
    loadingRelations.value = false;
  }
}

async function onCreateRelation() {
  if (!relationForm.role_a || !relationForm.role_b) {
    ElMessage.warning('请填写两个角色');
    return;
  }
  await createRoleRelation({ ...relationForm });
  ElMessage.success('关系已创建');
  relationDialogVisible.value = false;
  await loadRelations();
}

async function onDeleteRelation(row: Record<string, unknown>) {
  await ElMessageBox.confirm('确认删除该角色关系？', '确认');
  await deleteRoleRelation(row.id as number);
  ElMessage.success('已删除');
  await loadRelations();
}

// AI 模型
const modelVersions = ref<Array<Record<string, unknown>>>([]);
const modelCols = ref<string[]>([]);
const loadingModels = ref(false);
const aiResult = ref('');
const versionDialogVisible = ref(false);
const versionForm = reactive({ model_name: '', version: '', description: '' });

async function loadModels() {
  loadingModels.value = true;
  try {
    modelVersions.value = unwrapDataArray(await getModelVersionList());
    modelCols.value = cols(modelVersions.value, ['id'], 7);
  } finally {
    loadingModels.value = false;
  }
}

async function onCreateVersion() {
  if (!versionForm.model_name || !versionForm.version) {
    ElMessage.warning('请填写模型名与版本号');
    return;
  }
  await createModelVersion({ ...versionForm });
  ElMessage.success('版本已注册');
  versionDialogVisible.value = false;
  await loadModels();
}

async function onApprove(row: Record<string, unknown>) {
  await ElMessageBox.confirm('确认审批通过该版本？', '确认');
  await approveModelVersion(row.id as number);
  ElMessage.success('已审批');
  await loadModels();
}

async function onDrift(row: Record<string, unknown>) {
  aiResult.value = JSON.stringify(await detectModelDrift(row.id as number), null, 2);
}

async function onEvaluations(row: Record<string, unknown>) {
  aiResult.value = JSON.stringify(await getModelEvaluations(row.id as number), null, 2);
}

async function onReconcile() {
  aiResult.value = JSON.stringify(await reconcileMonthly(), null, 2);
}

async function onAccuracyReports() {
  aiResult.value = JSON.stringify(await getAccuracyReports(), null, 2);
}

// 设备
const devices = ref<Array<Record<string, unknown>>>([]);
const deviceCols = ref<string[]>([]);
const loadingDevices = ref(false);
const onlineCount = ref(0);
const deviceDialogVisible = ref(false);
const deviceForm = reactive({ device_code: '', device_name: '' });

async function loadDevices() {
  loadingDevices.value = true;
  try {
    const [list, count] = await Promise.all([getDeviceList(), getOnlineDeviceCount()]);
    devices.value = unwrapPagedItems(list);
    deviceCols.value = cols(devices.value, ['id'], 7);
    const c = count as { count?: number; data?: { count?: number } };
    onlineCount.value = c?.count ?? c?.data?.count ?? 0;
  } finally {
    loadingDevices.value = false;
  }
}

async function onRegisterDevice() {
  if (!deviceForm.device_code) {
    ElMessage.warning('请填写设备编码');
    return;
  }
  await registerDevice({ ...deviceForm });
  ElMessage.success('设备已注册');
  deviceDialogVisible.value = false;
  await loadDevices();
}

// 设备详情
async function onDeviceDetail(row: Record<string, unknown>) {
  try {
    const res = (await getDeviceConnection(String(row.id))) as ApiResponse<unknown>;
    await showDetail(res.data ?? res);
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  }
}

// 设备心跳
async function onDeviceHeartbeat(row: Record<string, unknown>) {
  try {
    await sendDeviceHeartbeat(String(row.id));
    ElMessage.success(`设备 #${row.id} 心跳已发送`);
    await loadDevices();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  }
}

// 断开设备
async function onDisconnectDevice(row: Record<string, unknown>) {
  try {
    await ElMessageBox.confirm(`确认断开设备 #${row.id}？`, '确认', { type: 'warning' });
  } catch {
    return;
  }
  try {
    await disconnectDevice(String(row.id));
    ElMessage.success('设备已断开');
    await loadDevices();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  }
}

// 清理超时设备
async function onCleanupTimeoutDevices() {
  try {
    await ElMessageBox.confirm('确认清理所有超时设备连接？', '确认', { type: 'warning' });
  } catch {
    return;
  }
  try {
    await cleanupTimeoutDevices();
    ElMessage.success('超时设备已清理');
    await loadDevices();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  }
}

// 模型版本状态变更（激活/弃用）
const onModelStatus = async (row: Record<string, unknown>, status: string) => {
  try {
    await changeModelStatus(Number(row.id), { status });
    ElMessage.success(`模型版本已${status === 'active' ? '激活' : '弃用'}`);
    loadModels();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  }
};

// 模型评估登记（对选中版本创建评估记录）
const onModelEvaluation = async (row?: Record<string, unknown>) => {
  const versionId = row ? Number(row.id) : modelVersions.value[0]?.id;
  if (!versionId) {
    ElMessage.warning('暂无模型版本');
    return;
  }
  try {
    await createModelEvaluation({
      model_version_id: versionId,
      evaluation_type: 'periodic',
      result: { note: 'manual evaluation' },
    });
    ElMessage.success('评估记录已创建');
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  }
};

// 决策日志查询（弹窗展示）
const onDecisionLogs = async () => {
  try {
    const res = (await getDecisionLogs({})) as ApiResponse<unknown>;
    const logs = unwrapList<Record<string, unknown>>(res.data);
    const lines = logs
      .slice(0, 20)
      .map(l => `#${l.id} ${l.decision_type || ''} ${l.created_at || ''}`);
    ElMessageBox.alert(lines.join('\n') || '暂无决策日志', 'AI 决策日志（最近 20 条）', {
      type: 'info',
    });
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  }
};

// 角色变更审批：列表 + L1/L2 审批/驳回/撤销
const roleChanges = ref<Record<string, unknown>[]>([]);
const roleChangeLoading = ref(false);
// 列键取后端 role_change_approvals::Model 真实键（role_id/reason 不存在，恒空白已修）
const roleChangeCols = [
  'id',
  'approval_no',
  'change_type',
  'target_role_id',
  'target_role_code',
  'status',
  'created_at',
];

const loadRoleChanges = async () => {
  roleChangeLoading.value = true;
  try {
    const res = (await getRoleChangeApprovals({})) as ApiResponse<unknown>;
    roleChanges.value = unwrapList<Record<string, unknown>>(res.data);
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  } finally {
    roleChangeLoading.value = false;
  }
};

// 新建角色变更审批
const roleChangeDialogVisible = ref(false);
// 键集与后端 CreateRoleChangeApprovalRequest 逐字一致：
// change_type/target_role_id/target_role_code 必填；无 reason（后端缺口，见串行清单）
// change_type 源保持 string 供 el-select v-model 回写，提交前用守卫收窄（禁止裸断言）
const ROLE_CHANGE_TYPES: readonly RoleChangeType[] = [
  'assign_role',
  'assign_permission',
  'remove_permission',
];
const isRoleChangeType = (v: string): v is RoleChangeType =>
  (ROLE_CHANGE_TYPES as readonly string[]).includes(v);

const roleChangeForm = reactive({
  change_type: 'assign_role',
  target_role_id: 1,
  target_role_code: '',
  target_user_id: undefined as number | undefined,
});

const onSaveRoleChange = async () => {
  if (!roleChangeForm.target_role_code.trim()) {
    ElMessage.warning(t('systemGovernance.roleChange.fillTargetRoleCode'));
    return;
  }
  const changeType = roleChangeForm.change_type;
  if (!isRoleChangeType(changeType)) {
    ElMessage.warning(t('systemGovernance.roleChange.invalidChangeType'));
    return;
  }
  try {
    await createRoleChangeApproval({
      change_type: changeType,
      target_role_id: roleChangeForm.target_role_id,
      target_role_code: roleChangeForm.target_role_code.trim(),
      target_user_id: roleChangeForm.target_user_id,
    });
    ElMessage.success(t('systemGovernance.roleChange.created'));
    roleChangeDialogVisible.value = false;
    await loadRoleChanges();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  }
};

// 审批单详情
const onRoleChangeDetail = async (row: Record<string, unknown>) => {
  try {
    const res = (await getRoleChangeApproval(Number(row.id))) as ApiResponse<unknown>;
    await showDetail(res.data ?? res);
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  }
};

const onRoleChangeAction = async (
  row: Record<string, unknown>,
  action: 'approve-l1' | 'approve-l2' | 'reject' | 'cancel'
) => {
  let reason = '';
  if (action === 'reject') {
    try {
      const { value } = await ElMessageBox.prompt('请输入驳回原因', `驳回 #${row.id}`, {
        type: 'warning',
        inputPattern: /\S+/,
        inputErrorMessage: '驳回原因不能为空',
      });
      reason = value;
    } catch {
      return;
    }
  }
  try {
    if (action === 'approve-l1') await approveRoleChangeL1(Number(row.id));
    else if (action === 'approve-l2') await approveRoleChangeL2(Number(row.id));
    else if (action === 'reject')
      // 后端 reject/approve 共用 ApproveRoleChangeRequest，意见键名 comments
      //（原键 opinion 后端不存在，驳回原因被 serde 静默丢弃）
      await rejectRoleChangeApproval(Number(row.id), { comments: reason });
    else await cancelRoleChangeApproval(Number(row.id));
    ElMessage.success(t('common.success'));
    loadRoleChanges();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  }
};

onMounted(() => {
  loadDelegations();
  loadRelations();
  loadModels();
  loadDevices();
});
</script>

<style scoped>
.mb {
  margin-bottom: 12px;
}
.toolbar {
  display: flex;
  gap: 8px;
  align-items: center;
}
.w-full {
  width: 100%;
}
.result-box {
  background: var(--el-fill-color-light);
  border-radius: 4px;
  padding: 12px;
  font-size: 12px;
  max-height: 320px;
  overflow: auto;
  white-space: pre-wrap;
  margin-top: 12px;
}
.detail-json {
  background: var(--el-fill-color-light);
  border-radius: 4px;
  padding: 12px;
  font-size: 12px;
  max-height: 420px;
  overflow: auto;
  white-space: pre-wrap;
  margin: 0;
}
</style>

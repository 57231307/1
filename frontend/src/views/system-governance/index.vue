<template>
  <div class="page">
    <el-tabs v-model="activeTab" type="border-card">
      <!-- 权限委托 -->
      <el-tab-pane label="权限委托" name="delegation">
        <div class="toolbar mb">
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
          <el-table-column label="操作" width="120" fixed="right">
            <template #default="{ row }">
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
          <el-table-column label="操作" width="300" fixed="right">
            <template #default="{ row }">
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
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
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
  logDecision,
  getDecisionLogs,
  getRoleChangeApprovals,
  approveRoleChangeL1,
  approveRoleChangeL2,
  rejectRoleChangeApproval,
  cancelRoleChangeApproval,
  checkMutualExclusive,
} from '@/api/system-governance';

const activeTab = ref('delegation');
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
    delegations.value = unwrapList(await getDelegationList());
    delegationCols.value = cols(delegations.value, ['id'], 6);
  } finally {
    loading.value = false;
  }
}

async function onExpireOverdue() {
  const res = await expireOverdueDelegations();
  ElMessage.success(`已清理过期委托：${JSON.stringify(res).slice(0, 60)}`);
  await loadDelegations();
}

async function onRevoke(row: Record<string, unknown>) {
  await ElMessageBox.confirm('确认撤销该委托？', '确认');
  await revokeDelegation(row.id as number);
  ElMessage.success('已撤销');
  await loadDelegations();
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
    relations.value = unwrapList(await getRoleRelations());
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
    modelVersions.value = unwrapList(await getModelVersionList());
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
    devices.value = unwrapList(list);
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
    const res = await getDecisionLogs({});
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
const roleChangeCols = ['id', 'role_id', 'status', 'reason', 'created_at'];

const loadRoleChanges = async () => {
  roleChangeLoading.value = true;
  try {
    const res = await getRoleChangeApprovals({});
    roleChanges.value = unwrapList<Record<string, unknown>>(res.data);
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  } finally {
    roleChangeLoading.value = false;
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
    else if (action === 'reject') await rejectRoleChangeApproval(Number(row.id), reason);
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
</style>

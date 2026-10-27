<!--
  系统治理中心页（路由 /system-governance）
  调用方：MainLayout 侧栏 系统管理 分组下的 系统治理中心 菜单项
  职责：权限委托、角色关系、AI 模型版本治理、角色变更审批、设备连接五个治理面
  展示文案全部走 i18n（中英双侧同源）；表格列按下后端各 Model 真实键集显式定义并逐列挂词条，
  不再用列名转下划线或 Object.keys 截断自动派生（那会把 snake_case 原始列名与漂移列直接露到界面）；
  写操作载荷键集逐字对齐各 handler 请求 DTO，缺失即后端 422，故每个创建/流转都显式补齐必填键并捕获失败。
-->
<template>
  <div class="page">
    <el-tabs v-model="activeTab" type="border-card">
      <!-- 权限委托 -->
      <el-tab-pane :label="t('systemGovernance.tab.delegation')" name="delegation">
        <div class="toolbar mb">
          <el-button type="primary" @click="openDelegationDialog">{{
            t('systemGovernance.button.newDelegation')
          }}</el-button>
          <el-button plain @click="loadDelegations">{{
            t('systemGovernance.button.allDelegations')
          }}</el-button>
          <el-button plain @click="onLoadActiveDelegations">{{
            t('systemGovernance.button.activeDelegations')
          }}</el-button>
          <el-button plain @click="onExpireOverdue">{{
            t('systemGovernance.button.expireOverdue')
          }}</el-button>
        </div>
        <el-table v-loading="loading" :data="delegations" border>
          <el-table-column
            v-for="col in delegationCols"
            :key="col.prop"
            :prop="col.prop"
            :label="t(col.labelKey)"
            :width="col.width"
            :min-width="col.width ? undefined : 140"
            show-overflow-tooltip
          >
            <template #default="{ row }">{{ renderCell(col, row) }}</template>
          </el-table-column>
          <el-table-column :label="t('systemGovernance.column.action')" width="180" fixed="right">
            <template #default="{ row }">
              <el-button size="small" @click="onDelegationDetail(row)">{{
                t('systemGovernance.button.detail')
              }}</el-button>
              <el-button size="small" type="danger" plain @click="onRevoke(row)">{{
                t('systemGovernance.button.revoke')
              }}</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <!-- 角色关系 -->
      <el-tab-pane :label="t('systemGovernance.tab.roleRelation')" name="role-relation">
        <div class="toolbar mb">
          <el-button type="primary" @click="relationDialogVisible = true">{{
            t('systemGovernance.button.newRelation')
          }}</el-button>
        </div>
        <el-table v-loading="loadingRelations" :data="relations" border>
          <el-table-column
            v-for="col in relationCols"
            :key="col.prop"
            :prop="col.prop"
            :label="t(col.labelKey)"
            :width="col.width"
            :min-width="col.width ? undefined : 140"
            show-overflow-tooltip
          >
            <template #default="{ row }">{{ renderCell(col, row) }}</template>
          </el-table-column>
          <el-table-column :label="t('systemGovernance.column.action')" width="120" fixed="right">
            <template #default="{ row }">
              <el-button size="small" type="danger" plain @click="onDeleteRelation(row)">{{
                t('systemGovernance.button.delete')
              }}</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <!-- AI 模型治理 -->
      <el-tab-pane :label="t('systemGovernance.tab.aiModel')" name="ai">
        <div class="toolbar mb">
          <el-button type="primary" @click="versionDialogVisible = true">{{
            t('systemGovernance.button.newModelVersion')
          }}</el-button>
          <el-button plain @click="onReconcile">{{
            t('systemGovernance.button.reconcile')
          }}</el-button>
          <el-button plain @click="onAccuracyReports">{{
            t('systemGovernance.button.accuracyReports')
          }}</el-button>
          <el-button plain @click="onModelEvaluation()">{{
            t('systemGovernance.button.modelEvaluation')
          }}</el-button>
          <el-button plain @click="onDecisionLogs">{{
            t('systemGovernance.button.decisionLogs')
          }}</el-button>
        </div>
        <el-table v-loading="loadingModels" :data="modelVersions" border>
          <el-table-column
            v-for="col in modelCols"
            :key="col.prop"
            :prop="col.prop"
            :label="t(col.labelKey)"
            :width="col.width"
            :min-width="col.width ? undefined : 140"
            show-overflow-tooltip
          >
            <template #default="{ row }">{{ renderCell(col, row) }}</template>
          </el-table-column>
          <el-table-column :label="t('systemGovernance.column.action')" width="340" fixed="right">
            <template #default="{ row }">
              <el-button size="small" type="success" @click="onApprove(row)">{{
                t('systemGovernance.button.approve')
              }}</el-button>
              <el-button size="small" type="primary" plain @click="onDrift(row)">{{
                t('systemGovernance.button.drift')
              }}</el-button>
              <el-button size="small" @click="onEvaluations(row)">{{
                t('systemGovernance.button.evaluations')
              }}</el-button>
              <el-button size="small" type="warning" plain @click="onModelStatus(row, 'active')">{{
                t('systemGovernance.button.activate')
              }}</el-button>
              <el-button size="small" type="info" plain @click="onModelStatus(row, 'retired')">{{
                t('systemGovernance.button.retire')
              }}</el-button>
            </template>
          </el-table-column>
        </el-table>
        <pre v-if="aiResult" class="result-box">{{ aiResult }}</pre>
      </el-tab-pane>

      <!-- 角色变更审批 -->
      <el-tab-pane :label="t('systemGovernance.tab.roleChange')" name="role-change">
        <div class="toolbar mb">
          <el-button type="primary" :loading="roleChangeLoading" @click="loadRoleChanges">{{
            t('systemGovernance.button.refresh')
          }}</el-button>
          <el-button type="primary" plain @click="roleChangeDialogVisible = true">{{
            t('systemGovernance.button.newApproval')
          }}</el-button>
        </div>
        <el-table v-loading="roleChangeLoading" :data="roleChanges" border>
          <el-table-column
            v-for="col in roleChangeCols"
            :key="col.prop"
            :prop="col.prop"
            :label="t(col.labelKey)"
            :width="col.width"
            :min-width="col.width ? undefined : 130"
            show-overflow-tooltip
          >
            <template #default="{ row }">{{ renderCell(col, row) }}</template>
          </el-table-column>
          <el-table-column :label="t('systemGovernance.column.action')" width="360" fixed="right">
            <template #default="{ row }">
              <el-button size="small" @click="onRoleChangeDetail(row)">{{
                t('systemGovernance.button.detail')
              }}</el-button>
              <el-button
                v-if="row.status === 'pending_l1'"
                size="small"
                type="success"
                @click="onRoleChangeAction(row, 'approve-l1')"
                >{{ t('systemGovernance.button.approveL1') }}</el-button
              >
              <el-button
                v-if="row.status === 'pending_l2'"
                size="small"
                type="success"
                @click="onRoleChangeAction(row, 'approve-l2')"
                >{{ t('systemGovernance.button.approveL2') }}</el-button
              >
              <el-button
                v-if="row.status === 'pending_l1' || row.status === 'pending_l2'"
                size="small"
                type="danger"
                @click="onRoleChangeAction(row, 'reject')"
                >{{ t('systemGovernance.button.reject') }}</el-button
              >
              <el-button size="small" @click="onRoleChangeAction(row, 'cancel')">{{
                t('systemGovernance.button.cancelApproval')
              }}</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <!-- 设备连接 -->
      <el-tab-pane :label="t('systemGovernance.tab.device')" name="device">
        <div class="toolbar mb">
          <el-tag>{{ t('systemGovernance.label.onlineDevices') }}{{ onlineCount }}</el-tag>
          <el-button type="primary" @click="deviceDialogVisible = true">{{
            t('systemGovernance.button.registerDevice')
          }}</el-button>
          <el-button plain @click="onCleanupTimeoutDevices">{{
            t('systemGovernance.button.cleanupTimeout')
          }}</el-button>
        </div>
        <el-table v-loading="loadingDevices" :data="devices" border>
          <el-table-column
            v-for="col in deviceCols"
            :key="col.prop"
            :prop="col.prop"
            :label="t(col.labelKey)"
            :width="col.width"
            :min-width="col.width ? undefined : 140"
            show-overflow-tooltip
          >
            <template #default="{ row }">{{ renderCell(col, row) }}</template>
          </el-table-column>
          <el-table-column :label="t('systemGovernance.column.action')" width="220" fixed="right">
            <template #default="{ row }">
              <el-button size="small" @click="onDeviceDetail(row)">{{
                t('systemGovernance.button.detail')
              }}</el-button>
              <el-button size="small" type="success" plain @click="onDeviceHeartbeat(row)">{{
                t('systemGovernance.button.heartbeat')
              }}</el-button>
              <el-button size="small" type="warning" plain @click="onDisconnectDevice(row)">{{
                t('systemGovernance.button.disconnect')
              }}</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>
    </el-tabs>

    <el-dialog
      v-model="relationDialogVisible"
      :title="t('systemGovernance.dialog.newRelation')"
      width="460px"
    >
      <el-form :model="relationForm" label-width="110px">
        <!-- 键集逐字对齐后端 CreateRoleRelationRequest：parent_role_code/child_role_code/
             relation_type 必填、description 可选（原 role_a/role_b 后端无此键，serde 缺必填即 422） -->
        <el-form-item :label="t('systemGovernance.form.parentRoleCode')" required>
          <el-input v-model="relationForm.parent_role_code" />
        </el-form-item>
        <el-form-item :label="t('systemGovernance.form.childRoleCode')" required>
          <el-input v-model="relationForm.child_role_code" />
        </el-form-item>
        <el-form-item :label="t('systemGovernance.form.relationType')" required>
          <el-select v-model="relationForm.relation_type" class="w-full">
            <el-option
              :label="t('systemGovernance.relationType.mutualExclusive')"
              value="mutual_exclusive"
            />
            <el-option
              :label="t('systemGovernance.relationType.inheritance')"
              value="inheritance"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('systemGovernance.form.description')">
          <el-input v-model="relationForm.description" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="relationDialogVisible = false">{{
          t('systemGovernance.button.cancel')
        }}</el-button>
        <el-button type="primary" @click="onCreateRelation">{{
          t('systemGovernance.button.save')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="versionDialogVisible"
      :title="t('systemGovernance.dialog.newModelVersion')"
      width="480px"
    >
      <el-form :model="versionForm" label-width="110px">
        <!-- 键集对齐后端 CreateModelVersionRequest：model_name/version/algorithm 必填、
             change_reason 可选（原 description 后端无此键，算法漏填即 422） -->
        <el-form-item :label="t('systemGovernance.form.modelName')" required>
          <el-input v-model="versionForm.model_name" />
        </el-form-item>
        <el-form-item :label="t('systemGovernance.form.version')" required>
          <el-input v-model="versionForm.version" />
        </el-form-item>
        <el-form-item :label="t('systemGovernance.form.algorithm')" required>
          <el-input v-model="versionForm.algorithm" />
        </el-form-item>
        <el-form-item :label="t('systemGovernance.form.changeReason')">
          <el-input v-model="versionForm.change_reason" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="versionDialogVisible = false">{{
          t('systemGovernance.button.cancel')
        }}</el-button>
        <el-button type="primary" @click="onCreateVersion">{{
          t('systemGovernance.button.save')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="deviceDialogVisible"
      :title="t('systemGovernance.dialog.registerDevice')"
      width="460px"
    >
      <el-form :model="deviceForm" label-width="110px">
        <!-- 键集对齐后端 RegisterDeviceRequest：device_id 必填（原 device_code 后端无此键）、
             device_type 必填（默认 other）、device_name 可选 -->
        <el-form-item :label="t('systemGovernance.form.deviceId')" required>
          <el-input v-model="deviceForm.device_id" />
        </el-form-item>
        <el-form-item :label="t('systemGovernance.form.deviceName')">
          <el-input v-model="deviceForm.device_name" />
        </el-form-item>
        <el-form-item :label="t('systemGovernance.form.deviceType')" required>
          <el-select v-model="deviceForm.device_type" class="w-full">
            <el-option :label="t('systemGovernance.deviceType.pda')" value="pda" />
            <el-option
              :label="t('systemGovernance.deviceType.industrialTerminal')"
              value="industrial_terminal"
            />
            <el-option :label="t('systemGovernance.deviceType.scanner')" value="scanner" />
            <el-option :label="t('systemGovernance.deviceType.other')" value="other" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="deviceDialogVisible = false">{{
          t('systemGovernance.button.cancel')
        }}</el-button>
        <el-button type="primary" @click="onRegisterDevice">{{
          t('systemGovernance.button.save')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="delegationDialogVisible"
      :title="t('systemGovernance.dialog.newDelegation')"
      width="480px"
    >
      <el-form :model="delegationForm" label-width="110px">
        <!-- 后端 CreateDelegationRequest：delegator 取当前登录用户（谁委托自己的权限），
             一条委托仅一个 permission_code（String 列），时间必填 RFC3339 -->
        <el-form-item :label="t('systemGovernance.form.delegator')">
          <span>{{ delegationDelegatorText }}</span>
        </el-form-item>
        <el-form-item :label="t('systemGovernance.form.delegateeId')" required>
          <!-- 数值录入禁步进控件：用 el-input 承接文本，提交前按正整数校验，范围靠校验器不靠加减号 -->
          <el-input v-model="delegationForm.delegatee_id" inputmode="numeric" />
        </el-form-item>
        <el-form-item :label="t('systemGovernance.form.permissionCode')" required>
          <el-input
            v-model="delegationForm.permission_code"
            :placeholder="t('systemGovernance.form.permissionCodePlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="t('systemGovernance.form.validFrom')" required>
          <el-date-picker
            v-model="delegationForm.valid_from"
            type="datetime"
            value-format="YYYY-MM-DDTHH:mm:ss"
            class="w-full"
          />
        </el-form-item>
        <el-form-item :label="t('systemGovernance.form.validUntil')" required>
          <el-date-picker
            v-model="delegationForm.valid_until"
            type="datetime"
            value-format="YYYY-MM-DDTHH:mm:ss"
            class="w-full"
          />
        </el-form-item>
        <el-form-item :label="t('systemGovernance.form.reason')">
          <el-input v-model="delegationForm.reason" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="delegationDialogVisible = false">{{
          t('systemGovernance.button.cancel')
        }}</el-button>
        <el-button type="primary" @click="onSaveDelegation">{{
          t('systemGovernance.button.save')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="roleChangeDialogVisible"
      :title="t('systemGovernance.dialog.newRoleChange')"
      width="500px"
    >
      <el-form :model="roleChangeForm" label-width="120px">
        <!-- change_type 取值域=后端服务层白名单（越界 422）；target_role_code 为非 Option 必填；
             后端 DTO/审批表均无 reason 字段，变更原因输入被丢弃（缺失已登记串行清单），故移除 -->
        <el-form-item :label="t('systemGovernance.form.changeType')" required>
          <el-select v-model="roleChangeForm.change_type" class="w-full">
            <el-option :label="t('systemGovernance.changeType.assignRole')" value="assign_role" />
            <el-option
              :label="t('systemGovernance.changeType.assignPermission')"
              value="assign_permission"
            />
            <el-option
              :label="t('systemGovernance.changeType.removePermission')"
              value="remove_permission"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('systemGovernance.form.targetRoleId')" required>
          <el-input v-model="roleChangeForm.target_role_id" inputmode="numeric" />
        </el-form-item>
        <el-form-item :label="t('systemGovernance.form.targetRoleCode')" required>
          <el-input
            v-model="roleChangeForm.target_role_code"
            :placeholder="t('systemGovernance.form.targetRoleCodePlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="t('systemGovernance.form.targetUserId')">
          <el-input v-model="roleChangeForm.target_user_id" inputmode="numeric" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="roleChangeDialogVisible = false">{{
          t('systemGovernance.button.cancel')
        }}</el-button>
        <el-button type="primary" @click="onSaveRoleChange">{{
          t('systemGovernance.button.submit')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="detailDialogVisible"
      :title="t('systemGovernance.dialog.detail')"
      width="560px"
    >
      <pre class="detail-json">{{ detailJson }}</pre>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref, computed } from 'vue';
import { logger, logAuxLoadFailure } from '@/utils/logger';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { isDialogDismissal, rethrowNonDismissal } from '@/utils/monitor';
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

/**
 * 表格列描述符。prop=后端 Model 真实键；labelKey=列头 i18n 键；
 * tokenLabels=枚举列的「后端 token → 完整 i18n 展示键」映射（词表外原样显示），
 * 非枚举列留空原样显示；width=定宽（时间/枚举列给宽，避免挤压换行）。
 * 列的集合与次序由本表显式钉死，不再 Object.keys(首行).slice(0,n)，杜绝序列化顺序漂移把
 * 未翻译的 snake_case 原名或随机截断的列露到界面。
 * 枚举映射键写成完整字面量（不用命名空间动态拼接），让每个可见文案都落到真实词条、
 * 可被 i18n 检查静态判定，缺失即在构建期暴露而非运行时露原始键串。
 */
interface ColumnDef {
  prop: string;
  labelKey: string;
  tokenLabels?: Record<string, string>;
  width?: number;
}

/**
 * 单元格取值渲染：枚举列按后端 token 查显式映射表得展示文案（词表外原样显示并 warn 一次），
 * 时间列原样、空值显示占位破折号。
 * 入参：列描述符与行对象；出参：用于渲染的字符串。
 */
const renderCell = (col: ColumnDef, row: Record<string, unknown>): string => {
  const raw = row[col.prop];
  if (raw === null || raw === undefined || raw === '') return '-';
  if (col.tokenLabels) {
    const token = String(raw);
    const labelKey = col.tokenLabels[token];
    if (!labelKey) {
      warnEnumTokenOnce(
        `${col.prop}:${token}`,
        `${t('systemGovernance.msg.unknownToken')}${token}`
      );
      return token;
    }
    return t(labelKey);
  }
  return String(raw);
};

// 同一 token 只点名一次，避免整表逐行刷屏把真实漂移淹掉
const warnedTokens = new Set<string>();
const warnEnumTokenOnce = (mark: string, message: string): void => {
  if (warnedTokens.has(mark)) return;
  warnedTokens.add(mark);
  logger.warn(message);
};

// 委托列：id + 委托人/受托人/权限码/生效区间/状态/创建时间（对齐 models/permission_delegation.rs）
const delegationCols: ColumnDef[] = [
  { prop: 'id', labelKey: 'systemGovernance.column.id', width: 80 },
  { prop: 'delegator_id', labelKey: 'systemGovernance.column.delegatorId', width: 100 },
  { prop: 'delegatee_id', labelKey: 'systemGovernance.column.delegateeId', width: 100 },
  { prop: 'permission_code', labelKey: 'systemGovernance.column.permissionCode' },
  { prop: 'valid_from', labelKey: 'systemGovernance.column.validFrom', width: 180 },
  { prop: 'valid_until', labelKey: 'systemGovernance.column.validUntil', width: 180 },
  {
    prop: 'status',
    labelKey: 'systemGovernance.column.status',
    tokenLabels: {
      pending: 'systemGovernance.delegationStatus.pending',
      active: 'systemGovernance.delegationStatus.active',
      expired: 'systemGovernance.delegationStatus.expired',
      revoked: 'systemGovernance.delegationStatus.revoked',
    },
    width: 120,
  },
  { prop: 'created_at', labelKey: 'systemGovernance.column.createdAt', width: 180 },
];

// 角色关系列：对齐 models/role_relation.rs（parent_role_code/child_role_code/relation_type/description/created_at）
const relationCols: ColumnDef[] = [
  { prop: 'id', labelKey: 'systemGovernance.column.id', width: 80 },
  { prop: 'parent_role_code', labelKey: 'systemGovernance.column.parentRoleCode' },
  { prop: 'child_role_code', labelKey: 'systemGovernance.column.childRoleCode' },
  {
    prop: 'relation_type',
    labelKey: 'systemGovernance.column.relationType',
    tokenLabels: {
      mutual_exclusive: 'systemGovernance.relationTypeToken.mutual_exclusive',
      inheritance: 'systemGovernance.relationTypeToken.inheritance',
    },
    width: 140,
  },
  { prop: 'description', labelKey: 'systemGovernance.column.description' },
  { prop: 'created_at', labelKey: 'systemGovernance.column.createdAt', width: 180 },
];

// 模型版本列：对齐 models/ai_model_version.rs（json 大字段不进主表，详情弹窗看全量）
const modelCols: ColumnDef[] = [
  { prop: 'id', labelKey: 'systemGovernance.column.id', width: 80 },
  { prop: 'model_name', labelKey: 'systemGovernance.column.modelName' },
  { prop: 'version', labelKey: 'systemGovernance.column.version', width: 120 },
  { prop: 'algorithm', labelKey: 'systemGovernance.column.algorithm', width: 140 },
  {
    prop: 'status',
    labelKey: 'systemGovernance.column.status',
    tokenLabels: {
      draft: 'systemGovernance.modelStatus.draft',
      active: 'systemGovernance.modelStatus.active',
      retired: 'systemGovernance.modelStatus.retired',
      archived: 'systemGovernance.modelStatus.archived',
    },
    width: 120,
  },
  {
    prop: 'approval_status',
    labelKey: 'systemGovernance.column.approvalStatus',
    tokenLabels: {
      pending: 'systemGovernance.approvalStatus.pending',
      approved: 'systemGovernance.approvalStatus.approved',
      rejected: 'systemGovernance.approvalStatus.rejected',
    },
    width: 130,
  },
  { prop: 'created_at', labelKey: 'systemGovernance.column.createdAt', width: 180 },
];

// 设备列：对齐 device_connection handler 的 DeviceConnectionInfo
const deviceCols: ColumnDef[] = [
  { prop: 'id', labelKey: 'systemGovernance.column.id', width: 80 },
  { prop: 'device_id', labelKey: 'systemGovernance.column.deviceId' },
  { prop: 'device_name', labelKey: 'systemGovernance.column.deviceName' },
  {
    prop: 'device_type',
    labelKey: 'systemGovernance.column.deviceType',
    tokenLabels: {
      pda: 'systemGovernance.deviceTypeToken.pda',
      industrial_terminal: 'systemGovernance.deviceTypeToken.industrial_terminal',
      scanner: 'systemGovernance.deviceTypeToken.scanner',
      other: 'systemGovernance.deviceTypeToken.other',
    },
    width: 130,
  },
  { prop: 'username', labelKey: 'systemGovernance.column.username', width: 120 },
  {
    prop: 'status',
    labelKey: 'systemGovernance.column.status',
    tokenLabels: {
      online: 'systemGovernance.deviceStatus.online',
      offline: 'systemGovernance.deviceStatus.offline',
      timeout: 'systemGovernance.deviceStatus.timeout',
    },
    width: 120,
  },
  { prop: 'last_heartbeat_at', labelKey: 'systemGovernance.column.lastHeartbeatAt', width: 180 },
];

// 角色变更审批列：对齐 role_change_approvals handler 的真实键（role_id/reason 不存在，恒空白已修）
const roleChangeCols: ColumnDef[] = [
  { prop: 'id', labelKey: 'systemGovernance.column.id', width: 80 },
  { prop: 'approval_no', labelKey: 'systemGovernance.column.approvalNo', width: 160 },
  {
    prop: 'change_type',
    labelKey: 'systemGovernance.column.changeType',
    tokenLabels: {
      assign_role: 'systemGovernance.changeTypeToken.assign_role',
      assign_permission: 'systemGovernance.changeTypeToken.assign_permission',
      remove_permission: 'systemGovernance.changeTypeToken.remove_permission',
    },
    width: 140,
  },
  { prop: 'target_role_code', labelKey: 'systemGovernance.column.targetRoleCode' },
  { prop: 'target_user_id', labelKey: 'systemGovernance.column.targetUserId', width: 110 },
  { prop: 'applicant_username', labelKey: 'systemGovernance.column.applicant', width: 120 },
  {
    prop: 'status',
    labelKey: 'systemGovernance.column.status',
    tokenLabels: {
      pending_l1: 'systemGovernance.roleChangeStatus.pending_l1',
      pending_l2: 'systemGovernance.roleChangeStatus.pending_l2',
      approved: 'systemGovernance.roleChangeStatus.approved',
      rejected: 'systemGovernance.roleChangeStatus.rejected',
      cancelled: 'systemGovernance.roleChangeStatus.cancelled',
    },
    width: 130,
  },
  { prop: 'created_at', labelKey: 'systemGovernance.column.createdAt', width: 180 },
];

/**
 * 解析文本型数字录入（受托人/角色/用户 ID）为后端要求的 i32。
 * 入参：用户在 el-input 里输入的字符串；出参：合法则返回正整数，非法（空/非数字/≤0/超范围）返回 null。
 * 数值录入用 el-input 承接文本、范围靠此校验器判定，不用带加减号的步进控件。
 */
const parsePositiveId = (raw: string): number | null => {
  const trimmed = String(raw ?? '').trim();
  if (!/^\d+$/.test(trimmed)) return null;
  const num = Number(trimmed);
  if (!Number.isSafeInteger(num) || num <= 0 || num > 2147483647) return null;
  return num;
};

// 委托
const delegations = ref<Array<Record<string, unknown>>>([]);
const loading = ref(false);

async function loadDelegations() {
  loading.value = true;
  try {
    delegations.value = unwrapDataArray(await getDelegationList());
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('systemGovernance.msg.loadFailed'));
  } finally {
    loading.value = false;
  }
}

async function onExpireOverdue() {
  try {
    await expireOverdueDelegations();
    ElMessage.success(t('systemGovernance.msg.expireOverdueDone'));
    await loadDelegations();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('systemGovernance.msg.operationFailed'));
  }
}

async function onRevoke(row: Record<string, unknown>) {
  try {
    await ElMessageBox.confirm(
      t('systemGovernance.msg.confirmRevokeDelegation'),
      t('systemGovernance.msg.confirmTitle'),
      { type: 'warning' }
    );
  } catch (error: unknown) {
    if (isDialogDismissal(error)) return;
    rethrowNonDismissal('systemGovernance.onRevoke', error);
  }
  try {
    await revokeDelegation(row.id as number);
    ElMessage.success(t('systemGovernance.msg.revokeDone'));
    await loadDelegations();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('systemGovernance.msg.operationFailed'));
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
    ElMessage.error(err.message || t('systemGovernance.msg.operationFailed'));
  }
}

// 生效中委托：按受托人查询 active 状态委托
async function onLoadActiveDelegations() {
  let delegateeId = 0;
  try {
    const { value } = await ElMessageBox.prompt(
      t('systemGovernance.msg.promptDelegateeId'),
      t('systemGovernance.button.activeDelegations'),
      {
        inputPattern: /^\d+$/,
        inputErrorMessage: t('systemGovernance.msg.invalidPositiveId'),
      }
    );
    delegateeId = Number(value);
  } catch (error: unknown) {
    if (isDialogDismissal(error)) return;
    rethrowNonDismissal('systemGovernance.onLoadActiveDelegations', error);
  }
  loading.value = true;
  try {
    const res = (await getActiveDelegatedPermissions(delegateeId)) as ApiResponse<unknown>;
    delegations.value = unwrapDataArray(res);
    ElMessage.success(t('systemGovernance.msg.activeDelegationsLoaded', { id: delegateeId }));
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('systemGovernance.msg.operationFailed'));
  } finally {
    loading.value = false;
  }
}

// 新建委托
const delegationDialogVisible = ref(false);
// 后端 CreateDelegationRequest 键集：delegator_id/delegatee_id/permission_code/
// valid_from/valid_until/reason；委托人=当前登录用户（真实取值，非手填）
// 受托人 ID 用文本录入（禁步进控件），提交前按正整数校验
const delegationForm = reactive({
  delegatee_id: '',
  permission_code: '',
  valid_from: '',
  valid_until: '',
  reason: '',
});

const delegationDelegatorText = computed(() =>
  t('systemGovernance.form.delegatorCurrentValue', {
    id: userStore.userInfo?.id ?? '-',
  })
);

const openDelegationDialog = () => {
  // 每次打开复位表单，避免上一委托残留值被当成本次要提交的数据
  delegationForm.delegatee_id = '';
  delegationForm.permission_code = '';
  delegationForm.valid_from = '';
  delegationForm.valid_until = '';
  delegationForm.reason = '';
  delegationDialogVisible.value = true;
};

async function onSaveDelegation() {
  const delegateeId = parsePositiveId(delegationForm.delegatee_id);
  if (delegateeId === null || !delegationForm.permission_code.trim()) {
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
      delegatee_id: delegateeId,
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
    ElMessage.error(err.message || t('systemGovernance.msg.operationFailed'));
  }
}

// 角色关系
const relations = ref<Array<Record<string, unknown>>>([]);
const loadingRelations = ref(false);
const relationDialogVisible = ref(false);
// 键集逐字对齐后端 CreateRoleRelationRequest：parent_role_code/child_role_code/relation_type 必填
const relationForm = reactive({
  parent_role_code: '',
  child_role_code: '',
  relation_type: 'mutual_exclusive',
  description: '',
});

async function loadRelations() {
  loadingRelations.value = true;
  try {
    relations.value = unwrapDataArray(await getRoleRelations());
  } catch (e: unknown) {
    logAuxLoadFailure(t('systemGovernance.msg.relationLoadFailed'), e);
  } finally {
    loadingRelations.value = false;
  }
}

async function onCreateRelation() {
  if (!relationForm.parent_role_code.trim() || !relationForm.child_role_code.trim()) {
    ElMessage.warning(t('systemGovernance.msg.fillBothRoles'));
    return;
  }
  try {
    await createRoleRelation({
      parent_role_code: relationForm.parent_role_code.trim(),
      child_role_code: relationForm.child_role_code.trim(),
      relation_type: relationForm.relation_type,
      // 可选 description 空则省略键，不发送空串污染后端
      ...(relationForm.description.trim() ? { description: relationForm.description.trim() } : {}),
    });
    ElMessage.success(t('systemGovernance.msg.relationCreated'));
    relationDialogVisible.value = false;
    await loadRelations();
  } catch (e: unknown) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('systemGovernance.msg.operationFailed'));
  }
}

async function onDeleteRelation(row: Record<string, unknown>) {
  try {
    await ElMessageBox.confirm(
      t('systemGovernance.msg.confirmDeleteRelation'),
      t('systemGovernance.msg.confirmTitle'),
      { type: 'warning' }
    );
  } catch (error: unknown) {
    if (isDialogDismissal(error)) return;
    rethrowNonDismissal('systemGovernance.onDeleteRelation', error);
  }
  try {
    await deleteRoleRelation(row.id as number);
    ElMessage.success(t('systemGovernance.msg.relationDeleted'));
    await loadRelations();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('systemGovernance.msg.operationFailed'));
  }
}

// AI 模型
const modelVersions = ref<Array<Record<string, unknown>>>([]);
const loadingModels = ref(false);
const aiResult = ref('');
const versionDialogVisible = ref(false);
// 键集对齐后端 CreateModelVersionRequest：model_name/version/algorithm 必填、change_reason 可选
const versionForm = reactive({ model_name: '', version: '', algorithm: '', change_reason: '' });

async function loadModels() {
  loadingModels.value = true;
  try {
    modelVersions.value = unwrapDataArray(await getModelVersionList());
  } catch (e: unknown) {
    logAuxLoadFailure(t('systemGovernance.msg.modelLoadFailed'), e);
  } finally {
    loadingModels.value = false;
  }
}

async function onCreateVersion() {
  if (
    !versionForm.model_name.trim() ||
    !versionForm.version.trim() ||
    !versionForm.algorithm.trim()
  ) {
    ElMessage.warning(t('systemGovernance.msg.fillModelNameVersionAlgorithm'));
    return;
  }
  try {
    await createModelVersion({
      model_name: versionForm.model_name.trim(),
      version: versionForm.version.trim(),
      algorithm: versionForm.algorithm.trim(),
      ...(versionForm.change_reason.trim()
        ? { change_reason: versionForm.change_reason.trim() }
        : {}),
    });
    ElMessage.success(t('systemGovernance.msg.modelVersionCreated'));
    versionDialogVisible.value = false;
    await loadModels();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('systemGovernance.msg.operationFailed'));
  }
}

async function onApprove(row: Record<string, unknown>) {
  try {
    await ElMessageBox.confirm(
      t('systemGovernance.msg.confirmApproveModel'),
      t('systemGovernance.msg.confirmTitle'),
      { type: 'warning' }
    );
  } catch (error: unknown) {
    if (isDialogDismissal(error)) return;
    rethrowNonDismissal('systemGovernance.onApprove', error);
  }
  try {
    // 后端 ApproveModelVersionRequest 必填 approval_status，取值域 pending/approved/rejected；
    // 原载荷发 {} 缺该键必 422，审批恒失败
    await approveModelVersion(row.id as number, { approval_status: 'approved' });
    ElMessage.success(t('systemGovernance.msg.modelApproved'));
    await loadModels();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('systemGovernance.msg.operationFailed'));
  }
}

async function onDrift(row: Record<string, unknown>) {
  try {
    aiResult.value = JSON.stringify(await detectModelDrift(row.id as number), null, 2);
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('systemGovernance.msg.operationFailed'));
  }
}

async function onEvaluations(row: Record<string, unknown>) {
  try {
    aiResult.value = JSON.stringify(await getModelEvaluations(row.id as number), null, 2);
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('systemGovernance.msg.operationFailed'));
  }
}

async function onReconcile() {
  try {
    aiResult.value = JSON.stringify(await reconcileMonthly(), null, 2);
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('systemGovernance.msg.operationFailed'));
  }
}

async function onAccuracyReports() {
  try {
    aiResult.value = JSON.stringify(await getAccuracyReports(), null, 2);
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('systemGovernance.msg.operationFailed'));
  }
}

// 设备
const devices = ref<Array<Record<string, unknown>>>([]);
const loadingDevices = ref(false);
const onlineCount = ref(0);
const deviceDialogVisible = ref(false);
// 键集对齐后端 RegisterDeviceRequest：device_id 必填、device_type 必填（默认 other）、device_name 可选
const deviceForm = reactive({ device_id: '', device_name: '', device_type: 'other' });

async function loadDevices() {
  loadingDevices.value = true;
  try {
    const [list, count] = await Promise.all([getDeviceList(), getOnlineDeviceCount()]);
    devices.value = unwrapPagedItems(list);
    const c = count as { count?: number; data?: { count?: number } };
    onlineCount.value = c?.count ?? c?.data?.count ?? 0;
  } catch (e: unknown) {
    logAuxLoadFailure(t('systemGovernance.msg.deviceLoadFailed'), e);
  } finally {
    loadingDevices.value = false;
  }
}

async function onRegisterDevice() {
  if (!deviceForm.device_id.trim()) {
    ElMessage.warning(t('systemGovernance.msg.fillDeviceId'));
    return;
  }
  try {
    await registerDevice({
      device_id: deviceForm.device_id.trim(),
      device_type: deviceForm.device_type,
      ...(deviceForm.device_name.trim() ? { device_name: deviceForm.device_name.trim() } : {}),
    });
    ElMessage.success(t('systemGovernance.msg.deviceRegistered'));
    deviceDialogVisible.value = false;
    await loadDevices();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('systemGovernance.msg.operationFailed'));
  }
}

// 设备详情
async function onDeviceDetail(row: Record<string, unknown>) {
  try {
    const res = (await getDeviceConnection(String(row.id))) as ApiResponse<unknown>;
    await showDetail(res.data ?? res);
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('systemGovernance.msg.operationFailed'));
  }
}

// 设备心跳
async function onDeviceHeartbeat(row: Record<string, unknown>) {
  try {
    await sendDeviceHeartbeat(String(row.id));
    ElMessage.success(t('systemGovernance.msg.deviceHeartbeatSent', { id: row.id }));
    await loadDevices();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('systemGovernance.msg.operationFailed'));
  }
}

// 断开设备
async function onDisconnectDevice(row: Record<string, unknown>) {
  try {
    await ElMessageBox.confirm(
      t('systemGovernance.msg.confirmDisconnect', { id: row.id }),
      t('systemGovernance.msg.confirmTitle'),
      { type: 'warning' }
    );
  } catch (error: unknown) {
    if (isDialogDismissal(error)) return;
    rethrowNonDismissal('systemGovernance.onDisconnectDevice', error);
  }
  try {
    await disconnectDevice(String(row.id));
    ElMessage.success(t('systemGovernance.msg.deviceDisconnected'));
    await loadDevices();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('systemGovernance.msg.operationFailed'));
  }
}

// 清理超时设备
async function onCleanupTimeoutDevices() {
  try {
    await ElMessageBox.confirm(
      t('systemGovernance.msg.confirmCleanupTimeout'),
      t('systemGovernance.msg.confirmTitle'),
      { type: 'warning' }
    );
  } catch (error: unknown) {
    if (isDialogDismissal(error)) return;
    rethrowNonDismissal('systemGovernance.onCleanupTimeoutDevices', error);
  }
  try {
    await cleanupTimeoutDevices();
    ElMessage.success(t('systemGovernance.msg.timeoutCleaned'));
    await loadDevices();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('systemGovernance.msg.operationFailed'));
  }
}

// 模型版本状态流转（激活 active / 弃用 retired）
// 后端 ChangeModelStatusRequest 键为 new_status（非 status），取值域 draft/active/retired/archived；
// 原前端发 {status} 键名不符必 422，且把"弃用"发成非法值 deprecated（词表无此值）
const onModelStatus = async (row: Record<string, unknown>, newStatus: 'active' | 'retired') => {
  try {
    await changeModelStatus(Number(row.id), { new_status: newStatus });
    ElMessage.success(
      newStatus === 'active'
        ? t('systemGovernance.msg.modelActivated')
        : t('systemGovernance.msg.modelRetired')
    );
    loadModels();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('systemGovernance.msg.operationFailed'));
  }
};

// 模型评估登记（对选中版本创建评估记录）
// 后端 CreateModelEvaluationRequest：model_version_id 必填、sample_count 必填 i32，
// 原前端发 evaluation_type/result 后端无此键、又漏 sample_count，评估恒 422
const onModelEvaluation = async (row?: Record<string, unknown>) => {
  const versionId = row ? Number(row.id) : (modelVersions.value[0]?.id as number | undefined);
  if (!versionId) {
    ElMessage.warning(t('systemGovernance.msg.noModelVersion'));
    return;
  }
  try {
    await createModelEvaluation({
      model_version_id: versionId,
      sample_count: 0,
      evaluation_report: 'manual evaluation',
    });
    ElMessage.success(t('systemGovernance.msg.evaluationCreated'));
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('systemGovernance.msg.operationFailed'));
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
    ElMessageBox.alert(
      lines.join('\n') || t('systemGovernance.msg.noDecisionLogs'),
      t('systemGovernance.msg.decisionLogsTitle'),
      {
        type: 'info',
      }
    );
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('systemGovernance.msg.operationFailed'));
  }
};

// 角色变更审批：列表 + L1/L2 审批/驳回/撤销
const roleChanges = ref<Record<string, unknown>[]>([]);
const roleChangeLoading = ref(false);

const loadRoleChanges = async () => {
  roleChangeLoading.value = true;
  try {
    const res = (await getRoleChangeApprovals({})) as ApiResponse<unknown>;
    roleChanges.value = unwrapList<Record<string, unknown>>(res.data);
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('systemGovernance.msg.loadFailed'));
  } finally {
    roleChangeLoading.value = false;
  }
};

// 新建角色变更审批
const roleChangeDialogVisible = ref(false);
// 键集与后端 CreateRoleChangeApprovalRequest 逐字一致：
// change_type/target_role_id/target_role_code 必填；无 reason（后端缺口，见串行清单）
// change_type 源保持 string 供 el-select v-model 回写，提交前用守卫收窄（禁止裸断言）
// target_role_id / target_user_id 用文本录入（禁步进控件），提交前按正整数校验
const ROLE_CHANGE_TYPES: readonly RoleChangeType[] = [
  'assign_role',
  'assign_permission',
  'remove_permission',
];
const isRoleChangeType = (v: string): v is RoleChangeType =>
  (ROLE_CHANGE_TYPES as readonly string[]).includes(v);

const roleChangeForm = reactive({
  change_type: 'assign_role',
  target_role_id: '',
  target_role_code: '',
  target_user_id: '',
});

const onSaveRoleChange = async () => {
  if (!roleChangeForm.target_role_code.trim()) {
    ElMessage.warning(t('systemGovernance.roleChange.fillTargetRoleCode'));
    return;
  }
  const targetRoleId = parsePositiveId(roleChangeForm.target_role_id);
  if (targetRoleId === null) {
    ElMessage.warning(t('systemGovernance.msg.fillTargetRoleId'));
    return;
  }
  // 目标用户为可选：填了才校验并按键发送，留空则省略键（不发送 0 或空串污染后端）
  let targetUserId: number | undefined;
  if (roleChangeForm.target_user_id.trim()) {
    const parsed = parsePositiveId(roleChangeForm.target_user_id);
    if (parsed === null) {
      ElMessage.warning(t('systemGovernance.msg.invalidTargetUserId'));
      return;
    }
    targetUserId = parsed;
  }
  const changeType = roleChangeForm.change_type;
  if (!isRoleChangeType(changeType)) {
    ElMessage.warning(t('systemGovernance.roleChange.invalidChangeType'));
    return;
  }
  try {
    await createRoleChangeApproval({
      change_type: changeType,
      target_role_id: targetRoleId,
      target_role_code: roleChangeForm.target_role_code.trim(),
      ...(targetUserId !== undefined ? { target_user_id: targetUserId } : {}),
    });
    ElMessage.success(t('systemGovernance.roleChange.created'));
    roleChangeDialogVisible.value = false;
    await loadRoleChanges();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('systemGovernance.msg.operationFailed'));
  }
};

// 审批单详情
const onRoleChangeDetail = async (row: Record<string, unknown>) => {
  try {
    const res = (await getRoleChangeApproval(Number(row.id))) as ApiResponse<unknown>;
    await showDetail(res.data ?? res);
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('systemGovernance.msg.operationFailed'));
  }
};

const onRoleChangeAction = async (
  row: Record<string, unknown>,
  action: 'approve-l1' | 'approve-l2' | 'reject' | 'cancel'
) => {
  let reason = '';
  if (action === 'reject') {
    try {
      const { value } = await ElMessageBox.prompt(
        t('systemGovernance.msg.promptRejectReason'),
        t('systemGovernance.msg.rejectTitle'),
        {
          type: 'warning',
          inputPattern: /\S+/,
          inputErrorMessage: t('systemGovernance.msg.rejectReasonRequired'),
        }
      );
      reason = value;
    } catch (error: unknown) {
      if (isDialogDismissal(error)) return;
      rethrowNonDismissal('systemGovernance.collectRejectReason', error);
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
    ElMessage.success(t('systemGovernance.msg.actionDone'));
    loadRoleChanges();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('systemGovernance.msg.operationFailed'));
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

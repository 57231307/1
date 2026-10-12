<!--
  ArReconciliationDispute.vue - AR 对账争议处理对话框
  拆分自 arReconciliation/enhanced.vue（P14 批 1 B3 I-2）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="$t('arReconciliationModule.disputeHandling')"
    width="900px"
    :aria-label="$t('arReconciliationModule.disputeDialogAria')"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form
      :model="localForm"
      label-width="100px"
      :aria-label="$t('arReconciliationModule.disputeFormAria')"
    >
      <!-- 后端 CreateDisputeApiRequest 只有 reconciliation_id/customer_id/reason/description，
           争议原因落 dispute_reason；原「争议类型/争议金额」采集项在 DTO 与表结构均无对应
           （提交即被 serde 丢弃 = 假采集），已移除 -->
      <el-form-item :label="$t('arReconciliationModule.disputeDescription')" required>
        <el-input
          :model-value="localForm.description"
          type="textarea"
          :rows="3"
          :placeholder="$t('arReconciliationModule.disputeDescriptionPlaceholder')"
          @update:model-value="(v: string) => (localForm.description = v ?? '')"
        />
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="emit('submit')">{{
          $t('arReconciliationModule.submitDispute')
        }}</el-button>
      </el-form-item>
    </el-form>

    <el-divider>{{ $t('arReconciliationModule.disputeRecords') }}</el-divider>
    <!-- list_disputes 返回的是对账单列表（reconciliation_status='disputed'），
         键为 reconciliation_no / period_* / dispute_reason / created_at -->
    <el-table
      :data="disputes"
      border
      style="width: 100%"
      :aria-label="$t('arReconciliationModule.disputeRecordsAria')"
    >
      <el-table-column
        prop="reconciliation_no"
        :label="$t('arReconciliationModule.sourceNo')"
        width="150"
      />
      <el-table-column :label="$t('arReconciliationModule.reconciliationPeriod')" width="210">
        <template #default="scope"
          >{{ scope.row.period_start }} ~ {{ scope.row.period_end }}</template
        >
      </el-table-column>
      <el-table-column
        prop="dispute_reason"
        :label="$t('arReconciliationModule.disputeDescription')"
        show-overflow-tooltip
      />
      <el-table-column prop="created_at" :label="$t('common.createTime')" width="160" />
      <el-table-column :label="$t('common.operation')" width="100" align="center">
        <template #default="scope">
          <el-button
            v-if="scope.row.reconciliation_status === 'disputed'"
            size="small"
            type="primary"
            @click="emit('resolve', scope.row)"
          >
            {{ $t('arReconciliationModule.resolve') }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue';
import type { CreateDisputePayload, DisputeRecord } from '@/api/ar-reconciliation-enhanced';

const props = defineProps<{
  visible: boolean;
  // 表单数据（由父组件管理，子组件通过 emit 回写）
  form: CreateDisputePayload;
  disputes: DisputeRecord[];
}>();

const emit = defineEmits<{
  'update:visible': [v: boolean];
  submit: [];
  resolve: [row: DisputeRecord];
  // 整体回写表单数据（父组件监听此事件并 Object.assign 到自己的 form）
  'update:form': [v: CreateDisputePayload];
}>();

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localForm = ref<CreateDisputePayload>({ ...props.form });

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false;

// 外部 prop 变化时同步到 local（如父组件重新打开对话框时填充数据）
watch(
  () => props.form,
  newForm => {
    if (syncing) return;
    syncing = true;
    localForm.value = { ...newForm };
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);

// 本地变化时通知父组件（用户输入）
watch(
  localForm,
  newForm => {
    if (syncing) return;
    syncing = true;
    emit('update:form', { ...newForm });
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);
</script>

<!--
  ColorCardIssueDetail.vue - 色卡发放详情操作组件（V15 P0-F11）

  设计原则：单一组件聚合 4 个操作对话框（归还/遗失/损坏/取消），
  通过 v-model:visible 与 action prop 控制显示，emit('confirm', payload)
  由父组件调用 composable 处理业务。

  关联：
    - composables/useColorCardIssue.ts
    - types/colorCardIssue.ts::{ReturnDialogState, LostDialogState, DamagedDialogState, CancelDialogState, IssueAction}
-->
<template>
  <!-- 归还对话框 -->
  <el-dialog v-model="visibleReturn" title="归还色卡" width="480px">
    <el-form label-width="80px">
      <el-form-item label="实际归还">
        <el-date-picker
          v-model="returnForm.actual_return_date"
          type="date"
          value-format="YYYY-MM-DD"
          style="width: 100%"
        />
      </el-form-item>
      <el-form-item label="备注">
        <el-input v-model="returnForm.remark" type="textarea" :rows="2" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visibleReturn = false">取消</el-button>
      <el-button type="primary" :loading="loading" @click="confirmReturn">
        确认归还
      </el-button>
    </template>
  </el-dialog>

  <!-- 遗失对话框 -->
  <el-dialog v-model="visibleLost" title="登记遗失" width="480px">
    <el-alert type="warning" :closable="false" style="margin-bottom: 16px">
      登记遗失后色卡状态将变为「遗失」，需要填写赔付金额。
    </el-alert>
    <el-form label-width="100px">
      <el-form-item label="赔付金额" required>
        <el-input-number
          v-model="lostForm.compensation_amount"
          :min="0.01"
          :precision="2"
          :step="100"
          style="width: 100%"
        />
      </el-form-item>
      <el-form-item label="遗失原因">
        <el-input v-model="lostForm.remark" type="textarea" :rows="2" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visibleLost = false">取消</el-button>
      <el-button type="danger" :loading="loading" @click="confirmLost">
        确认登记
      </el-button>
    </template>
  </el-dialog>

  <!-- 损坏对话框 -->
  <el-dialog v-model="visibleDamaged" title="标记损坏" width="480px">
    <el-form label-width="100px">
      <el-form-item label="赔付金额">
        <el-input-number
          v-model="damagedForm.compensation_amount"
          :min="0"
          :precision="2"
          :step="100"
          style="width: 100%"
        />
      </el-form-item>
      <el-form-item label="损坏原因">
        <el-input v-model="damagedForm.remark" type="textarea" :rows="2" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visibleDamaged = false">取消</el-button>
      <el-button type="warning" :loading="loading" @click="confirmDamaged">
        确认标记
      </el-button>
    </template>
  </el-dialog>

  <!-- 取消对话框 -->
  <el-dialog v-model="visibleCancel" title="取消发放" width="480px">
    <el-alert type="info" :closable="false" style="margin-bottom: 16px">
      取消发放后记录将变为「已取消」终态，不可再变更。
    </el-alert>
    <el-form label-width="80px">
      <el-form-item label="取消原因">
        <el-input v-model="cancelForm.remark" type="textarea" :rows="2" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visibleCancel = false">关闭</el-button>
      <el-button type="info" :loading="loading" @click="confirmCancel">
        确认取消
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { ElMessage } from 'element-plus'
import type { IssueRecordInfo } from '@/api/color-card'
import type {
  ReturnDialogState,
  LostDialogState,
  DamagedDialogState,
  CancelDialogState,
  IssueAction,
} from '@/types/colorCardIssue'

const props = defineProps<{
  /** 当前操作的记录 */
  record: IssueRecordInfo | null
  /** 操作类型 */
  action: IssueAction | null
  /** 是否可见（v-model） */
  visible: boolean
  /** 操作进行中 */
  loading: boolean
}>()

const emit = defineEmits<{
  (e: 'update:visible', val: boolean): void
  (e: 'confirm', payload: { action: IssueAction; recordId: number; data: ReturnDialogState | LostDialogState | DamagedDialogState | CancelDialogState }): void
}>()

// 4 个对话框可见性（独立控制）
const visibleReturn = ref(false)
const visibleLost = ref(false)
const visibleDamaged = ref(false)
const visibleCancel = ref(false)

// 4 个表单状态
const returnForm = reactive<ReturnDialogState>({ actual_return_date: '', remark: '' })
const lostForm = reactive<LostDialogState>({ compensation_amount: 0, remark: '' })
const damagedForm = reactive<DamagedDialogState>({ compensation_amount: 0, remark: '' })
const cancelForm = reactive<CancelDialogState>({ remark: '' })

// 监听 props.action + props.visible 触发对应对话框
watch(
  () => [props.action, props.visible] as const,
  ([action, visible]) => {
    if (!visible || !action || !props.record) return
    // 重置表单
    returnForm.actual_return_date = ''
    returnForm.remark = ''
    lostForm.compensation_amount = 0
    lostForm.remark = ''
    damagedForm.compensation_amount = 0
    damagedForm.remark = ''
    cancelForm.remark = ''

    // 显示对应对话框（关闭其他）
    visibleReturn.value = action === 'return'
    visibleLost.value = action === 'lost'
    visibleDamaged.value = action === 'damaged'
    visibleCancel.value = action === 'cancel'
  },
)

// 关闭任一对话框 → 通知父组件
watch([visibleReturn, visibleLost, visibleDamaged, visibleCancel], () => {
  if (!visibleReturn.value && !visibleLost.value && !visibleDamaged.value && !visibleCancel.value) {
    emit('update:visible', false)
  }
})

function closeAll(): void {
  visibleReturn.value = false
  visibleLost.value = false
  visibleDamaged.value = false
  visibleCancel.value = false
}

async function confirmReturn(): Promise<void> {
  if (!props.record) return
  emit('confirm', {
    action: 'return',
    recordId: props.record.id,
    data: { ...returnForm },
  })
  closeAll()
}

async function confirmLost(): Promise<void> {
  if (!props.record) return
  if (lostForm.compensation_amount <= 0) {
    ElMessage.warning('请填写赔付金额（必须 > 0）')
    return
  }
  emit('confirm', {
    action: 'lost',
    recordId: props.record.id,
    data: { ...lostForm },
  })
  closeAll()
}

async function confirmDamaged(): Promise<void> {
  if (!props.record) return
  emit('confirm', {
    action: 'damaged',
    recordId: props.record.id,
    data: { ...damagedForm },
  })
  closeAll()
}

async function confirmCancel(): Promise<void> {
  if (!props.record) return
  emit('confirm', {
    action: 'cancel',
    recordId: props.record.id,
    data: { ...cancelForm },
  })
  closeAll()
}
</script>

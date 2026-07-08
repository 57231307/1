/**
 * usePurchRcv - 采购收货 composable
 * 任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue 收货对话框）
 */
import { ref } from 'vue'
import { ElMessage } from 'element-plus'
import { purchaseApi, type PurchaseOrder, type PurchaseOrderItem } from '@/api/purchase'

/**
 * 收货明细行数据结构
 */
export type ReceiveItem = PurchaseOrderItem & {
  receive_quantity: number
  remarks: string
}

/**
 * 收货表单数据结构
 */
export interface ReceiveFormData {
  order_id: number
  order_no: string
  supplier_name: string
  receive_date: string
  warehouse_id: number | undefined
  items: ReceiveItem[]
}

/**
 * 采购收货 composable
 */
export function usePurchRcv(onSuccess: () => void) {
  const receiveDialogVisible = ref(false)
  const receiveForm = ref<ReceiveFormData>({
    order_id: 0,
    order_no: '',
    supplier_name: '',
    receive_date: new Date().toISOString().split('T')[0],
    warehouse_id: undefined,
    items: [],
  })

  /**
   * 打开收货对话框
   */
  const handleReceive = (row: PurchaseOrder) => {
    receiveForm.value = {
      order_id: row.id,
      order_no: row.order_no,
      supplier_name: row.supplier_name,
      receive_date: new Date().toISOString().split('T')[0],
      warehouse_id: undefined,
      items: (row.items || []).map((item: PurchaseOrderItem) => ({
        ...item,
        receive_quantity: 0,
        remarks: '',
      })),
    }
    receiveDialogVisible.value = true
  }

  /**
   * 提交收货
   */
  const submitReceive = async () => {
    if (!receiveForm.value.warehouse_id) {
      ElMessage.warning('请选择收货仓库')
      return
    }
    const validItems = receiveForm.value.items.filter(item => item.receive_quantity > 0)
    if (validItems.length === 0) {
      ElMessage.warning('请填写至少一项收货数量')
      return
    }
    try {
      await purchaseApi.createReceipt({
        order_id: receiveForm.value.order_id,
        receipt_date: receiveForm.value.receive_date,
        warehouse_id: receiveForm.value.warehouse_id,
        items: validItems.map(item => ({
          product_id: item.product_id,
          received_quantity: item.receive_quantity,
          remark: item.remarks,
        })),
      })
      ElMessage.success('收货成功')
      receiveDialogVisible.value = false
      onSuccess()
    } catch (error: unknown) {
      ElMessage.error((error instanceof Error ? error.message : '') || '收货失败')
    }
  }

  return {
    receiveDialogVisible,
    receiveForm,
    handleReceive,
    submitReceive,
  }
}

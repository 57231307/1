/**
 * usePurchRcv - 采购收货 composable
 * 任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue 收货对话框）
 */
import { ref } from 'vue';
import { ElMessage } from 'element-plus';
import { msg } from '@/utils/message';
import { logger } from '@/utils/logger';
import {
  createPurchaseReceipt,
  type CreatePurchaseReceiptRequest,
  type CreateReceiptItemRequest,
} from '@/api/purchase-receipt';
import type { PurchaseOrder, PurchaseOrderItem } from '@/api/purchase';
import type { Product } from '@/api/product';

/**
 * 收货明细行数据结构
 */
export type ReceiveItem = PurchaseOrderItem & {
  receive_quantity: number;
  remarks: string;
  /**
   * 收货批次号：入库四维之一，后端 create_receipt→validate_receipt_item_dimensions
   * 建单期强校验非空；批次属收货实测数据，无自动来源，由收货人在对话框录入。
   */
  batch_no: string;
};

/**
 * 收货表单数据结构
 */
export interface ReceiveFormData {
  order_id: number;
  order_no: string;
  /** 随关联采购订单一并带出：后端 CreatePurchaseReceiptRequest.supplier_id 为必填非 Option */
  supplier_id: number;
  supplier_name: string | null;
  receive_date: string;
  warehouse_id: number | undefined;
  items: ReceiveItem[];
}

/**
 * 采购收货 composable
 * @param onSuccess 收货成功后的列表刷新回调
 * @param getProducts 产品主数据提供者（物料编码/名称/主单位的唯一干净来源，
 *   后端 CreateReceiptItemRequest.material_code/material_name/unit_master 为必填）
 */
export function usePurchRcv(onSuccess: () => void, getProducts: () => Product[]) {
  const receiveDialogVisible = ref(false);
  const receiveForm = ref<ReceiveFormData>({
    order_id: 0,
    order_no: '',
    supplier_id: 0,
    supplier_name: '',
    receive_date: new Date().toISOString().split('T')[0],
    warehouse_id: undefined,
    items: [],
  });

  /**
   * 打开收货对话框
   */
  const handleReceive = (row: PurchaseOrder) => {
    receiveForm.value = {
      order_id: row.id,
      order_no: row.order_no,
      supplier_id: row.supplier_id,
      supplier_name: row.supplier_name,
      receive_date: new Date().toISOString().split('T')[0],
      warehouse_id: undefined,
      items: (row.items || []).map((item: PurchaseOrderItem) => ({
        ...item,
        receive_quantity: 0,
        remarks: '',
        batch_no: '',
      })),
    };
    receiveDialogVisible.value = true;
  };

  /**
   * 组装后端 CreatePurchaseReceiptRequest 契约 payload。
   * 逐字段映射（原 product_id/received_quantity/remark 是错误键名，后端 serde 拒绝）：
   *   - order_id/supplier_id 来自关联采购订单（supplier_id 为后端必填非 Option）；
   *   - material_id = 订单明细 product_id；material_code/material_name/unit_master 取产品主数据；
   *   - batch_no 取收货人录入值；quantity = 本次收货数量；
   *   - quantity_alt 本对话框未采集辅助单位数量，按入库契约给确定性 0（与正规页/seed 同口径，
   *     非 ?? 兜底掩盖缺键——收货仅按主单位计量时辅助量即为 0）。
   * 物料主数据缺失时抛错，交由 submitReceive 拦截并暴露，禁止拼 P{id}/'米' 伪值提交。
   */
  const buildReceiptPayload = (validItems: ReceiveItem[]): CreatePurchaseReceiptRequest => {
    const products = getProducts();
    const items: CreateReceiptItemRequest[] = validItems.map((item, idx) => {
      const product = products.find(p => p.id === item.product_id);
      if (!product || !product.product_code || !product.product_name || !product.unit) {
        throw new Error(msg.translate('receiptItemMasterMissing', { line: idx + 1 }));
      }
      return {
        line_no: item.line_no,
        material_id: item.product_id,
        material_code: product.product_code,
        material_name: product.product_name,
        batch_no: item.batch_no.trim(),
        quantity: item.receive_quantity,
        quantity_alt: 0,
        unit_master: product.unit,
        unit_price: item.unit_price,
      };
    });
    return {
      order_id: receiveForm.value.order_id,
      supplier_id: receiveForm.value.supplier_id,
      receipt_date: receiveForm.value.receive_date,
      // 前置校验已保证 warehouse_id 非空，此处为类型收窄非 ?? 兜底
      warehouse_id: receiveForm.value.warehouse_id as number,
      items,
    };
  };

  /**
   * 提交收货
   */
  const submitReceive = async () => {
    if (!receiveForm.value.warehouse_id) {
      msg.warning('pleaseSelectWarehouse');
      return;
    }
    const validItems = receiveForm.value.items.filter(item => item.receive_quantity > 0);
    if (validItems.length === 0) {
      msg.warning('pleaseFillItem');
      return;
    }
    // 批次号是后端建单期强校验维度，缺任一行的批次即整单拒绝；只能实测录入，禁止塞假值
    const missingBatch = validItems.find(item => !item.batch_no.trim());
    if (missingBatch) {
      msg.warning('receiveBatchRequired');
      logger.warn(`[收货] 产品 ${missingBatch.product_id} 未录入批次号，拒绝建单`);
      return;
    }
    try {
      await createPurchaseReceipt(buildReceiptPayload(validItems));
      msg.success('receiveSuccess');
      receiveDialogVisible.value = false;
      onSuccess();
    } catch (error: unknown) {
      ElMessage.error(
        (error instanceof Error ? error.message : '') || msg.translate('receiveFailed')
      );
    }
  };

  return {
    receiveDialogVisible,
    receiveForm,
    handleReceive,
    submitReceive,
  };
}

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
import { getPurchaseOrderById, type PurchaseOrder, type PurchaseOrderItem } from '@/api/purchase';
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
  /**
   * 实收辅助数量（提交时映射到契约键 quantity_alt）：
   * 后端 CreateReceiptItemRequest.quantity_alt 为**非 Option 必填**十进制
   * （backend/src/services/purchase_receipt_dto 内定义），缺键即 serde 反序列化拒绝；
   * 该列为 NOT NULL 且累加进 received_quantity_alt/total_quantity_alt，
   * 未录入不得塌成 0 伪造成「实收为 0」的假量（0 只允许是用户显式输入的实测值）。
   * 命名与主量 receive_quantity 对齐：订单行自带的 quantity_alt 是**订购**辅量
   * （PurchaseOrderItem.quantity_alt: string，出参十进制字符串），语义不同，不复用键位。
   */
  receive_quantity_alt?: number;
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
   * 十进制出参解析：明细的数量/单价列在后端是 DECIMAL（purchase_order_item 模型，
   * rust_decimal 经 JSON 序列化为**字符串**，如 "20.0000"），el-input-number 与
   * :max 计算只接受 number，加载边界必须显式解析：
   * Number('')===0、Number(null)===0 会把缺值伪装成合法数量，NaN 会静默传导到
   * 可收量上限与提交 payload，故缺值/非法值一律抛错，由调用方拒绝打开并留痕。
   */
  const parseItemDecimal = (value: unknown, field: string, lineNo: unknown): number => {
    if (value === null || value === undefined || value === '') {
      throw new Error(`采购订单明细第 ${String(lineNo)} 行缺少十进制列 ${field}`);
    }
    const n = Number(value);
    if (!Number.isFinite(n)) {
      throw new Error(
        `采购订单明细第 ${String(lineNo)} 行的 ${field} 不是合法十进制：${String(value)}`
      );
    }
    return n;
  };

  /**
   * 打开收货对话框
   *
   * 明细必须回源：列表出参 = `PurchaseOrderDto`（backend/src/services/po/order 内定义），
   * 该 DTO 不含 items 键（明细只在 `get_order` 详情里由 handler 单查 purchase_order_item
   * 并 LEFT JOIN products 补 product_name/product_code 后挂到 order_json["items"]，
   * 见 backend/src/handlers/purchase_order_handler）。若直接用列表行 row.items，
   * 该键恒不存在 → 对话框明细表恒 0 行 → 「本次收货」输入框根本不渲染，收货无法录入。
   */
  const handleReceive = async (row: PurchaseOrder) => {
    let detailItems: PurchaseOrderItem[];
    try {
      const res = await getPurchaseOrderById(row.id);
      // 详情出参的 items 由后端 handler 注入（见上），而 api/purchase.ts 的 PurchaseOrder 把 items
      // 声明为必填键（列表 DTO 实际不给这个键），故这里按真实出参形状取值并做运行时数组校验，
      // 缺键/空数组一律显式报错中止，不用 || [] 之类的默认值掩盖。
      const items = (res.data as unknown as { items?: PurchaseOrderItem[] })?.items;
      if (!Array.isArray(items) || items.length === 0) {
        logger.error('[收货] 采购订单详情未返回可用明细，拒绝打开收货对话框', {
          orderNo: row.order_no,
          items,
        });
        msg.error('loadPurchaseOrderDetailFailed');
        return;
      }
      detailItems = items;
    } catch (error: unknown) {
      // 非 2xx 由 api/request.ts 响应拦截器统一提示后端 message，此处只记日志并中止打开
      logger.error(`[收货] 获取采购订单 ${row.order_no} 明细失败`, error);
      return;
    }

    // 十进制出参解析失败 = 详情数据不可信，与请求失败分流：拦截器不会为解析错误出提示，
    // 这里显式报错中止打开（每步失败都有可归因的外显与日志，不静默吞错）。
    let receiveItems: ReceiveItem[];
    try {
      receiveItems = detailItems.map((item: PurchaseOrderItem) => ({
        ...item,
        quantity: parseItemDecimal(item.quantity, 'quantity', item.line_no),
        received_quantity: parseItemDecimal(
          item.received_quantity,
          'received_quantity',
          item.line_no
        ),
        unit_price: parseItemDecimal(item.unit_price, 'unit_price', item.line_no),
        receive_quantity: 0,
        remarks: '',
        batch_no: '',
      }));
    } catch (error: unknown) {
      logger.error(
        `[收货] 采购订单 ${row.order_no} 明细十进制字段解析失败，拒绝打开收货对话框`,
        error
      );
      msg.error('loadPurchaseOrderDetailFailed');
      return;
    }

    receiveForm.value = {
      order_id: row.id,
      order_no: row.order_no,
      supplier_id: row.supplier_id,
      supplier_name: row.supplier_name,
      receive_date: new Date().toISOString().split('T')[0],
      warehouse_id: undefined,
      items: receiveItems,
    };
    receiveDialogVisible.value = true;
  };

  /**
   * 组装后端 CreatePurchaseReceiptRequest 契约 payload。
   * 逐字段映射（原 product_id/received_quantity/remark 是错误键名，后端 serde 拒绝）：
   *   - order_id/supplier_id 来自关联采购订单（supplier_id 为后端必填非 Option）；
   *   - material_id = 订单明细 product_id；material_code/material_name/unit_master 取产品主数据；
   *   - batch_no 取收货人录入值；quantity = 本次收货数量；
   *   - quantity_alt = 收货人实录入参的辅助数量（submitReceive 已按创建契约必填键拦截未录入行，
   *     非空断言仅为类型收窄，禁止 ?? 0 把未采集伪装成实收 0）。
   * 维度采集面只补批次+辅量，不造色号/缸号输入：后端建单期维度校验无条件强制的只有
   * material_id 与 batch_no（backend/src/services/purchase_receipt_ops/crud 的
   * validate_receipt_item_dimensions），色号/缸号由同文件委托的
   * inv::fabric_class::validate_fabric_trace
   * 按布种判定——色号为空即白坯、免缸号，是契约内的合法录入口径；染色布四维全量录入的
   * 采集面在正规入库页（/purchase-receipt），本快录对话框不重复造半套维度输入诱导假维度。
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
        // 创建契约必填非 Option 键（backend/src/services/purchase_receipt_dto 的
        // CreateReceiptItemRequest）；
        // 未录入行已由 submitReceive 拦下，此处非空断言是类型收窄而非兜底
        quantity_alt: item.receive_quantity_alt!,
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
    // 辅量必填拦截（与正规入库页 usePrcProc 同口径）：创建契约 quantity_alt 为非 Option 必填键
    // （backend/src/services/purchase_receipt_dto 的 CreateReceiptItemRequest），本对话框未采集「省键」三态的能力，
    // 未录入必须本地显式拦下提示，而不是让后端缺键 422 打哑谜；
    // 「实收为 0」的合法口径是用户显式输入 0，不是留空塌成默认值。
    const missingAltIndex = validItems.findIndex(
      item => item.receive_quantity_alt == null || !Number.isFinite(item.receive_quantity_alt)
    );
    if (missingAltIndex >= 0) {
      msg.warning('receiptItemAltQtyRequired', { line: missingAltIndex + 1 });
      logger.warn(
        `[收货] 第 ${missingAltIndex + 1} 行未录入辅助数量（创建契约必填键 quantity_alt），拒绝建单`
      );
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

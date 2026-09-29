/**
 * usePrcProc.ts - 采购入库流程操作 composable
 * 任务编号: P14 批 2 I-3 第 4 批（拆分原 purchaseReceipt/index.vue）
 * 封装搜索 / 翻页 / 打开对话框 / 增删明细 / 提交 / 删除 / 审核等流程性方法
 * 行为完全保持一致（仅结构重构）
 *
 * 设计说明：通过 callbacks 接收 usePrc 的状态引用（Reactive 包装层）；
 * 由于 usePrc 返回 reactive({...})，父组件传入 prc.searchForm 等会自动解包为值
 */
import { reactive, ref } from 'vue';
import { logger } from '@/utils/logger';
import { ElMessageBox } from 'element-plus';
import { msg } from '@/utils/message';
import { PURCHASE_RECEIPT_STATUS } from '@/utils/purchase-receipt-status';
import {
  createReceiptItem,
  updateReceiptItem,
  deleteReceiptItem,
  generatePurchaseReceiptNo,
} from '@/api/purchase-receipt';
import {
  getPurchaseReceipt,
  getReceiptItems,
  createPurchaseReceipt,
  updatePurchaseReceipt,
  deletePurchaseReceipt,
  approvePurchaseReceipt,
  type PurchaseReceiptEntity,
  type ReceiptItem,
  type CreatePurchaseReceiptRequest,
  type CreateReceiptItemRequest,
} from '@/api/purchase-receipt';
import type { PrcForm } from './usePrc';

/**
 * 流程回调（接收 usePrc 返回的状态，自动解包后的值类型）
 * 批次 285：queryParams 放宽为 Record<string, unknown>，page 改为独立字段
 */
interface PrcCallbacks {
  // 查询参数（放宽为 Record<string, unknown>，兼容 useTableApi）
  queryParams: Record<string, unknown>;
  // 当前页（独立字段，不在 queryParams 内）
  page: number;
  // 表单
  dialogVisible: boolean;
  dialogTitle: string;
  form: PrcForm;
  // 详情
  viewDialogVisible: boolean;
  viewData: PurchaseReceiptEntity | null;
  detailData: ReceiptItem[];
  // 列表刷新
  loadData: () => Promise<void>;
}

/**
 * 采购入库流程操作方法集合
 */
export function usePrcProc(cb: PrcCallbacks) {
  /** 查询 */
  const handleSearch = () => {
    cb.page = 1;
    cb.loadData();
  };

  /** 重置 */
  const handleReset = () => {
    cb.queryParams = {
      receipt_no: '',
      supplier_id: '',
      warehouse_id: '',
      status: '',
    };
    cb.page = 1;
    cb.loadData();
  };

  /** 打开新增对话框（预生成单号供参考，后端保存时以最终生成为准） */
  const openAddDialog = () => {
    cb.dialogTitle = msg.translate('addReceiptTitle');
    cb.form = {
      receipt_no: '',
      receipt_date: new Date().toISOString().split('T')[0],
      supplier_id: undefined,
      warehouse_id: undefined,
      status: 'draft',
      items: [
        {
          product_id: 0,
          quantity: 0,
          quantity_alt: 0,
          unit_price: 0,
          amount: 0,
          batch_no: '',
          color_code: '',
          lot_no: '',
          grade: '',
        },
      ],
    };
    cb.dialogVisible = true;
    // 预生成单号展示（generatePurchaseReceiptNo，后端保存时最终生成）
    void generatePurchaseReceiptNo()
      .then(res => {
        const no = res.data?.receipt_no;
        if (no && !cb.form.id) {
          cb.form.receipt_no = no;
          cb.dialogTitle = msg.translate('addReceiptTitleWithNo', { no });
        }
      })
      .catch(error => {
        logger.error(msg.translate('pregenReceiptNoFailed'), error);
      });
  };

  /** 打开编辑对话框 */
  const openEditDialog = async (row: PurchaseReceiptEntity) => {
    cb.dialogTitle = msg.translate('editReceiptTitle');
    const res = await getPurchaseReceipt(row.id!);
    const itemsRes = await getReceiptItems(row.id!);
    cb.form = { ...(res.data as unknown as PrcForm), items: itemsRes.data };
    cb.dialogVisible = true;
  };

  /** 打开详情对话框 */
  const openViewDialog = async (row: PurchaseReceiptEntity) => {
    try {
      const res = await getPurchaseReceipt(row.id!);
      cb.viewData = res.data || null;
      const itemsRes = await getReceiptItems(row.id!);
      cb.detailData = itemsRes.data;
      cb.viewDialogVisible = true;
    } catch (error) {
      msg.error('loadDetailFailed');
    }
  };

  /** 添加明细 */
  const addItem = () => {
    if (!cb.form.items) cb.form.items = [];
    cb.form.items.push({
      product_id: 0,
      quantity: 0,
      quantity_alt: 0,
      unit_price: 0,
      amount: 0,
      batch_no: '',
      color_code: '',
      lot_no: '',
      grade: '',
    });
  };

  /** 删除明细（编辑态记录已删除的明细 ID，提交时走 deleteReceiptItem） */
  const removedItemIds = ref<number[]>([]);
  const removeItem = (index: number) => {
    if ((cb.form.items || []).length > 1) {
      const removed = cb.form.items![index];
      if (cb.form.id && removed?.id) removedItemIds.value.push(removed.id);
      cb.form.items!.splice(index, 1);
    }
  };

  /** 计算明细金额 */
  const calculateItemAmount = (item: ReceiptItem) => {
    item.amount = (item.quantity || 0) * (item.unit_price || 0);
  };

  /**
   * 提交表单（仅 API 调用 + 明细校验，表单规则校验已由 PurchaseReceiptForm 内部完成）
   */
  const handleSubmit = async () => {
    const validItems = (cb.form.items || []).filter(e => e.product_id > 0 && e.quantity !== 0);
    if (validItems.length === 0) {
      msg.warning('pleaseAddReceiptDetail');
      return;
    }
    // 物料编码/名称/主单位来自产品档案（表单选产品时带入），缺失说明产品档案未维护
    // 计量单位或选择未生效——直接拦下暴露问题，禁止用 P{id}/物料{id}/'m' 伪值提交
    const broken = validItems.findIndex(
      it => !it.material_code || !it.material_name || !it.unit_master
    );
    if (broken >= 0) {
      msg.error('receiptItemMasterMissing', { line: broken + 1 });
      logger.error(
        `[入库明细] 第 ${broken + 1} 行物料主数据缺失：material_code=${validItems[broken].material_code} material_name=${validItems[broken].material_name} unit_master=${validItems[broken].unit_master} product_id=${validItems[broken].product_id}`
      );
      return;
    }
    // 批次号是后端 create_receipt / add_receipt_item → validate_receipt_item_dimensions
    // 建单期强校验的入库四维之一，缺任一行即整单 400；本独立入库页不关联采购订单行、
    // 产品主数据也不含批次，属收货实测维度，只能由操作人在对话框逐行录入，禁止塞假值蒙混建单。
    const missingBatch = validItems.find(it => !it.batch_no?.trim());
    if (missingBatch) {
      msg.warning('receiveBatchRequired');
      logger.warn(`[入库明细] 产品 ${missingBatch.product_id} 未录入批次号，四维不全，拒绝建单`);
      return;
    }
    // 染色布追溯口径（与后端 wave5l validate_fabric_trace 对齐）：色号非空 ⇒ 缸号必填。
    // 后端强制分支落地后此前端拦截即其前置镜像；未落地时也先行拦下，避免脏数据入库。
    const missingLot = validItems.find(it => it.color_code?.trim() && !it.lot_no?.trim());
    if (missingLot) {
      msg.warning('lotNoRequiredForColor');
      logger.warn(
        `[入库明细] 产品 ${missingLot.product_id} 已填色号（${missingLot.color_code}）但未填缸号，染色布追溯维度不全，拒绝建单`
      );
      return;
    }

    // 明细字段映射到后端 CreateReceiptItemRequest 契约（键名严格对齐 DTO：
    // material_id/material_code/material_name/batch_no/color_code/lot_no/grade/
    // line_no/quantity/quantity_alt/unit_master/unit_price）。
    // material_code/name/unit_master/batch_no 经上方校验保证存在，用非空断言收窄类型（
    // 非 ?? 兜底掩盖缺键——缺值已在校验阶段整单拦下，不会走到这里）。
    const mapItem = (it: ReceiptItem, idx: number): CreateReceiptItemRequest => ({
      line_no: idx + 1,
      material_id: it.product_id,
      material_code: it.material_code!,
      material_name: it.material_name!,
      batch_no: it.batch_no!.trim(),
      // 追溯维度：有值才传（后端 DTO 为 Option），空值不下发空串避免落库脏维度
      color_code: it.color_code?.trim() || undefined,
      lot_no: it.lot_no?.trim() || undefined,
      grade: it.grade?.trim() || undefined,
      quantity: it.quantity,
      quantity_alt: it.quantity_alt ?? 0,
      unit_master: it.unit_master!,
      unit_price: it.unit_price || undefined,
    });

    try {
      if (cb.form.id) {
        // 编辑：明细走 item 级端点（PUT /{id} 仅接受表头字段，原整单 PUT 会静默丢弃明细改动）
        await updatePurchaseReceipt(
          cb.form.id,
          cb.form as unknown as Partial<PurchaseReceiptEntity>
        );
        let idx = 0;
        for (const it of validItems) {
          if (it.id) {
            await updateReceiptItem(cb.form.id, it.id, mapItem(it, idx));
          } else {
            await createReceiptItem(cb.form.id, mapItem(it, idx));
          }
          idx += 1;
        }
        for (const itemId of removedItemIds.value) {
          await deleteReceiptItem(cb.form.id, itemId);
        }
        removedItemIds.value = [];
        msg.success('updateSuccess');
      } else {
        // 建单：显式组装 CreatePurchaseReceiptRequest，不再把整个 cb.form（含 id/status/receipt_no
        // 等响应/生成列）经 `as unknown as` 强塞进创建契约
        const payload: CreatePurchaseReceiptRequest = {
          supplier_id: cb.form.supplier_id as number,
          warehouse_id: cb.form.warehouse_id as number,
          receipt_date: cb.form.receipt_date as string,
          items: validItems.map(mapItem),
        };
        await createPurchaseReceipt(payload);
        msg.success('createSuccess');
      }
      cb.dialogVisible = false;
      await cb.loadData();
    } catch (error) {
      msg.error('operationFailed');
    }
  };

  /** 删除入库单 */
  const handleDelete = async (row: PurchaseReceiptEntity) => {
    // 行内删除按钮仅在 DRAFT 态渲染；此处二次防御：非草稿（已确认/已完成）视为已审核，禁止删除。
    // 比较值与后端 purchase_receipt.receipt_status 写入原值逐字一致。
    if (row.receipt_status !== PURCHASE_RECEIPT_STATUS.DRAFT) {
      msg.warning('auditedReceiptCannotDelete');
      return;
    }
    try {
      await ElMessageBox.confirm('确定要删除这个入库单吗？', '提示', { type: 'warning' });
      await deletePurchaseReceipt(row.id!);
      msg.success('deleteSuccess');
      await cb.loadData();
    } catch (error) {
      msg.info('deleteCancelled');
    }
  };

  /** 审核入库单 */
  const handleApprove = async (row: PurchaseReceiptEntity) => {
    try {
      await ElMessageBox.confirm('确定要审核这个入库单吗？', '提示', { type: 'warning' });
      await approvePurchaseReceipt(row.id!);
      msg.success('auditSuccess');
      await cb.loadData();
    } catch (error) {
      msg.info('operationCancelled');
    }
  };

  // 使用 reactive 包装，访问字段时自动解包 ref
  return reactive({
    handleSearch,
    handleReset,
    openAddDialog,
    openEditDialog,
    openViewDialog,
    addItem,
    removeItem,
    calculateItemAmount,
    handleSubmit,
    handleDelete,
    handleApprove,
  });
}

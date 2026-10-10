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
import { isDialogDismissal } from '@/utils/monitor';
import { ElMessageBox } from 'element-plus';
import { msg } from '@/utils/message';
import { i18n } from '@/i18n';
import {
  PURCHASE_RECEIPT_STATUS,
  PURCHASE_RECEIPT_INSPECTION_STATUS,
} from '@/utils/purchase-receipt-status';
import {
  createReceiptItem,
  updateReceiptItem,
  deleteReceiptItem,
  generatePurchaseReceiptNo,
  concedePurchaseReceipt,
  rejudgePurchaseReceipt,
} from '@/api/purchase-receipt';
import { getReceiptInspectionStatusLabel } from './prcFmts';

/** 纯 .ts 文案解析走 i18n.global.t（本仓约定：.vue 用 useI18n，.ts 用全局实例） */
const t = i18n.global.t;
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
  type UpdateReceiptItemRequest,
} from '@/api/purchase-receipt';
import type { PrcForm } from './usePrc';

/**
 * 十进制出参解析（rust_decimal 序列化输出十进制**字符串**，models/purchase_receipt_item.rs
 * 的 quantity/quantity_alt/unit_price/amount 均为 Decimal 列）：
 * 表单模型（el-input-number / 金额计算）只接受 number，加载边界必须显式解析，
 * 禁止 `?? 0` 把缺值/脏值伪装成合法数量。
 * - 必填（NOT NULL 列 quantity）：缺席/非法即抛错，由调用方拒绝回显并留痕——
 *   那是契约违例，不能静默渲染；
 * - 可空（DB nullable 列 quantity_alt/unit_price/amount）：null/undefined ⇒ undefined（未采集）；
 *   有值非法同样抛错留痕。
 */
function toRequiredDecimal(value: unknown, field: string, lineNo: unknown): number {
  if (value === null || value === undefined) {
    throw new Error(`入库明细第 ${String(lineNo)} 行缺少必填十进制列 ${field}`);
  }
  const n = Number(value);
  if (!Number.isFinite(n)) {
    throw new Error(`入库明细第 ${String(lineNo)} 行的 ${field} 不是合法十进制：${String(value)}`);
  }
  return n;
}

function toOptionalDecimal(value: unknown, field: string, lineNo: unknown): number | undefined {
  if (value === null || value === undefined) return undefined;
  const n = Number(value);
  if (!Number.isFinite(n)) {
    throw new Error(`入库明细第 ${String(lineNo)} 行的 ${field} 不是合法十进制：${String(value)}`);
  }
  return n;
}

/** 后端明细行 → 表单模型：十进制字符串显式转 number，可空列 null → undefined（未采集，提交时按三态处理） */
function normalizeReceiptItemForForm(it: ReceiptItem): ReceiptItem {
  return {
    ...it,
    quantity: toRequiredDecimal(it.quantity, 'quantity', it.line_no),
    quantity_alt: toOptionalDecimal(it.quantity_alt, 'quantity_alt', it.line_no),
    unit_price: toOptionalDecimal(it.unit_price, 'unit_price', it.line_no),
    amount: toOptionalDecimal(it.amount, 'amount', it.line_no),
  };
}

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
    // 编辑态残留（已删行 / 辅量原值表）随新建作废
    removedItemIds.value = [];
    altQtyOriginals.value.clear();
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
          // 辅量不预置 0：0 是「实测为 0」，未录入必须保持未采集（undefined）
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
    try {
      const res = await getPurchaseReceipt(row.id!);
      const itemsRes = await getReceiptItems(row.id!);
      // 十进制字符串出参在加载边界显式解析为 number（详见 normalizeReceiptItemForForm）
      const items = itemsRes.data.map(normalizeReceiptItemForForm);
      // 记录各行辅量原值，供提交时三态判定（清空→显式 null / 未动→原值 / 未采集→省略键）
      altQtyOriginals.value = new Map(
        items
          .filter(i => i.id != null)
          .map(i => [i.id as number, i.quantity_alt == null ? null : i.quantity_alt])
      );
      cb.form = { ...(res.data as unknown as PrcForm), items };
      cb.dialogVisible = true;
    } catch (error) {
      logger.error(`[入库编辑] 入库单 ${row.receipt_no} 明细回显失败（十进制解析契约违例）`, error);
      msg.error('loadDetailFailed');
    }
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
      // 辅量保持未采集（undefined）：预置 0 会把「没量过」伪装成「量过且为 0」
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
  /**
   * 编辑态各行辅量「原值」快照（openEditDialog 写入）：
   * number=库里已采集值；null=该行原本未采集。提交时与当前值对比重建三态。
   */
  const altQtyOriginals = ref<Map<number, number | null>>(new Map());
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

    // 辅量（quantity_alt）录入拦截：创建契约中该键为**非 Option 必填**
    // （backend/src/services/purchase_receipt_dto 的 CreateReceiptItemRequest：
    // `pub quantity_alt: Decimal` 非 Option，缺键=反序列化直接拒绝），故新建单行与编辑态追加分录行必须由操作人实测录入——
    // 未采集不得塌成 0（0 会被累加进 received_quantity_alt/total_quantity_alt 成为假量，
    // 正是本批要消灭的辅量断链镜像）。三态中的「留空省略键」仅更新契约可表达，见 mapItemUpdate。
    const isCreateRow = (it: ReceiptItem) => !cb.form.id || it.id == null;
    const missingAltIndex = validItems.findIndex(
      it => isCreateRow(it) && (it.quantity_alt == null || !Number.isFinite(it.quantity_alt))
    );
    if (missingAltIndex >= 0) {
      msg.warning('receiptItemAltQtyRequired', { line: missingAltIndex + 1 });
      logger.warn(
        `[入库明细] 第 ${missingAltIndex + 1} 行未录入辅助数量（创建契约必填键 quantity_alt），拒绝提交`
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
      // 非空断言：创建路径的辅量已由上方拦截保证已采集（quantity_alt 必填键，禁止 ?? 0 伪装）
      quantity_alt: it.quantity_alt!,
      unit_master: it.unit_master!,
      unit_price: it.unit_price || undefined,
    });

    // 更新明细映射到后端 UpdateReceiptItemRequest 三态契约（RFC 7386）：
    // NOT NULL 列 line_no/material_id/material_code/material_name/quantity 恒送值
    //（送 null 被后端 400「XX不能清空」拒绝）；DB 可空列 batch_no/color_code/lot_no/
    // grade/unit_price UI 清空 ⇒ 送显式 null（=清空为 NULL）——沿用 mapItem 的
    // `|| undefined` 会把"清空维度"塌成"保持原值"，正是本轮消灭的静默丢弃形态。
    // quantity_alt 按三态细分（m0009 可空列 + UpdateReceiptItemRequest double_option :236-238）：
    // 有值=覆盖；清空且原值已采集=显式 null；原本未采集且仍留空=**省略键**（不送 0、不送 null 假清空）。
    const mapItemUpdate = (it: ReceiptItem, idx: number): UpdateReceiptItemRequest => {
      const patch: UpdateReceiptItemRequest = {
        line_no: idx + 1,
        material_id: it.product_id,
        material_code: it.material_code!,
        material_name: it.material_name!,
        batch_no: it.batch_no!.trim() || null,
        color_code: it.color_code?.trim() || null,
        lot_no: it.lot_no?.trim() || null,
        grade: it.grade?.trim() || null,
        quantity: it.quantity,
        unit_price: it.unit_price ?? null,
      };
      const original = it.id != null ? altQtyOriginals.value.get(it.id) : undefined;
      if (it.quantity_alt != null && Number.isFinite(it.quantity_alt)) {
        patch.quantity_alt = it.quantity_alt;
      } else if (original != null) {
        patch.quantity_alt = null;
      }
      // original == null 且当前留空 ⇒ 未采集，省略 quantity_alt 键
      return patch;
    };

    try {
      if (cb.form.id) {
        // 编辑：表头 PUT 仅提交 UpdatePurchaseReceiptRequest 契约内的字段（此前把整个 cb.form
        // ——含 id/receipt_no/warehouse_id/status/items 等非契约键——强转提交，全部被 serde 丢弃，
        // 属假保存）；明细改动走 item 级端点（POST/PUT/DELETE /{id}/items）。
        // warehouse_id 不在更新契约（后端更新 DTO 无该列，换仓需后端支持，已登记后端串行清单）。
        await updatePurchaseReceipt(cb.form.id, {
          supplier_id: cb.form.supplier_id as number,
          receipt_date: cb.form.receipt_date as string,
        });
        let idx = 0;
        for (const it of validItems) {
          if (it.id) {
            await updateReceiptItem(cb.form.id, it.id, mapItemUpdate(it, idx));
          } else {
            await createReceiptItem(cb.form.id, mapItem(it, idx));
          }
          idx += 1;
        }
        for (const itemId of removedItemIds.value) {
          await deleteReceiptItem(cb.form.id, itemId);
        }
        removedItemIds.value = [];
        altQtyOriginals.value.clear();
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

  /**
   * 让步接收（行内入口）：对**待检/质检不合格**的收货单显式办理特采降级接收。
   * 合法前驱由后端状态机把关（PENDING/REJECTED → CONCESSION_ACCEPTED，非法前驱 400）；
   * 理由动态必填——prompt 空/纯空白即前端拦下（inputValidator），不提交半态。
   * 操作人不进请求体（后端取会话身份），前端无需要求。让步后仍不可入库，需复检改判。
   */
  const handleConcede = async (row: PurchaseReceiptEntity) => {
    if (row.id == null) {
      logger.error('[让步接收] 行缺少 id，拒绝打开理由输入');
      msg.error('operationFailed');
      return;
    }
    // 前端预门控：仅待检/不合格可让步（与后端合法前驱逐字一致），非法前驱不发起无谓请求。
    if (
      row.inspection_status !== PURCHASE_RECEIPT_INSPECTION_STATUS.PENDING &&
      row.inspection_status !== PURCHASE_RECEIPT_INSPECTION_STATUS.REJECTED
    ) {
      msg.warning('concessionIllegalPredecessor');
      return;
    }
    let reason: string;
    try {
      const res = await ElMessageBox.prompt(
        `${t('purchaseReceipt.concession.promptPrefix')}${getReceiptInspectionStatusLabel(
          row.inspection_status
        )}`,
        t('purchaseReceipt.concession.promptTitle'),
        {
          inputType: 'textarea',
          inputPlaceholder: t('purchaseReceipt.concession.reasonPlaceholder'),
          inputValidator: (v: string) =>
            (v ?? '').trim().length > 0 || t('purchaseReceipt.concession.reasonRequired'),
        }
      );
      reason = (res.value ?? '').trim();
    } catch (error: unknown) {
      // 取消/关闭是用户主动放弃，不是错误
      if (isDialogDismissal(error)) return;
      logger.error('[让步接收] 理由输入异常', error);
      msg.error('operationFailed');
      return;
    }
    try {
      await concedePurchaseReceipt(row.id, { reason });
      msg.success('concessionSuccess');
      await cb.loadData();
    } catch (error: unknown) {
      // 非 2xx 由响应拦截器外显后端 message（状态门/字段校验文案），此处仅记日志
      logger.error(`[让步接收] 入库单 ${row.receipt_no} 让步失败`, error);
    }
  };

  /**
   * 复检改判（行内入口）：把让步态的**同一张收货单**显式改判为合格/不合格。
   * 结论取值经对话框选择（合格→pass / 不合格→fail），与后端质检结论权威词表同源；
   * 理由动态必填同上；改判前后状态与操作人由后端落专用列 + 审计快照，可回读。
   */
  const handleRejudge = async (row: PurchaseReceiptEntity) => {
    if (row.id == null) {
      logger.error('[复检改判] 行缺少 id，拒绝打开改判输入');
      msg.error('operationFailed');
      return;
    }
    // 前端预门控：仅让步态可改判（后端 CONCESSION_ACCEPTED → PASSED/REJECTED）
    if (row.inspection_status !== PURCHASE_RECEIPT_INSPECTION_STATUS.CONCESSION_ACCEPTED) {
      msg.warning('rejudgeIllegalPredecessor');
      return;
    }
    let payload: { inspection_result: string; reason: string };
    try {
      const selected = await ElMessageBox.confirm(
        `${t('purchaseReceipt.rejudge.promptPrefix')}${row.concession_reason ?? ''}`,
        t('purchaseReceipt.rejudge.title'),
        {
          confirmButtonText: t('purchaseReceipt.rejudge.toPassed'),
          cancelButtonText: t('purchaseReceipt.rejudge.toRejected'),
          distinguishCancelAndClose: true,
          type: 'warning',
        }
      );
      // 走到这里说明点了「改判为合格」
      payload = { inspection_result: 'pass', reason: '' };
      void selected;
    } catch (error: unknown) {
      // 点「改判为不合格」→ reject 'cancel'；关闭/取消 → reject 'close'/其他
      if (error !== 'cancel') {
        if (isDialogDismissal(error)) return;
        logger.error('[复检改判] 结论选择异常', error);
        msg.error('operationFailed');
        return;
      }
      payload = { inspection_result: 'fail', reason: '' };
    }
    // 理由动态必填（改判两向均要求，落 rejudge_reason 专用列）
    let reason: string;
    try {
      const res = await ElMessageBox.prompt(
        t('purchaseReceipt.rejudge.reasonPrefix'),
        t('purchaseReceipt.rejudge.title'),
        {
          inputType: 'textarea',
          inputPlaceholder: t('purchaseReceipt.rejudge.reasonPlaceholder'),
          inputValidator: (v: string) =>
            (v ?? '').trim().length > 0 || t('purchaseReceipt.rejudge.reasonRequired'),
        }
      );
      reason = (res.value ?? '').trim();
    } catch (error: unknown) {
      if (isDialogDismissal(error)) return;
      logger.error('[复检改判] 理由输入异常', error);
      msg.error('operationFailed');
      return;
    }
    payload.reason = reason;
    try {
      await rejudgePurchaseReceipt(row.id, payload);
      msg.success('rejudgeSuccess');
      await cb.loadData();
    } catch (error: unknown) {
      logger.error(`[复检改判] 入库单 ${row.receipt_no} 改判失败`, error);
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
    handleConcede,
    handleRejudge,
  });
}

<template>
  <div class="page">
    <el-card shadow="never">
      <el-tabs v-model="activeTab">
        <el-tab-pane :label="$t('outsourcing.tabs.orders')" name="orders">
          <div class="toolbar mb">
            <el-button type="primary" @click="openCreate">{{
              $t('outsourcing.actions.createOrder')
            }}</el-button>
          </div>
          <el-table v-loading="loading" :data="orders" border>
            <el-table-column
              prop="order_no"
              :label="$t('outsourcing.columns.orderNo')"
              width="150"
            />
            <el-table-column
              prop="order_type"
              :label="$t('outsourcing.columns.orderType')"
              width="100"
            />
            <el-table-column :label="$t('common.status')" width="110">
              <template #default="{ row }">
                <el-tag :type="outsourcingStatusTagType(row.status)">{{
                  $t(outsourcingStatusLabelKey(row.status))
                }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column
              prop="supplier_id"
              :label="$t('outsourcing.columns.supplierId')"
              width="100"
            />
            <el-table-column
              prop="issue_date"
              :label="$t('outsourcing.columns.issueDate')"
              width="120"
            />
            <el-table-column
              prop="expected_return_date"
              :label="$t('outsourcing.columns.expectedReturnDate')"
              width="120"
            />
            <el-table-column
              prop="issue_quantity"
              :label="$t('outsourcing.columns.issueQuantity')"
              width="110"
            />
            <el-table-column :label="$t('common.operation')" width="340" fixed="right">
              <template #default="{ row }">
                <el-button size="small" @click="openDetail(row)">{{
                  $t('common.detail')
                }}</el-button>
                <el-button
                  v-if="row.status === OUTSOURCING_ORDER_STATUS.draft"
                  size="small"
                  @click="openEdit(row)"
                  >{{ $t('common.edit') }}</el-button
                >
                <el-button
                  v-if="row.status === OUTSOURCING_ORDER_STATUS.draft"
                  size="small"
                  type="primary"
                  @click="onIssue(row)"
                  >{{ $t('outsourcing.actions.issue') }}</el-button
                >
                <el-button
                  v-if="row.status === OUTSOURCING_ORDER_STATUS.issued"
                  size="small"
                  type="primary"
                  @click="onProcess(row)"
                  >{{ $t('outsourcing.actions.markProcessing') }}</el-button
                >
                <!-- 结算入口与后端 settle 两道硬拒同口径（提前提示，不替代后端校验，
                     后端拒绝原因仍由失败信封正常外显）：状态门 order.rs:594-597 仅 received；
                     费用门 order.rs:607-611 processing_fee+freight_fee<=0 拒 400 -->
                <el-tooltip
                  v-if="row.status === OUTSOURCING_ORDER_STATUS.received"
                  :content="$t('outsourcing.gate.zeroFeeSettleTip')"
                  placement="top"
                  :disabled="!isZeroFeeOrder(row)"
                >
                  <span>
                    <el-button
                      size="small"
                      type="success"
                      :disabled="isZeroFeeOrder(row)"
                      @click="onSettle(row)"
                      >{{ $t('outsourcing.actions.settle') }}</el-button
                    >
                  </span>
                </el-tooltip>
                <el-button
                  v-if="row.status === OUTSOURCING_ORDER_STATUS.settled"
                  size="small"
                  type="success"
                  plain
                  @click="onClose(row)"
                  >{{ $t('common.close') }}</el-button
                >
                <el-button
                  v-if="row.status === OUTSOURCING_ORDER_STATUS.draft"
                  size="small"
                  type="danger"
                  plain
                  @click="onCancel(row)"
                  >{{ $t('common.cancel') }}</el-button
                >
                <el-button
                  v-if="row.status === OUTSOURCING_ORDER_STATUS.draft"
                  size="small"
                  type="danger"
                  @click="onDelete(row)"
                  >{{ $t('common.delete') }}</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>

        <el-tab-pane :label="$t('outsourcing.tabs.receipts')" name="receipts">
          <div class="toolbar mb">
            <el-button type="primary" @click="openCreateReceipt">{{
              $t('outsourcing.actions.createReceipt')
            }}</el-button>
            <el-button plain @click="loadReceipts">{{ $t('common.refresh') }}</el-button>
          </div>
          <el-table v-loading="receiptLoading" :data="receipts" border>
            <el-table-column prop="id" label="ID" width="70" />
            <el-table-column
              prop="receipt_no"
              :label="$t('outsourcing.columns.receiptNo')"
              width="160"
            />
            <el-table-column
              prop="outsourcing_order_id"
              :label="$t('outsourcing.columns.outsourcingOrderId')"
              width="100"
            />
            <el-table-column
              prop="receipt_date"
              :label="$t('outsourcing.columns.receiptDate')"
              width="120"
            />
            <el-table-column
              prop="product_id"
              :label="$t('outsourcing.columns.productId')"
              width="90"
            />
            <el-table-column
              prop="return_quantity"
              :label="$t('outsourcing.columns.returnQuantity')"
              width="110"
            />
            <el-table-column
              prop="loss_quantity"
              :label="$t('outsourcing.columns.lossQuantity')"
              width="110"
            />
            <el-table-column :label="$t('outsourcing.columns.qualityStatus')" width="110">
              <template #default="{ row }">
                <el-tag :type="qualityTagType(row.quality_status)" size="small">
                  {{ qualityLabel(row.quality_status) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="grade" :label="$t('outsourcing.columns.grade')" width="80" />
            <!-- 打卷实测值三列（成品布入库标签数据源）：出参为 Decimal 字符串，
                 未录入（null）显示「未补录」占位文案，不显示空/0（0 属伪造实测值） -->
            <el-table-column :label="$t('outsourcing.receipt.measured.weight')" width="110">
              <template #default="{ row }">{{ measuredCellText(row.weight) }}</template>
            </el-table-column>
            <el-table-column :label="$t('outsourcing.receipt.measured.width')" width="110">
              <template #default="{ row }">{{ measuredCellText(row.width) }}</template>
            </el-table-column>
            <el-table-column :label="$t('outsourcing.receipt.measured.gramWeight')" width="110">
              <template #default="{ row }">{{ measuredCellText(row.gram_weight) }}</template>
            </el-table-column>
            <el-table-column :label="$t('common.status')" width="100">
              <template #default="{ row }">
                <el-tag :type="row.status === 'confirmed' ? 'success' : 'info'">{{
                  row.status
                }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column :label="$t('common.operation')" width="180" fixed="right">
              <template #default="{ row }">
                <!-- 确认入口与后端 confirm 两道硬拒同口径（提前提示，不替代后端校验）：
                     状态门 receipt.rs:326-331 仅 draft；数量门 receipt.rs:338-342
                     return_quantity<=0 拒 400（存量 0 量草稿同样拦） -->
                <el-tooltip
                  v-if="row.status === 'draft'"
                  :content="$t('outsourcing.gate.zeroQtyConfirmTip')"
                  placement="top"
                  :disabled="!isZeroQtyReceipt(row)"
                >
                  <span>
                    <el-button
                      size="small"
                      type="success"
                      :disabled="isZeroQtyReceipt(row)"
                      @click="onConfirmReceipt(row)"
                      >{{ $t('common.confirm') }}</el-button
                    >
                  </span>
                </el-tooltip>
                <!-- 补录打卷实测值：可见性与本页既有收回单操作按钮同款写法（状态门控、
                     无独立权限键）；后端 update 状态门 receipt.rs:316-321 仅 draft，
                     confirmed 后实测值已随确认透传进匹行，本入口不再可达属预期 -->
                <el-button
                  v-if="row.status === 'draft'"
                  size="small"
                  @click="openMeasuredDialog(row)"
                  >{{ $t('outsourcing.receipt.measured.editTitle') }}</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>
      </el-tabs>
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="
        editingId ? $t('outsourcing.dialog.editOrder') : $t('outsourcing.actions.createOrder')
      "
      width="560"
    >
      <el-form ref="orderFormRef" :model="form" :rules="orderRules" label-width="110px">
        <el-form-item v-if="!editingId" :label="$t('outsourcing.columns.orderNo')" required>
          <el-input v-model="form.order_no" readonly />
        </el-form-item>
        <el-form-item :label="$t('outsourcing.columns.orderType')" required>
          <el-select v-model="form.order_type" class="w-full">
            <el-option :label="$t('outsourcing.orderTypes.dyeing')" value="dyeing" />
            <el-option :label="$t('outsourcing.orderTypes.finishing')" value="finishing" />
            <el-option :label="$t('outsourcing.orderTypes.other')" value="other" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('outsourcing.form.supplier')" required>
          <el-select
            v-model="form.supplier_id"
            filterable
            :filter-method="filterSupplierOptions"
            :placeholder="$t('outsourcing.form.placeholderSelectSupplier')"
            class="w-full"
          >
            <el-option
              v-for="s in visibleSupplierOptions"
              :key="s.id"
              :label="supplierLabel(s)"
              :value="s.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('outsourcing.columns.issueDate')" required>
          <el-date-picker
            v-model="form.issue_date"
            type="date"
            value-format="YYYY-MM-DD"
            class="w-full"
          />
        </el-form-item>
        <el-form-item :label="$t('outsourcing.columns.expectedReturnDate')">
          <el-date-picker
            v-model="form.expected_return_date"
            type="date"
            value-format="YYYY-MM-DD"
            class="w-full"
          />
        </el-form-item>
        <el-form-item
          :label="$t('outsourcing.columns.issueQuantity')"
          prop="issue_quantity"
          required
        >
          <el-input v-model="form.issue_quantity" inputmode="decimal" class="w-full" />
        </el-form-item>
        <el-form-item :label="$t('outsourcing.columns.unit')">
          <el-input v-model="form.issue_unit" placeholder="kg / m" />
        </el-form-item>
        <el-form-item :label="$t('outsourcing.form.materialCost')" prop="material_cost" required>
          <el-input
            v-model="form.material_cost"
            inputmode="decimal"
            :placeholder="$t('outsourcing.formPlaceholders.materialCost')"
            class="w-full"
          />
        </el-form-item>
        <!-- 三费录入（缺陷②修复）：outsourcing_order NOT NULL 列（后端 v15:3247-3249），
             建单必填可填 0；结算 FEE 凭证金额=加工费+运费，不录入成本链恒 0 -->
        <el-form-item
          :label="$t('outsourcing.columns.processingFee')"
          prop="processing_fee"
          required
        >
          <el-input
            v-model="form.processing_fee"
            inputmode="decimal"
            :placeholder="$t('outsourcing.formPlaceholders.processingFee')"
            class="w-full"
          />
        </el-form-item>
        <el-form-item :label="$t('outsourcing.columns.freightFee')" prop="freight_fee" required>
          <el-input
            v-model="form.freight_fee"
            inputmode="decimal"
            :placeholder="$t('outsourcing.formPlaceholders.freightFee')"
            class="w-full"
          />
        </el-form-item>
        <el-form-item :label="$t('outsourcing.columns.taxAmount')" prop="tax_amount" required>
          <el-input
            v-model="form.tax_amount"
            inputmode="decimal"
            :placeholder="$t('outsourcing.formPlaceholders.taxAmount')"
            class="w-full"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ $t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="saving" @click="onSave">{{
          editingId ? $t('common.update') : $t('common.save')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="detailVisible" :title="$t('outsourcing.dialog.orderDetail')" width="820">
      <el-descriptions v-if="detailOrder" :column="2" border>
        <el-descriptions-item :label="$t('outsourcing.columns.orderNo')">{{
          detailOrder.order_no
        }}</el-descriptions-item>
        <el-descriptions-item :label="$t('outsourcing.columns.orderType')">{{
          detailOrder.order_type
        }}</el-descriptions-item>
        <el-descriptions-item :label="$t('common.status')">{{
          $t(outsourcingStatusLabelKey(detailOrder.status))
        }}</el-descriptions-item>
        <el-descriptions-item :label="$t('outsourcing.columns.supplierId')">{{
          detailOrder.supplier_id
        }}</el-descriptions-item>
        <el-descriptions-item :label="$t('outsourcing.columns.issueDate')">{{
          detailOrder.issue_date
        }}</el-descriptions-item>
        <el-descriptions-item :label="$t('outsourcing.columns.expectedReturnDate')">{{
          detailOrder.expected_return_date || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="$t('outsourcing.columns.issueQuantity')">{{
          detailOrder.issue_quantity
        }}</el-descriptions-item>
        <el-descriptions-item :label="$t('outsourcing.columns.unit')">{{
          detailOrder.issue_unit || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="$t('outsourcing.form.materialCost')">{{
          detailOrder.material_cost
        }}</el-descriptions-item>
        <!-- 三费与成本链回显（后端 outsourcing_order 真实 NOT NULL 列，键恒在，Decimal 出参为字符串） -->
        <el-descriptions-item :label="$t('outsourcing.columns.processingFee')">{{
          detailOrder.processing_fee
        }}</el-descriptions-item>
        <el-descriptions-item :label="$t('outsourcing.columns.freightFee')">{{
          detailOrder.freight_fee
        }}</el-descriptions-item>
        <el-descriptions-item :label="$t('outsourcing.columns.taxAmount')">{{
          detailOrder.tax_amount
        }}</el-descriptions-item>
        <el-descriptions-item :label="$t('outsourcing.columns.totalCost')">{{
          detailOrder.total_cost
        }}</el-descriptions-item>
      </el-descriptions>

      <div class="items-section">
        <div class="items-toolbar">
          <span class="items-title">{{ $t('outsourcing.dialog.itemsTitle') }}</span>
          <el-button
            v-if="detailOrder && detailOrder.status === OUTSOURCING_ORDER_STATUS.draft"
            type="primary"
            size="small"
            @click="itemDialogVisible = true"
            >{{ $t('outsourcing.actions.addItem') }}</el-button
          >
        </div>
        <el-table v-loading="itemLoading" :data="orderItems" border size="small" max-height="300">
          <el-table-column prop="id" label="ID" width="60" />
          <el-table-column
            prop="product_id"
            :label="$t('outsourcing.columns.productId')"
            width="90"
          />
          <el-table-column prop="color_no" :label="$t('outsourcing.columns.colorNo')" width="110" />
          <el-table-column
            prop="dye_lot_no"
            :label="$t('outsourcing.columns.dyeLotNo')"
            width="110"
          />
          <el-table-column prop="piece_no" :label="$t('outsourcing.columns.pieceNo')" width="120" />
          <el-table-column
            prop="quantity"
            :label="$t('outsourcing.columns.quantity')"
            width="100"
          />
          <el-table-column prop="unit" :label="$t('outsourcing.columns.unit')" width="80" />
          <el-table-column
            prop="unit_cost"
            :label="$t('outsourcing.columns.unitCost')"
            width="100"
          />
          <el-table-column
            prop="processing_fee"
            :label="$t('outsourcing.columns.processingFee')"
            width="100"
          />
          <el-table-column
            prop="freight_fee"
            :label="$t('outsourcing.columns.freightFee')"
            width="100"
          />
        </el-table>
      </div>
      <template #footer>
        <el-button @click="detailVisible = false">{{ $t('common.close') }}</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="itemDialogVisible"
      :title="$t('outsourcing.dialog.addItemTitle')"
      width="520"
    >
      <el-form ref="itemFormRef" :model="itemForm" :rules="itemRules" label-width="110px">
        <el-form-item :label="$t('outsourcing.columns.productId')" prop="product_id" required>
          <el-input v-model="itemForm.product_id" inputmode="numeric" class="w-full" />
        </el-form-item>
        <el-form-item :label="$t('outsourcing.columns.colorNo')"
          ><el-input v-model="itemForm.color_no"
        /></el-form-item>
        <el-form-item :label="$t('outsourcing.columns.dyeLotNo')"
          ><el-input v-model="itemForm.dye_lot_no"
        /></el-form-item>
        <!-- 匹号：委外发料逐条引用真实可用生产匹，选项按产品从 GET /inventory/pieces 下推
             取回（生产匹 + 可用），不由前端手填或自行推断 -->
        <el-form-item :label="$t('outsourcing.columns.pieceNo')" required>
          <el-select
            v-model="itemForm.piece_no"
            :placeholder="$t('outsourcing.formPlaceholders.pieceNo')"
            :disabled="!itemForm.product_id"
            class="w-full"
            @visible-change="(v: boolean) => v && void loadProductionPieces()"
          >
            <el-option
              v-for="p in pieceOptions"
              :key="p.piece_no"
              :label="`${$t('outsourcing.columns.pieceNo')}: ${p.piece_no} · ${$t('outsourcing.columns.pieceLength')}: ${p.length}`"
              :value="p.piece_no"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('outsourcing.columns.quantity')" prop="quantity" required>
          <el-input v-model="itemForm.quantity" inputmode="decimal" class="w-full" />
        </el-form-item>
        <el-form-item :label="$t('outsourcing.columns.unit')"
          ><el-input v-model="itemForm.unit" placeholder="kg / m"
        /></el-form-item>
        <el-form-item :label="$t('outsourcing.columns.unitCost')" prop="unit_cost" required>
          <el-input v-model="itemForm.unit_cost" inputmode="decimal" class="w-full" />
        </el-form-item>
        <el-form-item :label="$t('outsourcing.columns.processingFee')" prop="processing_fee">
          <el-input v-model="itemForm.processing_fee" inputmode="decimal" class="w-full" />
        </el-form-item>
        <el-form-item :label="$t('outsourcing.columns.freightFee')" prop="freight_fee">
          <el-input v-model="itemForm.freight_fee" inputmode="decimal" class="w-full" />
        </el-form-item>
        <el-form-item :label="$t('outsourcing.columns.remarks')"
          ><el-input v-model="itemForm.remarks" type="textarea" :rows="2"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="itemDialogVisible = false">{{ $t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="itemSaving" @click="onSaveItem">{{
          $t('common.save')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="receiptDialogVisible"
      :title="$t('outsourcing.actions.createReceipt')"
      width="560"
    >
      <el-form ref="receiptFormRef" :model="receiptForm" :rules="receiptRules" label-width="110px">
        <el-form-item :label="$t('outsourcing.columns.receiptNo')" required>
          <el-input v-model="receiptForm.receipt_no" readonly />
        </el-form-item>
        <el-form-item
          :label="$t('outsourcing.columns.outsourcingOrderId')"
          prop="outsourcing_order_id"
          required
        >
          <el-input v-model="receiptForm.outsourcing_order_id" inputmode="numeric" class="w-full" />
        </el-form-item>
        <el-form-item :label="$t('outsourcing.columns.receiptDate')" required>
          <el-date-picker
            v-model="receiptForm.receipt_date"
            type="date"
            value-format="YYYY-MM-DD"
            class="w-full"
          />
        </el-form-item>
        <el-form-item :label="$t('outsourcing.columns.productId')" prop="product_id" required>
          <el-input v-model="receiptForm.product_id" inputmode="numeric" class="w-full" />
        </el-form-item>
        <el-form-item :label="$t('outsourcing.columns.colorNo')"
          ><el-input v-model="receiptForm.color_no"
        /></el-form-item>
        <el-form-item :label="$t('outsourcing.columns.dyeLotNo')"
          ><el-input v-model="receiptForm.dye_lot_no"
        /></el-form-item>
        <el-form-item
          :label="$t('outsourcing.columns.returnQuantity')"
          prop="return_quantity"
          required
        >
          <el-input v-model="receiptForm.return_quantity" inputmode="decimal" class="w-full" />
        </el-form-item>
        <el-form-item :label="$t('outsourcing.columns.lossQuantity')" prop="loss_quantity">
          <el-input v-model="receiptForm.loss_quantity" inputmode="decimal" class="w-full" />
        </el-form-item>
        <el-form-item :label="$t('outsourcing.columns.qualityStatus')">
          <el-select v-model="receiptForm.quality_status" class="w-full">
            <el-option
              v-for="value in OUTSOURCING_QUALITY_FORM_VALUES"
              :key="value"
              :label="OUTSOURCING_QUALITY_STATUS_LABELS[value]"
              :value="value"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('outsourcing.columns.grade')">
          <el-input v-model="receiptForm.grade" placeholder="A / B / C" />
        </el-form-item>
        <!-- 打卷实测值（成品布入库标签数据源）：DB 可空列，可留空不录入；
             建单直写口径（CreateOutsourcingReceiptRequest types.rs:230-234），
             留空=落 NULL（未补录），填 0/负数本地按 >0 拦下（同后端 validate_measured_values） -->
        <el-divider content-position="left">{{
          $t('outsourcing.receipt.measuredTitle')
        }}</el-divider>
        <el-form-item :label="$t('outsourcing.receipt.measured.weight')" prop="weight">
          <el-input
            v-model="receiptForm.weight"
            inputmode="decimal"
            :placeholder="$t('outsourcing.receipt.measured.weightRequired')"
            class="w-full"
          />
        </el-form-item>
        <el-form-item :label="$t('outsourcing.receipt.measured.width')" prop="width">
          <el-input
            v-model="receiptForm.width"
            inputmode="decimal"
            :placeholder="$t('outsourcing.receipt.measured.widthRequired')"
            class="w-full"
          />
        </el-form-item>
        <el-form-item :label="$t('outsourcing.receipt.measured.gramWeight')" prop="gram_weight">
          <el-input
            v-model="receiptForm.gram_weight"
            inputmode="decimal"
            :placeholder="$t('outsourcing.receipt.measured.gramWeightRequired')"
            class="w-full"
          />
        </el-form-item>
        <div class="measured-hint">{{ $t('outsourcing.receipt.measured.hint') }}</div>
        <el-form-item :label="$t('outsourcing.columns.remarks')"
          ><el-input v-model="receiptForm.remarks" type="textarea" :rows="2"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="receiptDialogVisible = false">{{ $t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="receiptSaving" @click="onSaveReceipt">{{
          $t('common.save')
        }}</el-button>
      </template>
    </el-dialog>

    <!-- 补录打卷实测值（仅 draft，后端 update 状态门 receipt.rs:316-321）：
         三态提交——改动/录入=覆盖、清空已有值=显式 null 清空回未补录、未动=键缺席保持 -->
    <el-dialog
      v-model="measuredDialogVisible"
      :title="$t('outsourcing.receipt.measured.editTitle')"
      width="560"
    >
      <el-form
        ref="measuredFormRef"
        :model="measuredForm"
        :rules="measuredRules"
        label-width="110px"
      >
        <el-form-item :label="$t('outsourcing.receipt.measured.weight')" prop="weight">
          <el-input
            v-model="measuredForm.weight"
            inputmode="decimal"
            :placeholder="$t('outsourcing.receipt.measured.weightRequired')"
            class="w-full"
          />
        </el-form-item>
        <el-form-item :label="$t('outsourcing.receipt.measured.width')" prop="width">
          <el-input
            v-model="measuredForm.width"
            inputmode="decimal"
            :placeholder="$t('outsourcing.receipt.measured.widthRequired')"
            class="w-full"
          />
        </el-form-item>
        <el-form-item :label="$t('outsourcing.receipt.measured.gramWeight')" prop="gram_weight">
          <el-input
            v-model="measuredForm.gram_weight"
            inputmode="decimal"
            :placeholder="$t('outsourcing.receipt.measured.gramWeightRequired')"
            class="w-full"
          />
        </el-form-item>
        <div class="measured-hint">{{ $t('outsourcing.receipt.measured.hint') }}</div>
      </el-form>
      <template #footer>
        <el-button @click="measuredDialogVisible = false">{{ $t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="measuredSaving" @click="onSaveMeasured">{{
          $t('common.save')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';
import { generateUniqueDocNo } from '@/utils/document-no';
import { logAuxLoadFailure, logger } from '@/utils/logger';
import { numberInputRules, numericView } from '@/utils/number-input';
import { msg } from '@/utils/message';
import { isDialogDismissal } from '@/utils/monitor';
import {
  OUTSOURCING_ORDER_STATUS,
  outsourcingStatusLabelKey,
  outsourcingStatusTagType,
} from '@/utils/outsourcing-status';
import {
  OUTSOURCING_QUALITY_FORM_VALUES,
  OUTSOURCING_QUALITY_STATUS,
  OUTSOURCING_QUALITY_STATUS_LABELS,
  OUTSOURCING_QUALITY_STATUS_TAG_TYPE,
  OUTSOURCING_QUALITY_STATUS_VALUES,
  type OutsourcingQualityTagType,
} from '@/constants/outsourcing-quality';
import {
  cancelOutsourcingOrder,
  closeOutsourcingOrder,
  createOutsourcingOrder,
  getOutsourcingOrderList,
  issueOutsourcingOrder,
  processOutsourcingOrder,
  settleOutsourcingOrder,
  updateOutsourcingOrder,
  deleteOutsourcingOrder,
  getOutsourcingItems,
  createOutsourcingItem,
  getOutsourcingReceiptList,
  createOutsourcingReceipt,
  updateOutsourcingReceipt,
  confirmOutsourcingReceipt,
  type OutsourcingOrder,
  type OutsourcingOrderItem,
  type OutsourcingReceipt,
  type UpdateOutsourcingReceiptPayload,
} from '@/api/outsourcing';
import {
  getAvailablePieces,
  INVENTORY_PIECE_STATUS,
  PIECE_TYPE,
  type InventoryPieceRow,
} from '@/api/inventory-transfer';
import { getSupplierList, type Supplier } from '@/api/supplier';

const { t } = useI18n();

/**
 * 收回质检结论展示：结论为空只可能出现在未归一的历史行上（v15 域内的归一语句 已把
 * NULL 归一为 pending），统一按「待检」显示；取值域外的值告警后原样显示。
 */
function qualityLabel(value: string | null | undefined): string {
  const raw = value ?? OUTSOURCING_QUALITY_STATUS.pending;
  const label = OUTSOURCING_QUALITY_STATUS_LABELS[raw];
  if (!label) {
    logger.warn(
      `[outsourcing] 未知收回质检结论「${raw}」，不在取值域 ${OUTSOURCING_QUALITY_STATUS_VALUES.join('/')} 内`
    );
    return raw;
  }
  return label;
}

function qualityTagType(value: string | null | undefined): OutsourcingQualityTagType {
  const raw = value ?? OUTSOURCING_QUALITY_STATUS.pending;
  return OUTSOURCING_QUALITY_STATUS_TAG_TYPE[raw] ?? 'info';
}

const activeTab = ref('orders');
const orders = ref<OutsourcingOrder[]>([]);
const loading = ref(false);
const saving = ref(false);
const dialogVisible = ref(false);
const editingId = ref<number | null>(null);

const form = reactive({
  order_no: '',
  order_type: 'dyeing',
  supplier_id: undefined as number | undefined,
  issue_date: '',
  expected_return_date: '',
  // 数值录入项统一为手写文本模型（空串=未填），提交前显式 Number() 归一为契约数值
  issue_quantity: '',
  issue_unit: '',
  material_cost: '',
  // 三费：outsourcing_order NOT NULL 列（后端 v15:3247-3249），建单必填（0 为合法值，
  // 判空用 ===''，不得用 !value 误判 0）；编辑回显按 Decimal 出参经 numericView 归一
  processing_fee: '',
  freight_fee: '',
  tax_amount: '',
});

// 建单/编辑对话框数值录入校验（范围与精度靠校验器，不靠步进按钮）
const orderFormRef = ref<FormInstance>();
const orderRules = computed<FormRules>(() => ({
  issue_quantity: numberInputRules({
    label: t('outsourcing.columns.issueQuantity'),
    required: true,
    min: 0.01,
    precision: 2,
  }),
  material_cost: numberInputRules({
    label: t('outsourcing.form.materialCost'),
    required: true,
    min: 0,
    precision: 2,
  }),
  processing_fee: numberInputRules({
    label: t('outsourcing.columns.processingFee'),
    required: true,
    min: 0,
    precision: 2,
  }),
  freight_fee: numberInputRules({
    label: t('outsourcing.columns.freightFee'),
    required: true,
    min: 0,
    precision: 2,
  }),
  tax_amount: numberInputRules({
    label: t('outsourcing.columns.taxAmount'),
    required: true,
    min: 0,
    precision: 2,
  }),
}));

/** 手写文本模型转数值：空串回 fallback（仅用于允许缺省的数值提交点） */
const numOr = (text: string, fallback: number): number =>
  text.trim() === '' ? fallback : Number(text);

/** 表单级校验（数字格式/范围/精度由 numberInputRules 单源判定），失败即拦截提交 */
const validateFormRef = async (formRef?: FormInstance): Promise<boolean> => {
  if (!formRef) return true;
  let ok = false;
  await formRef.validate(valid => {
    ok = valid;
  });
  return ok;
};

const validateOrderForm = () => validateFormRef(orderFormRef.value);

/** 打开新建委外单：自动预生成单据号（查重唯一后只读展示，防手动输入重复） */
const openCreate = async () => {
  editingId.value = null;
  // 三费回到未填态（NOT NULL 列 0 为合法值，须显式填写而非沿用上次编辑回显值）
  form.processing_fee = '';
  form.freight_fee = '';
  form.tax_amount = '';
  form.order_no = await generateUniqueDocNo('OUT', 'outsourcing_order');
  dialogVisible.value = true;
};

// 供应商/加工商候选：新建与编辑共用同一表单，选中项按 supplier_id 落库；
// 输入名称或编码即时筛选，避免只能手填数字主键。
const supplierOptions = ref<Supplier[]>([]);
const supplierQuery = ref('');

const visibleSupplierOptions = computed(() => {
  const q = supplierQuery.value.trim().toLowerCase();
  if (!q) return supplierOptions.value;
  return supplierOptions.value.filter(
    s =>
      s.supplier_name.toLowerCase().includes(q) ||
      s.supplier_code.toLowerCase().includes(q) ||
      String(s.id).startsWith(q)
  );
});

const supplierLabel = (s: Supplier): string =>
  s.supplier_code ? `${s.supplier_name}（${s.supplier_code}）` : s.supplier_name;

const filterSupplierOptions = (query: string) => {
  supplierQuery.value = query;
};

const loadSupplierOptions = async () => {
  try {
    const res = await getSupplierList({ page: 1, page_size: 500 });
    supplierOptions.value = res.data.items;
  } catch (e: unknown) {
    logAuxLoadFailure('委外供应商候选加载失败', e);
  }
};

async function load() {
  loading.value = true;
  try {
    // GET /production/outsourcing-orders 出参 = ApiResponse<PaginatedResponse>
    // （handlers/outsourcing_handler.rs:113-137）：分页唯一形状 {items,total,page,page_size}
    // （utils/response.rs:38-43），读取键固定 data.items，不做形状探测、不做 ?? 兜底。
    const res = await getOutsourcingOrderList();
    orders.value = res.data.items;
  } catch (e: unknown) {
    logAuxLoadFailure('委外订单列表加载失败', e);
  } finally {
    loading.value = false;
  }
}

async function onCreate() {
  // material_cost/三费后端允许为 0（真实成本可为 0），仅校验是否填写，不能用 !value 误判 0
  if (
    !form.order_no ||
    !form.supplier_id ||
    !form.issue_date ||
    form.issue_quantity.trim() === '' ||
    form.material_cost.trim() === '' ||
    form.processing_fee.trim() === '' ||
    form.freight_fee.trim() === '' ||
    form.tax_amount.trim() === ''
  ) {
    ElMessage.warning(t('outsourcing.message.requiredOrderFields'));
    return;
  }
  const valid = await validateOrderForm();
  if (!valid) return;
  saving.value = true;
  try {
    await createOutsourcingOrder({
      order_no: form.order_no,
      order_type: form.order_type,
      supplier_id: form.supplier_id,
      issue_date: form.issue_date,
      expected_return_date: form.expected_return_date || undefined,
      issue_quantity: Number(form.issue_quantity),
      issue_unit: form.issue_unit || undefined,
      material_cost: Number(form.material_cost),
      processing_fee: Number(form.processing_fee),
      freight_fee: Number(form.freight_fee),
      tax_amount: Number(form.tax_amount),
    });
    ElMessage.success(t('outsourcing.message.orderCreated'));
    dialogVisible.value = false;
    form.order_no = '';
    form.issue_quantity = '';
    form.material_cost = '';
    form.processing_fee = '';
    form.freight_fee = '';
    form.tax_amount = '';
    await load();
  } finally {
    saving.value = false;
  }
}

// 编辑委外单（draft 态）
const openEdit = (row: OutsourcingOrder) => {
  editingId.value = row.id;
  Object.assign(form, {
    order_no: row.order_no,
    order_type: row.order_type || 'dyeing',
    supplier_id: row.supplier_id,
    issue_date: row.issue_date || '',
    expected_return_date: row.expected_return_date || '',
    // 回显：后端 Decimal 列出参为字符串（如 "100.0000"），经 Number() 归一后按
    // 两位小数格式化灌入手写录入框（NOT NULL 列键恒在，无需 ?? 兜底缺键）
    issue_quantity: numericView(Number(row.issue_quantity), 2),
    issue_unit: row.issue_unit || '',
    material_cost: numericView(Number(row.material_cost), 2),
    processing_fee: numericView(Number(row.processing_fee), 2),
    freight_fee: numericView(Number(row.freight_fee), 2),
    tax_amount: numericView(Number(row.tax_amount), 2),
  });
  dialogVisible.value = true;
};

const onSave = async () => {
  if (editingId.value) {
    if (!form.supplier_id || !form.issue_date || form.issue_quantity.trim() === '') {
      ElMessage.warning(t('outsourcing.message.requiredEditFields'));
      return;
    }
    const valid = await validateOrderForm();
    if (!valid) return;
    saving.value = true;
    try {
      await updateOutsourcingOrder(editingId.value, {
        order_type: form.order_type,
        supplier_id: form.supplier_id,
        issue_date: form.issue_date,
        // expected_return_date 为 DB 可空列（三态）：对话框已回显原值，
        // UI 清空 ⇒ 送显式 null（=清空），未改动 ⇒ 原值回传；其余未采集键省略=保持原值
        expected_return_date: form.expected_return_date || null,
        issue_quantity: Number(form.issue_quantity),
        issue_unit: form.issue_unit || undefined,
        material_cost: numOr(form.material_cost, 0),
        // 三费 NOT NULL 列：编辑框已回显库中原值，UI 清空归一为 0 显式覆盖
        //（0 为合法值；显式 null 会被后端拒"不能清空"，键缺席=保持原值，
        // 两种写法都不如把用户看到的空如实写成 0——不留静默歧义）
        processing_fee: numOr(form.processing_fee, 0),
        freight_fee: numOr(form.freight_fee, 0),
        tax_amount: numOr(form.tax_amount, 0),
      });
      ElMessage.success(t('outsourcing.message.orderUpdated'));
      dialogVisible.value = false;
      editingId.value = null;
      await load();
    } finally {
      saving.value = false;
    }
  } else {
    editingId.value = null;
    await onCreate();
  }
};

// 删除委外单（draft 态）
const onDelete = async (row: OutsourcingOrder) => {
  try {
    await ElMessageBox.confirm(
      t('outsourcing.message.confirmDeleteOrder', { orderNo: row.order_no }),
      t('message.confirmTitle'),
      { type: 'warning' }
    );
  } catch (e) {
    // 取消/关闭确认框是正常中止路径，不是错误；其余 reject 按真实失败外显并留痕
    if (!isDialogDismissal(e)) {
      logger.error('[outsourcing] 删除确认对话框异常', e);
      ElMessage.error((e as Error).message || t('message.deleteFailed'));
    }
    return;
  }
  try {
    await deleteOutsourcingOrder(row.id);
    msg.deleteOk();
    await load();
  } catch (e) {
    ElMessage.error((e as Error).message || t('message.deleteFailed'));
  }
};

// 详情 + 发料明细
const detailVisible = ref(false);
const detailOrder = ref<OutsourcingOrder | null>(null);
const orderItems = ref<OutsourcingOrderItem[]>([]);
const itemLoading = ref(false);
const itemDialogVisible = ref(false);
const itemSaving = ref(false);
const itemFormRef = ref<FormInstance>();
const itemForm = reactive({
  product_id: '',
  color_no: '',
  dye_lot_no: '',
  // 生产匹号：委外发料逐条必须引用真实可用生产匹（后端发料门精确到匹），
  // 取值仅允许来自 loadProductionPieces 经 GET /inventory/pieces 下推查回的生产匹
  piece_no: '',
  quantity: '',
  unit: '',
  unit_cost: '0',
  processing_fee: '0',
  freight_fee: '0',
  remarks: '',
});

// 明细登记数值录入校验（范围与精度靠校验器，不靠步进按钮）
const itemRules = computed<FormRules>(() => ({
  product_id: numberInputRules({
    label: t('outsourcing.columns.productId'),
    required: true,
    min: 1,
    integer: true,
  }),
  quantity: numberInputRules({
    label: t('outsourcing.columns.quantity'),
    required: true,
    min: 0.01,
    precision: 2,
  }),
  unit_cost: numberInputRules({
    label: t('outsourcing.columns.unitCost'),
    required: true,
    min: 0,
    precision: 2,
  }),
  processing_fee: numberInputRules({
    label: t('outsourcing.columns.processingFee'),
    min: 0,
    precision: 2,
  }),
  freight_fee: numberInputRules({
    label: t('outsourcing.columns.freightFee'),
    min: 0,
    precision: 2,
  }),
}));

// 生产匹候选按产品 ID 缓存（产品→该产品的可用生产匹列表），与调拨/发货出库第四维
// 同源取数（GET /inventory/pieces，piece_type=greige + status=AVAILABLE）
const productionPiecesMap = ref<Record<number, InventoryPieceRow[]>>({});

// 当前明细行的匹号候选：已填产品则读其缓存（未加载时为空数组，由下拉展开时按需取回），
// 未填产品无候选可选
const pieceOptions = computed(() => {
  const productId = numOr(itemForm.product_id, NaN);
  return Number.isInteger(productId) && productId > 0
    ? (productionPiecesMap.value[productId] ?? [])
    : [];
});

/** 下拉展开时按当前产品拉取可用生产匹（生产匹 + 可用），命中缓存则不重复请求 */
const loadProductionPieces = async () => {
  const productId = numOr(itemForm.product_id, NaN);
  if (!Number.isInteger(productId) || productId <= 0 || productionPiecesMap.value[productId])
    return;
  try {
    // 出参 PaginatedResponse{items,total,page,page_size} 单一形状，固定读 res.data.items，
    // 不做 items/data 双形状探测、不做 ?? [] 兜底掩盖缺键
    const res = await getAvailablePieces({
      product_id: productId,
      piece_type: PIECE_TYPE.GREIGE,
      status: INVENTORY_PIECE_STATUS.AVAILABLE,
      page: 1,
      page_size: 100,
    });
    productionPiecesMap.value = { ...productionPiecesMap.value, [productId]: res.data.items };
  } catch (e) {
    logger.error('[outsourcing] 加载可用生产匹失败', e);
    ElMessage.error(t('outsourcing.message.pieceNoLoadFailed'));
  }
};

// 产品变更使已选匹号失效（生产匹按产品归属），清空已选，等待按新产品重新取回
watch(
  () => itemForm.product_id,
  () => {
    itemForm.piece_no = '';
  }
);

// 每次打开明细登记对话框都清空上一次的匹号残留并按最新库存重新取数，
// 避免把已被他单占用/发出的旧匹号带入本次登记
watch(itemDialogVisible, visible => {
  if (visible) {
    itemForm.piece_no = '';
    productionPiecesMap.value = {};
  }
});

const openDetail = async (row: OutsourcingOrder) => {
  detailOrder.value = row;
  detailVisible.value = true;
  itemLoading.value = true;
  try {
    // 后端 outsourcing_handler.rs::list_outsourcing_items 出参 ApiResponse<Vec<Model>>：
    // 成功信封载荷在 res.data（明细数组本身），非 res.items。按端点定型读取单一键，
    // 不做 items/data 双形状探测、不做 ?? [] 兜底（缺键属契约失配，须暴露不得掩盖）。
    const res = await getOutsourcingItems(row.id);
    orderItems.value = res.data;
  } finally {
    itemLoading.value = false;
  }
};

const onSaveItem = async () => {
  if (!detailOrder.value || !itemForm.product_id || itemForm.quantity.trim() === '') {
    ElMessage.warning(t('outsourcing.message.requiredItemFields'));
    return;
  }
  const valid = await validateFormRef(itemFormRef.value);
  if (!valid) return;
  // 委外发料精确到匹：无匹号的明细无法通过后端发料门，登记时必须已选定真实可用生产匹
  if (!itemForm.piece_no) {
    ElMessage.warning(t('outsourcing.message.pieceNoRequired'));
    return;
  }
  itemSaving.value = true;
  try {
    await createOutsourcingItem(detailOrder.value.id, {
      outsourcing_order_id: detailOrder.value.id,
      product_id: Number(itemForm.product_id),
      color_no: itemForm.color_no || null,
      dye_lot_no: itemForm.dye_lot_no || null,
      piece_no: itemForm.piece_no,
      quantity: Number(itemForm.quantity),
      unit: itemForm.unit || null,
      unit_cost: numOr(itemForm.unit_cost, 0),
      // 附加费为可空列：空或 0 如实提交 null（原控件默认 0 即"未发生"）
      processing_fee: numOr(itemForm.processing_fee, 0) || null,
      freight_fee: numOr(itemForm.freight_fee, 0) || null,
      remarks: itemForm.remarks || null,
    });
    ElMessage.success(t('outsourcing.message.itemAdded'));
    itemDialogVisible.value = false;
    itemForm.piece_no = '';
    await openDetail(detailOrder.value);
  } finally {
    itemSaving.value = false;
  }
};

// 收回单
const receipts = ref<OutsourcingReceipt[]>([]);
const receiptLoading = ref(false);
const receiptDialogVisible = ref(false);
const receiptSaving = ref(false);
const receiptFormRef = ref<FormInstance>();
const receiptForm = reactive({
  receipt_no: '',
  outsourcing_order_id: '',
  receipt_date: '',
  product_id: '',
  color_no: '',
  dye_lot_no: '',
  return_quantity: '',
  loss_quantity: '0',
  quality_status: OUTSOURCING_QUALITY_STATUS.qualified,
  grade: '',
  remarks: '',
  // 打卷实测值（DB 可空列，空串=未填即未补录；0 非合法实测值，判空一律 ===''，>0 由校验器拦）
  weight: '',
  width: '',
  gram_weight: '',
});

// 收回单数值录入校验（范围与精度靠校验器，不靠步进按钮；实测三列留空合法）
const receiptRules = computed<FormRules>(() => ({
  outsourcing_order_id: numberInputRules({
    label: t('outsourcing.columns.outsourcingOrderId'),
    required: true,
    min: 1,
    integer: true,
  }),
  product_id: numberInputRules({
    label: t('outsourcing.columns.productId'),
    required: true,
    min: 1,
    integer: true,
  }),
  return_quantity: numberInputRules({
    label: t('outsourcing.columns.returnQuantity'),
    required: true,
    min: 0.01,
    precision: 2,
  }),
  loss_quantity: numberInputRules({
    label: t('outsourcing.columns.lossQuantity'),
    min: 0,
    precision: 2,
  }),
  weight: numberInputRules({
    label: t('outsourcing.receipt.measured.weight'),
    min: 0,
    precision: 4,
  }),
  width: numberInputRules({
    label: t('outsourcing.receipt.measured.width'),
    min: 0,
    precision: 4,
  }),
  gram_weight: numberInputRules({
    label: t('outsourcing.receipt.measured.gramWeight'),
    min: 0,
    precision: 4,
  }),
}));

async function loadReceipts() {
  receiptLoading.value = true;
  try {
    // 后端 outsourcing_handler.rs::list_outsourcing_receipts 出参 ApiResponse<PaginatedResponse>：
    // 分页唯一形状 {items,total,page,page_size}（utils/response.rs:38-43），载荷在 res.data.items，
    // 非 res.items。按单一形状取键，不做 ?? 兜底。
    const res = await getOutsourcingReceiptList();
    receipts.value = res.data.items;
  } catch (e: unknown) {
    logAuxLoadFailure('委外收货列表加载失败', e);
  } finally {
    receiptLoading.value = false;
  }
}

const openCreateReceipt = async () => {
  // 实测值回到未填态（可空列，空串=未补录），不沿用上一单的编辑残留
  receiptForm.weight = '';
  receiptForm.width = '';
  receiptForm.gram_weight = '';
  receiptForm.receipt_no = await generateUniqueDocNo('ORC', 'outsourcing_receipt');
  receiptDialogVisible.value = true;
};

/**
 * 实测值 >0 本地校验（与后端 validate_measured_values receipt.rs:76-86 同口径）：
 * 留空放行（DB 可空=未补录）；0/负数按列点名拒绝并给出留空指引，不放行到后端吃 400。
 */
function checkMeasuredPositive(form: {
  weight: string;
  width: string;
  gram_weight: string;
}): boolean {
  const isNonPositive = (text: string) => {
    if (text.trim() === '') return false;
    const num = Number(text);
    return !Number.isFinite(num) || num <= 0;
  };
  if (isNonPositive(form.weight)) {
    ElMessage.warning(t('outsourcing.receipt.measured.weightPositive'));
    return false;
  }
  if (isNonPositive(form.width)) {
    ElMessage.warning(t('outsourcing.receipt.measured.widthPositive'));
    return false;
  }
  if (isNonPositive(form.gram_weight)) {
    ElMessage.warning(t('outsourcing.receipt.measured.gramWeightPositive'));
    return false;
  }
  return true;
}

/** 实测值列表格单元：null=未补录显示占位文案（不显示空/0）；有值如实显示后端出参原文 */
function measuredCellText(value: string | null): string {
  return value == null ? t('outsourcing.receipt.measured.notRecorded') : value;
}

const onSaveReceipt = async () => {
  if (
    !receiptForm.receipt_no ||
    !receiptForm.outsourcing_order_id ||
    !receiptForm.receipt_date ||
    !receiptForm.product_id ||
    receiptForm.return_quantity.trim() === ''
  ) {
    ElMessage.warning(t('outsourcing.message.requiredReceiptFields'));
    return;
  }
  const valid = await validateFormRef(receiptFormRef.value);
  if (!valid) return;
  if (!checkMeasuredPositive(receiptForm)) {
    return;
  }
  receiptSaving.value = true;
  try {
    await createOutsourcingReceipt({
      receipt_no: receiptForm.receipt_no,
      outsourcing_order_id: Number(receiptForm.outsourcing_order_id),
      receipt_date: receiptForm.receipt_date,
      product_id: Number(receiptForm.product_id),
      color_no: receiptForm.color_no || null,
      dye_lot_no: receiptForm.dye_lot_no || null,
      return_quantity: Number(receiptForm.return_quantity),
      loss_quantity: numOr(receiptForm.loss_quantity, 0) || null,
      quality_status: receiptForm.quality_status || null,
      grade: receiptForm.grade || null,
      remarks: receiptForm.remarks || null,
      // 建单直写口径（CreateOutsourcingReceiptRequest 的 weight width gram_weight 为
      // Option<Decimal>）：留空=空串=如实落 NULL（未补录），
      // 建单无"保持原值"态故不涉及键缺席；>0 已由 checkMeasuredPositive 前置拦截
      weight: receiptForm.weight.trim() === '' ? null : Number(receiptForm.weight),
      width: receiptForm.width.trim() === '' ? null : Number(receiptForm.width),
      gram_weight: receiptForm.gram_weight.trim() === '' ? null : Number(receiptForm.gram_weight),
    });
    ElMessage.success(t('outsourcing.message.receiptCreated'));
    receiptDialogVisible.value = false;
    await loadReceipts();
  } finally {
    receiptSaving.value = false;
  }
};

// ========== 补录打卷实测值（PUT /production/outsourcing-receipts/{id}，仅 draft） ==========
const measuredDialogVisible = ref(false);
const measuredSaving = ref(false);
const measuredFormRef = ref<FormInstance>();
const measuredReceiptId = ref<number | null>(null);
/** 打开对话框时的库中原值（后端出参 string|null），三态 diff 的基准，保存前不被改写 */
const measuredOriginal = reactive<{
  weight: string | null;
  width: string | null;
  gram_weight: string | null;
}>({ weight: null, width: null, gram_weight: null });
const measuredForm = reactive({
  weight: '',
  width: '',
  gram_weight: '',
});

// 补录实测值校验：DB 可空列留空合法（三态由 measuredTriState 判定），
// 填了就必须是 ≤4 位小数的非负数；>0 由 checkMeasuredPositive 点名拦截
const measuredRules = computed<FormRules>(() => ({
  weight: numberInputRules({
    label: t('outsourcing.receipt.measured.weight'),
    min: 0,
    precision: 4,
  }),
  width: numberInputRules({
    label: t('outsourcing.receipt.measured.width'),
    min: 0,
    precision: 4,
  }),
  gram_weight: numberInputRules({
    label: t('outsourcing.receipt.measured.gramWeight'),
    min: 0,
    precision: 4,
  }),
}));

const openMeasuredDialog = (row: OutsourcingReceipt) => {
  measuredReceiptId.value = row.id;
  measuredOriginal.weight = row.weight;
  measuredOriginal.width = row.width;
  measuredOriginal.gram_weight = row.gram_weight;
  // 回显：Decimal 出参为字符串（如 "12.5000"），经 Number() 归一后按 4 位小数格式化
  // 灌入手写录入框；null=未补录=留空
  measuredForm.weight = row.weight == null ? '' : numericView(Number(row.weight), 4);
  measuredForm.width = row.width == null ? '' : numericView(Number(row.width), 4);
  measuredForm.gram_weight = row.gram_weight == null ? '' : numericView(Number(row.gram_weight), 4);
  measuredDialogVisible.value = true;
};

/**
 * 单字段三态映射（UpdateOutsourcingReceiptRequest double_option 三态语义
 * types.rs:282-288 / receipt.rs:373-383）：
 * 留空且原值空 ⇒ undefined（键缺席=保持原值，不下发该键）；
 * 留空且原值有 ⇒ null（显式清空回"未补录"）；
 * 有值 ⇒ 覆盖写入。
 */
function measuredTriState(cur: string, orig: string | null): number | null | undefined {
  if (cur.trim() === '') {
    return orig == null ? undefined : null;
  }
  return Number(cur);
}

const onSaveMeasured = async () => {
  if (measuredReceiptId.value == null) {
    return;
  }
  const valid = await validateFormRef(measuredFormRef.value);
  if (!valid) return;
  if (!checkMeasuredPositive(measuredForm)) {
    return;
  }
  const weight = measuredTriState(measuredForm.weight, measuredOriginal.weight);
  const width = measuredTriState(measuredForm.width, measuredOriginal.width);
  const gramWeight = measuredTriState(measuredForm.gram_weight, measuredOriginal.gram_weight);
  const payload: UpdateOutsourcingReceiptPayload = {};
  if (weight !== undefined) {
    payload.weight = weight;
  }
  if (width !== undefined) {
    payload.width = width;
  }
  if (gramWeight !== undefined) {
    payload.gram_weight = gramWeight;
  }
  if (!('weight' in payload) && !('width' in payload) && !('gram_weight' in payload)) {
    // 三字段均无改动：不发请求、不伪造"已保存"提示，直接关闭
    measuredDialogVisible.value = false;
    return;
  }
  measuredSaving.value = true;
  try {
    await updateOutsourcingReceipt(measuredReceiptId.value, payload);
    // 本次提交仅含清空（无任何写入值）⇒ 清空提示；含写入/覆盖 ⇒ 补录成功提示
    const clearedOnly =
      [weight, width, gramWeight].some(v => v === null) &&
      [weight, width, gramWeight].every(v => v === null || v === undefined);
    ElMessage.success(
      t(clearedOnly ? 'outsourcing.receipt.measured.cleared' : 'outsourcing.receipt.measured.saved')
    );
    measuredDialogVisible.value = false;
    await loadReceipts();
  } finally {
    measuredSaving.value = false;
  }
};

/**
 * 结算前端门（与后端 settle 费用门 order.rs:607-611 同口径 `processing_fee + freight_fee <= 0`，
 * 提前提示不可结算，不替代后端校验——后端拒绝仍走失败信封正常外显）。
 * 两列为 NOT NULL DECIMAL（v15/mod.rs:3247-3248），出参恒为字符串键，Number() 归一求和，无缺键兜底。
 */
const isZeroFeeOrder = (row: OutsourcingOrder) =>
  Number(row.processing_fee) + Number(row.freight_fee) <= 0;

/**
 * 收回确认前端门（与后端 confirm 数量门 receipt.rs:338-342 同口径 `return_quantity <= 0`；
 * 存量 0 量草稿由本门先拦，点击路径不可达，后端仍会硬拒兜底）。
 */
const isZeroQtyReceipt = (row: OutsourcingReceipt) => Number(row.return_quantity) <= 0;

const onConfirmReceipt = async (row: OutsourcingReceipt) => {
  try {
    await ElMessageBox.confirm(
      t('outsourcing.message.confirmReceiptAction', { id: row.id }),
      t('message.confirmTitle')
    );
  } catch (e) {
    // 取消/关闭确认框是正常中止路径，不是错误；其余 reject 按真实失败外显并留痕
    if (!isDialogDismissal(e)) {
      logger.error('[outsourcing] 收回确认对话框异常', e);
      ElMessage.error((e as Error).message || t('outsourcing.message.confirmReceiptFailed'));
    }
    return;
  }
  try {
    await confirmOutsourcingReceipt(row.id);
    ElMessage.success(t('outsourcing.message.receiptConfirmed'));
    await loadReceipts();
    await load();
  } catch (e) {
    ElMessage.error((e as Error).message || t('outsourcing.message.confirmReceiptFailed'));
  }
};

/**
 * 状态流转通用动作：confirmKey/successKey 均为字面 i18n 完整键（调用点传入，
 * check-i18n 对动态 t(var) 不判定，键存在性由调用点字面键与语言包双侧保证）。
 */
const act = async (
  row: OutsourcingOrder,
  fn: (id: number, d?: Record<string, unknown>) => Promise<unknown>,
  confirmKey: string,
  successKey: string,
  prompt = false
) => {
  if (prompt) {
    try {
      await ElMessageBox.confirm(t(confirmKey), t('message.confirmTitle'));
    } catch (e) {
      // 弹窗取消不是错误：isDialogDismissal 命中即静默中止；其余 reject 外显并留痕
      if (!isDialogDismissal(e)) {
        logger.error('[outsourcing] 操作确认对话框异常', e);
        ElMessage.error((e as Error).message || t('message.operationFailed'));
      }
      return;
    }
  }
  await fn(row.id);
  ElMessage.success(t(successKey));
  await load();
};

const onIssue = (row: OutsourcingOrder) =>
  act(
    row,
    issueOutsourcingOrder,
    'outsourcing.message.confirmIssue',
    'outsourcing.message.issueSuccess',
    true
  );
const onProcess = (row: OutsourcingOrder) =>
  act(
    row,
    processOutsourcingOrder,
    'outsourcing.message.confirmProcess',
    'outsourcing.message.processSuccess',
    true
  );
const onSettle = (row: OutsourcingOrder) =>
  act(
    row,
    settleOutsourcingOrder,
    'outsourcing.message.confirmSettle',
    'outsourcing.message.settleSuccess',
    true
  );
const onClose = (row: OutsourcingOrder) =>
  act(
    row,
    closeOutsourcingOrder,
    'outsourcing.message.confirmClose',
    'outsourcing.message.closeSuccess',
    true
  );
const onCancel = (row: OutsourcingOrder) =>
  act(
    row,
    cancelOutsourcingOrder,
    'outsourcing.message.confirmCancel',
    'outsourcing.message.cancelSuccess',
    true
  );

onMounted(() => {
  load();
  loadReceipts();
  loadSupplierOptions();
});
</script>

<style scoped>
.w-full {
  width: 100%;
}
.mb {
  margin-bottom: 12px;
}
.toolbar {
  display: flex;
  gap: 8px;
  align-items: center;
}
.items-section {
  margin-top: 16px;
}
.items-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}
.items-title {
  font-weight: 600;
  font-size: 14px;
}
.measured-hint {
  margin: -8px 0 12px 110px;
  font-size: 12px;
  line-height: 1.5;
  color: var(--el-text-color-secondary);
}
</style>

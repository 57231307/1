// 扫描 Batch 6 计划文件中实际未接入 useI18n 的文件
const fs = require('fs');
const path = require('path');

const PLAN = [
  // api-gateway
  'frontend/src/views/api-gateway/components/ApiEndpointForm.vue',
  'frontend/src/views/api-gateway/components/KeyForm.vue',
  'frontend/src/views/api-gateway/components/LogDetail.vue',
  'frontend/src/views/api-gateway/tabs/ApiEndpointTab.vue',
  'frontend/src/views/api-gateway/tabs/ApiKeyTab.vue',
  'frontend/src/views/api-gateway/tabs/ApiLogTab.vue',
  'frontend/src/views/api-gateway/index.vue',
  // bpm
  'frontend/src/views/bpm/approval/components/BpmApprovalApprovalDialog.vue',
  'frontend/src/views/bpm/approval/components/BpmApprovalStat.vue',
  'frontend/src/views/bpm/approval/components/BpmApprovalTransferDialog.vue',
  'frontend/src/views/bpm/approval/index.vue',
  'frontend/src/views/bpm/definitions/components/BpmDefinitionFilter.vue',
  'frontend/src/views/bpm/definitions/components/BpmDefinitionForm.vue',
  'frontend/src/views/bpm/definitions/components/BpmDefinitionTemplateDialog.vue',
  // fabric
  'frontend/src/views/fabric/tabs/DyeFormDialogTab.vue',
  'frontend/src/views/fabric/tabs/DyeTab.vue',
  'frontend/src/views/fabric/tabs/GreigeFormDialogTab.vue',
  'frontend/src/views/fabric/tabs/GreigeTab.vue',
  'frontend/src/views/fabric/tabs/RecipeFormDialogTab.vue',
  'frontend/src/views/fabric/tabs/RecipeTab.vue',
  'frontend/src/views/fabric/index.vue',
  // finance
  'frontend/src/views/finance/tabs/components/VoucherDetail.vue',
  'frontend/src/views/finance/tabs/components/VoucherFilter.vue',
  'frontend/src/views/finance/tabs/components/VoucherForm.vue',
  'frontend/src/views/finance/tabs/components/VoucherTable.vue',
  'frontend/src/views/finance/tabs/SubjectTab.vue',
  'frontend/src/views/finance/tabs/VoucherTab.vue',
  'frontend/src/views/finance/index.vue',
  // inventory
  'frontend/src/views/inventory/components/AdjustmentDialog.vue',
  'frontend/src/views/inventory/components/StatCards.vue',
  'frontend/src/views/inventory/components/TransferDialog.vue',
  'frontend/src/views/inventory/tabs/InventoryAlertTab.vue',
  'frontend/src/views/inventory/tabs/InventoryStockTab.vue',
  'frontend/src/views/inventory/tabs/InventoryTransferTab.vue',
  'frontend/src/views/inventory/index.vue',
  // logistics (已在 Batch 5 接入)
  'frontend/src/views/logistics/components/LogisticsDetail.vue',
  'frontend/src/views/logistics/components/LogisticsFilter.vue',
  'frontend/src/views/logistics/components/LogisticsForm.vue',
  'frontend/src/views/logistics/components/LogisticsStat.vue',
  'frontend/src/views/logistics/components/LogisticsStatDialog.vue',
  'frontend/src/views/logistics/components/LogisticsTable.vue',
  'frontend/src/views/logistics/index.vue',
  // system-update
  'frontend/src/views/system-update/components/SystemUpdateBackupForm.vue',
  'frontend/src/views/system-update/components/SystemUpdateInfoCards.vue',
  'frontend/src/views/system-update/components/SystemUpdateVersionDetail.vue',
  'frontend/src/views/system-update/tabs/SystemUpdateBackupTab.vue',
  'frontend/src/views/system-update/tabs/SystemUpdateTaskTab.vue',
  'frontend/src/views/system-update/tabs/SystemUpdateVersionTab.vue',
  'frontend/src/views/system-update/index.vue',
];

let missing = 0;
let hasI18n = 0;
let notExist = 0;
const missingFiles = [];
const hasI18nFiles = [];

for (const rel of PLAN) {
  const abs = path.resolve('/workspace', rel);
  if (!fs.existsSync(abs)) {
    console.log(`[NOT_EXIST] ${rel}`);
    notExist++;
    continue;
  }
  const content = fs.readFileSync(abs, 'utf8');
  const hasUseI18n = /import\s+\{\s*useI18n\s*\}\s+from\s+['"]vue-i18n['"]/.test(content) ||
                     /useI18n\(\s*\{\s*useScope:\s*['"]global['"]\s*\}\s*\)/.test(content);
  const hasChinese = /[\u4e00-\u9fa5]/.test(content);
  
  if (hasUseI18n) {
    hasI18n++;
    hasI18nFiles.push(rel);
  } else if (hasChinese) {
    missing++;
    missingFiles.push(rel);
  } else {
    hasI18nFiles.push(`${rel} (无中文，无需接入)`);
  }
}

console.log(`\n=== 统计 ===`);
console.log(`总文件数: ${PLAN.length}`);
console.log(`已接入或无需接入: ${hasI18n}`);
console.log(`未接入（有硬编码中文）: ${missing}`);
console.log(`不存在的文件: ${notExist}`);

console.log(`\n=== 未接入文件清单 ===`);
for (const f of missingFiles) {
  console.log(`  ${f}`);
}

console.log(`\n=== 已接入或无需接入文件清单 ===`);
for (const f of hasI18nFiles) {
  console.log(`  ${f}`);
}

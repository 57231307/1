-- fixed_asset_disposals 处置损益列回滚（批次 88 PH-3）
ALTER TABLE "fixed_asset_disposals" DROP COLUMN IF EXISTS "gain_loss";

-- batch-13 P3: 色卡能力字段（染色能力、印花能力、色牢度等级）
ALTER TABLE color_cards ADD COLUMN IF NOT EXISTS dyeing_capability VARCHAR(50);
ALTER TABLE color_cards ADD COLUMN IF NOT EXISTS printing_capability VARCHAR(50);
ALTER TABLE color_cards ADD COLUMN IF NOT EXISTS color_fastness_grade VARCHAR(20);
COMMENT ON COLUMN color_cards.dyeing_capability IS '染色能力：reactive/acid/disperse/direct/vat';
COMMENT ON COLUMN color_cards.printing_capability IS '印花能力：screen/digital/transfer/block';
COMMENT ON COLUMN color_cards.color_fastness_grade IS '色牢度等级：A/B/C/D';

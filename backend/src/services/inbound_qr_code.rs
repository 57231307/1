//! 面料二维码定长解析（扫码入库的形切分腿）
//!
//! 二维码外形由用户定义，示例 0065230029265JL26092602022，四段依次是
//! 006523 我方款号，0029265 染厂色号，JL26092602 缸号，022 该缸内匹号。
//! 取位是固定的，头六字符是款号，第七到第十三字符是染厂色号，末三字符是匹号，
//! 缸号占剩下的全部字符，长度可变。
//!
//! 本模块只做字符串外形切分与逐段字符集校验，不查库也不解释业务含义，
//! 因此不存在兜底路径，任何不合形的输入都返回一个明确的拒绝原因。
//! 拒绝原因到现场文案的翻译在 services 下的 scan_inbound 模块，本模块不产出文案。
//!
//! 设计要点，四段的取位靠字符数而不是字节数，且空白与控制字符一律拒绝、不做裁剪。
//! 扫码枪把回车附在串尾是常见现象，按裁剪处理会让"多读了一个换行"与"标签破损"
//! 两种现场混成同一条通过路径，落库后是一条长度与匹号都可疑的匹行。

/// 款号段长度，我方款号补零到六位后写进二维码。
const PRODUCT_CODE_LEN: usize = 6;

/// 染厂色号段长度，染厂侧色号固定七位。
const EXTERNAL_COLOR_LEN: usize = 7;

/// 匹号段长度，缸号内匹号补零到三位。
const PIECE_NO_LEN: usize = 3;

/// 三段定长之和，串长短于此连三段都放不下，谈不上缸号。
const MIN_FRAME_LEN: usize = PRODUCT_CODE_LEN + EXTERNAL_COLOR_LEN + PIECE_NO_LEN;

/// 定长切分成功后交付的四段，全部是二维码原文的子串，不做数值化也不去前导零。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedRollQr {
    /// 前六字符，我方款号，补零形态原样保留，用于按 products.code 定位产品。
    pub product_code: String,
    /// 第七到第十三字符，染厂侧色号，是外部编码，必须经对照表换成内部色号才可落库。
    pub external_color_no: String,
    /// 中间可变长段，缸号，允许字母与数字混排。
    pub dye_lot_no: String,
    /// 末三字符，该缸内的匹号，前导零属于编码本身。
    pub piece_no: String,
}

/// 定长切分的拒绝原因，一变体一理由，调用方据此出文案，不允许折叠成一条。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QrRejection {
    /// 串里出现空白或控制字符，含首尾与内嵌，一律拒绝而不是裁剪。
    WhitespaceOrControl,
    /// 串长不足以同时容纳款号、色号与匹号三段定长。
    TooShort,
    /// 串长恰好只够三段定长，缸号段一个字符都没留下。
    EmptyDyeLot,
    /// 款号段含非 ASCII 数字字符，含字母与全角数字。
    ProductCodeNotNumeric,
    /// 染厂色号段含非 ASCII 数字字符。
    ExternalColorNotNumeric,
    /// 缸号段含 ASCII 字母与数字之外的字符。
    DyeLotNotAlphanumeric,
    /// 末三位不是全数字匹号，即匹号段长度或字符集不合形。
    PieceNoNotNumeric,
}

/// 按固定取位把面料二维码原文切成款号、染厂色号、缸号与匹号四段。
///
/// 做什么，定长切分加逐段字符集校验，不改写输入、不裁剪空白、不补默认值，
/// 各类失败各自返回对应的拒绝变体。
/// 调用方，services 下的 scan_inbound 模块的入库入口函数，
/// 由 handlers 下的 barcode_scanner_handler 的扫码入库端点触达。
/// 入参，raw，扫码枪交付的整串原文，允许任意 Unicode 内容。
/// 出参消费方，scan_inbound 用四段定位内部身份与查重，
/// 落库腿把 raw 整串原样写进库存匹行的条码列，出参本身不落库。
/// 存什么，什么都不存，本函数是纯函数，无副作用也不产生日志。
/// 存哪里，无落库。
pub fn parse_inbound_qr(raw: &str) -> Result<ParsedRollQr, QrRejection> {
    // 统一按字符取位，非 ASCII 字符占多字节，按字节切会在字符边界上 panic
    let chars: Vec<char> = raw.chars().collect();

    // 空白与控制字符先判，且判整串。
    // 为什么排在长度之前，扫码枪常把回车附在串尾，先判长度会把这一类
    // 现场误报成"长度不合形"，操作工按错方向排查，串首串尾与内嵌同等对待，不裁剪。
    if chars.iter().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(QrRejection::WhitespaceOrControl);
    }

    if chars.len() < MIN_FRAME_LEN {
        return Err(QrRejection::TooShort);
    }

    let dye_lot_start = PRODUCT_CODE_LEN + EXTERNAL_COLOR_LEN;
    let piece_start = chars.len() - PIECE_NO_LEN;

    // 三段定长正好铺满整串，缸号段为空。这是与 TooShort 不同的现场，
    // 标签上确实印了款号与色号与匹号，缺的是缸号，操作工要问的是缸号去哪了。
    if piece_start == dye_lot_start {
        return Err(QrRejection::EmptyDyeLot);
    }

    let product_code: String = chars[..PRODUCT_CODE_LEN].iter().copied().collect();
    if !product_code.chars().all(|c| c.is_ascii_digit()) {
        return Err(QrRejection::ProductCodeNotNumeric);
    }

    let external_color_no: String = chars[PRODUCT_CODE_LEN..dye_lot_start]
        .iter()
        .copied()
        .collect();
    if !external_color_no.chars().all(|c| c.is_ascii_digit()) {
        return Err(QrRejection::ExternalColorNotNumeric);
    }

    let dye_lot_no: String = chars[dye_lot_start..piece_start].iter().copied().collect();
    if !dye_lot_no.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(QrRejection::DyeLotNotAlphanumeric);
    }

    let piece_no: String = chars[piece_start..].iter().copied().collect();
    if !piece_no.chars().all(|c| c.is_ascii_digit()) {
        return Err(QrRejection::PieceNoNotNumeric);
    }

    Ok(ParsedRollQr {
        product_code,
        external_color_no,
        dye_lot_no,
        piece_no,
    })
}

// =====================================================
// 单元测试（纯函数，不依赖数据库）
// =====================================================
#[cfg(test)]
mod tests {
    use super::*;

    /// 用户给定的样例串，四段切分的基准真值，长度 26。
    const WORKED_EXAMPLE: &str = "0065230029265JL26092602022";

    /// 断言切分成功并交付四段，样例串以外的正例都走这里取真值。
    fn ok_parts(raw: &str) -> ParsedRollQr {
        parse_inbound_qr(raw).unwrap_or_else(|rej| panic!("预期切分成功，实际被拒为 {rej:?}"))
    }

    #[test]
    fn worked_example_splits_into_four_segments() {
        let parsed = ok_parts(WORKED_EXAMPLE);
        assert_eq!(parsed.product_code, "006523");
        assert_eq!(parsed.external_color_no, "0029265");
        assert_eq!(parsed.dye_lot_no, "JL26092602");
        assert_eq!(parsed.piece_no, "022");
    }

    #[test]
    fn leading_zero_in_roll_number_is_preserved_not_parsed_as_number() {
        // 匹号 002 的前导零属于编码本身，切分不得把它归一成 2
        let parsed = ok_parts("0065230029265JL26092602002");
        assert_eq!(parsed.piece_no, "002");
        assert_eq!(parsed.dye_lot_no, "JL26092602");
    }

    #[test]
    fn shortest_legal_lot_of_one_char_is_accepted() {
        // 串长 17，缸号段只留一个字符，这是缸号可变长的下界
        let parsed = ok_parts("0065230029265A022");
        assert_eq!(parsed.dye_lot_no, "A");
        assert_eq!(parsed.piece_no, "022");
    }

    #[test]
    fn too_short_string_is_rejected_as_too_short() {
        // 15 个字符，连款号与色号与匹号三段都放不下
        assert_eq!(
            parse_inbound_qr("0065230029265JL"),
            Err(QrRejection::TooShort)
        );
    }

    #[test]
    fn empty_lot_part_is_rejected_as_empty_dye_lot_not_as_too_short() {
        // 串长 16，三段定长正好铺满，缸号段为空，是与长度不足不同的现场
        assert_eq!(
            parse_inbound_qr("0065230029265022"),
            Err(QrRejection::EmptyDyeLot)
        );
    }

    #[test]
    fn alphabetic_char_in_product_code_is_rejected_by_its_own_variant() {
        assert_eq!(
            parse_inbound_qr("00A5230029265JL26092602022"),
            Err(QrRejection::ProductCodeNotNumeric)
        );
    }

    #[test]
    fn full_width_digit_in_product_code_is_rejected_by_its_own_variant() {
        // 全角数字是合法 Unicode 数字但不是 ASCII 数字，形切分只认 ASCII，
        // 否则同一条码值在库里会以两种形态存在
        assert_eq!(
            parse_inbound_qr("０065230029265JL26092602022"),
            Err(QrRejection::ProductCodeNotNumeric)
        );
    }

    #[test]
    fn alphabetic_char_in_color_segment_is_rejected_by_its_own_variant() {
        assert_eq!(
            parse_inbound_qr("00652300292A5JL26092602022"),
            Err(QrRejection::ExternalColorNotNumeric)
        );
    }

    #[test]
    fn hyphen_inside_lot_part_is_rejected_by_its_own_variant() {
        // 缸号里带连字符，多数是标签上把缸号与匹号用分隔符写开或读串了位
        assert_eq!(
            parse_inbound_qr("0065230029265JL-26092602022"),
            Err(QrRejection::DyeLotNotAlphanumeric)
        );
    }

    #[test]
    fn roll_number_that_is_not_three_ascii_digits_is_rejected_by_its_own_variant() {
        // 末三位含字母，即匹号段不合形，缸号段本身是合形的
        assert_eq!(
            parse_inbound_qr("0065230029265JL2609260202A"),
            Err(QrRejection::PieceNoNotNumeric)
        );
    }

    #[test]
    fn trailing_newline_is_rejected_not_trimmed() {
        // 扫码枪在串尾附回车，这是最常见的现场，必须点名空白与控制字符，
        // 而不是裁剪后当合法串入库
        let scanned_with_newline = format!("{WORKED_EXAMPLE}\n");
        assert_eq!(
            parse_inbound_qr(&scanned_with_newline),
            Err(QrRejection::WhitespaceOrControl)
        );
    }

    #[test]
    fn inner_space_is_rejected_not_trimmed() {
        // 去掉这一个空格就是一条完全合形的串，静默裁剪会让破损标签变成合法入库
        assert_eq!(
            parse_inbound_qr("006523 0029265JL26092602022"),
            Err(QrRejection::WhitespaceOrControl)
        );
    }

    #[test]
    fn empty_string_is_rejected_as_too_short() {
        assert_eq!(parse_inbound_qr(""), Err(QrRejection::TooShort));
    }

    #[test]
    fn parse_is_pure_and_leaves_input_untouched() {
        // 纯函数判据，同一串两次切分结果必须逐字段相同，且原文不被改写
        let raw = WORKED_EXAMPLE.to_string();
        let first = ok_parts(&raw);
        let second = ok_parts(&raw);
        assert_eq!(first, second);
        assert_eq!(raw, WORKED_EXAMPLE);
    }
}

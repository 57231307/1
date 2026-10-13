//! IPP 传输：以 IPP/1.1 Print-Job 操作经 HTTP 向打印机端点投递作业
//!
//! 请求体按 IPP over HTTP 约定编码为 `application/ipp`：版本 1.1、操作号 Print-Job(0x0002)、
//! operation 属性组（`attributes-charset=utf-8`、`attributes-natural-language`、`printer-uri`、
//! `requested-attributes=job-id`）、job 属性组（`copies`）、结束符后紧跟文档字节。响应体从
//! job 属性组解析 `job-id` 作为外部作业号。编码/解码均为纯函数（无网络），便于按字节夹具单测。

use async_trait::async_trait;
use tracing::warn;

use crate::services::print_dispatch::transport::{DispatchError, PrintJob, PrintTransport};

// —— IPP/1.1 线上格式常量（RFC 8010 / RFC 8011，取值与打印机实现一致）——
/// IPP 版本 major
const VERSION_MAJOR: u8 = 1;
/// IPP 版本 minor
const VERSION_MINOR: u8 = 1;
/// 操作号：Print-Job
const OPERATION_PRINT_JOB: u16 = 0x0002;
/// 属性分隔标签：operation-attributes-group
const TAG_OPERATION_ATTRS: u8 = 0x01;
/// 属性分隔标签：job-attributes-group
const TAG_JOB_ATTRS: u8 = 0x02;
/// 属性分隔标签：结束属性（其后为文档数据）
const TAG_END_OF_ATTRS: u8 = 0x03;
/// 值标签：charset
const VALUE_TAG_CHARSET: u8 = 0x47;
/// 值标签：natural-language
const VALUE_TAG_LANGUAGE: u8 = 0x48;
/// 值标签：uri
const VALUE_TAG_URI: u8 = 0x45;
/// 值标签：keyword
const VALUE_TAG_KEYWORD: u8 = 0x44;
/// 值标签：integer
const VALUE_TAG_INTEGER: u8 = 0x21;

/// 写一条 IPP 属性：值标签 + 名称(2 字节大端长度) + 名称字节 + 值(2 字节大端长度) + 值字节。
fn push_attr(buf: &mut Vec<u8>, value_tag: u8, name: &str, value: &[u8]) {
    buf.push(value_tag);
    buf.extend_from_slice(&(name.len() as u16).to_be_bytes());
    buf.extend_from_slice(name.as_bytes());
    buf.extend_from_slice(&(value.len() as u16).to_be_bytes());
    buf.extend_from_slice(value);
}

/// 写一条整数属性（4 字节大端值）。
fn push_int_attr(buf: &mut Vec<u8>, name: &str, value: i32) {
    push_attr(buf, VALUE_TAG_INTEGER, name, &value.to_be_bytes());
}

/// 构造一个 IPP/1.1 Print-Job 请求体（含文档字节）。
///
/// 入参：目标 `printer_uri`、`request_id`（单调请求标识）、`copies`（份数）、`doc`（文档字节）。
/// 出参：完整可 POST 的字节序列。属性顺序固定，输出对相同输入逐字节确定。
pub fn encode_print_job_request(
    printer_uri: &str,
    request_id: u32,
    copies: i32,
    doc: &[u8],
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(64 + doc.len());
    buf.push(VERSION_MAJOR);
    buf.push(VERSION_MINOR);
    buf.extend_from_slice(&OPERATION_PRINT_JOB.to_be_bytes());
    buf.extend_from_slice(&request_id.to_be_bytes());

    buf.push(TAG_OPERATION_ATTRS);
    push_attr(&mut buf, VALUE_TAG_CHARSET, "attributes-charset", b"utf-8");
    push_attr(
        &mut buf,
        VALUE_TAG_LANGUAGE,
        "attributes-natural-language",
        b"en",
    );
    push_attr(
        &mut buf,
        VALUE_TAG_URI,
        "printer-uri",
        printer_uri.as_bytes(),
    );
    push_attr(
        &mut buf,
        VALUE_TAG_KEYWORD,
        "requested-attributes",
        b"job-id",
    );

    buf.push(TAG_JOB_ATTRS);
    push_int_attr(&mut buf, "copies", copies);

    buf.push(TAG_END_OF_ATTRS);
    buf.extend_from_slice(doc);
    buf
}

/// 解析 IPP 响应状态码（字节偏移 2..4 的大端整数）。长度不足返回 `None`。
pub fn decode_status_code(resp: &[u8]) -> Option<u16> {
    if resp.len() < 4 {
        return None;
    }
    Some(u16::from_be_bytes([resp[2], resp[3]]))
}

/// 从 IPP 响应字节流中扫描 `job-id` 整数属性值。
///
/// 逐属性遍历（值标签 + 名称 + 值），命中名为 `job-id` 且为 4 字节整数者返回其值；
/// 遇 `end-of-attributes-tag` 或越界即停止。解析不到返回 `None`，由调用方如实处置。
pub fn decode_job_id(resp: &[u8]) -> Option<u32> {
    // 头部至少：版本(2)+状态(2)+请求号(4) = 8 字节
    if resp.len() < 8 {
        return None;
    }
    let mut i = 8usize;
    while i < resp.len() {
        let tag = resp[i];
        // 结束标签：其后为文档数据，属性区到此为止
        if tag == TAG_END_OF_ATTRS {
            return None;
        }
        // 属性组分隔标签：跳过继续向后找属性
        if matches!(tag, TAG_OPERATION_ATTRS | TAG_JOB_ATTRS) {
            i += 1;
            continue;
        }
        // 属性：value-tag(1) 已读 tag；name-len(2) + name + value-len(2) + value
        // 至少需要 2(名字长度)+2(值长度) 之后再判内容
        if i + 3 > resp.len() {
            return None;
        }
        let name_len = u16::from_be_bytes([resp[i + 1], resp[i + 2]]) as usize;
        let after_name = i + 3 + name_len;
        if after_name + 2 > resp.len() {
            return None;
        }
        let name = std::str::from_utf8(&resp[i + 3..after_name]).ok()?;
        let value_len = u16::from_be_bytes([resp[after_name], resp[after_name + 1]]) as usize;
        let value_start = after_name + 2;
        let value_end = value_start + value_len;
        if value_end > resp.len() {
            return None;
        }
        if name == "job-id" && tag == VALUE_TAG_INTEGER && value_len == 4 {
            return Some(u32::from_be_bytes([
                resp[value_start],
                resp[value_start + 1],
                resp[value_start + 2],
                resp[value_start + 3],
            ]));
        }
        i = value_end;
    }
    None
}

/// IPP 传输实现，持有目标端点 URL（来自 `print_targets.endpoint_url`）。
pub struct IppTransport {
    endpoint_url: String,
}

impl IppTransport {
    pub fn new(endpoint_url: String) -> Self {
        Self { endpoint_url }
    }
}

#[async_trait]
impl PrintTransport for IppTransport {
    async fn send(&self, job: &PrintJob, bytes: &[u8]) -> Result<String, DispatchError> {
        let request_id = fnv_hash(job.job_code.as_bytes());
        let request = encode_print_job_request(&self.endpoint_url, request_id, job.copies, bytes);

        let client = reqwest::Client::new();
        let response = client
            .post(&self.endpoint_url)
            .header(reqwest::header::CONTENT_TYPE, "application/ipp")
            .body(request)
            .send()
            .await
            .map_err(|e| DispatchError::Transport {
                detail: format!("IPP 请求发送失败（端点 {}）: {e}", self.endpoint_url),
            })?;

        let http_status = response.status();
        if !http_status.is_success() {
            return Err(DispatchError::Transport {
                detail: format!(
                    "IPP 端点 {} 返回非成功 HTTP 状态 {http_status}",
                    self.endpoint_url
                ),
            });
        }

        let resp_bytes = response
            .bytes()
            .await
            .map_err(|e| DispatchError::Transport {
                detail: format!("IPP 响应体读取失败: {e}"),
            })?;

        if let Some(code) = decode_status_code(&resp_bytes)
            && code != 0x0000
        {
            return Err(DispatchError::Transport {
                detail: format!(
                    "IPP 操作失败，设备返回状态码 0x{:04X}（端点 {}）",
                    code, self.endpoint_url
                ),
            });
        }

        match decode_job_id(&resp_bytes) {
            Some(id) => Ok(id.to_string()),
            // 状态为成功但未带 job-id：作业多半已被接受，不谎报失败以免重试重复出纸，
            // 改用可关联任务号的合成标识并显式记 WARN（后续不可回查真实作业号）。
            None => {
                warn!(
                    job_code = %job.job_code,
                    endpoint = %self.endpoint_url,
                    "IPP 成功响应未解析到 job-id，改用合成外部作业号"
                );
                Ok(format!("ipp-{}", job.job_code))
            }
        }
    }
}

/// 对字节序列求一个稳定的 31-bit 非负整数，用作 IPP request-id。
///
/// request-id 只要求在同一 IPP 客户端连接上单调/可区分（RFC 8011 §4.1.5），这里用
/// FNV-1a 从任务号导出确定值，避免引入随机源导致同任务重放无法对齐。
fn fnv_hash(input: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for &b in input {
        hash ^= b as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash & 0x7fff_ffff
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 编码请求头（版本/操作号/请求号/属性组起始标签）必须逐字节等于固定夹具。
    #[test]
    fn encodes_deterministic_header_fixture() {
        let req = encode_print_job_request("ipp://p", 0x0102_0304, 2, b"D");
        // 版本 1.1
        assert_eq!(&req[0..2], &[VERSION_MAJOR, VERSION_MINOR]);
        // 操作号 Print-Job = 0x0002 大端
        assert_eq!(&req[2..4], &[0x00, 0x02]);
        // 请求号 4 字节大端
        assert_eq!(&req[4..8], &[0x01, 0x02, 0x03, 0x04]);
        // 紧跟 operation-attributes-tag
        assert_eq!(req[8], TAG_OPERATION_ATTRS);
    }

    /// 编码必须包含关键属性名、结束标签，并把文档字节原样附加在末尾。
    #[test]
    fn encodes_required_attributes_and_document() {
        let doc = b"HELLO";
        let req = encode_print_job_request("ipp://printer/queue", 7, 3, doc);
        let text = String::from_utf8_lossy(&req);
        assert!(text.contains("attributes-charset"));
        assert!(text.contains("attributes-natural-language"));
        assert!(text.contains("printer-uri"));
        assert!(text.contains("requested-attributes"));
        assert!(text.contains("copies"));
        // 文档字节在结束标签之后原样出现于末尾
        assert!(req.ends_with(doc));
        assert!(req.contains(&TAG_END_OF_ATTRS));
    }

    /// 从合成的成功响应字节流解析 job-id（整数属性）应命中。
    #[test]
    fn decodes_job_id_from_synthetic_response() {
        let mut resp = Vec::new();
        resp.extend_from_slice(&[VERSION_MAJOR, VERSION_MINOR]); // 版本
        resp.extend_from_slice(&0x0000u16.to_be_bytes()); // 状态 successful-ok
        resp.extend_from_slice(&0x0000_0007u32.to_be_bytes()); // 请求号
        resp.push(TAG_JOB_ATTRS);
        // job-id = 42
        push_int_attr(&mut resp, "job-id", 42);
        // 另一属性干扰：job-state = 3
        push_int_attr(&mut resp, "job-state", 3);
        resp.push(TAG_END_OF_ATTRS);

        assert_eq!(decode_status_code(&resp), Some(0x0000));
        assert_eq!(decode_job_id(&resp), Some(42));
    }

    /// 响应不含 job-id 时解码返回 None（不误造作业号）。
    #[test]
    fn decodes_none_when_job_id_absent() {
        let mut resp = Vec::new();
        resp.extend_from_slice(&[VERSION_MAJOR, VERSION_MINOR]);
        resp.extend_from_slice(&0x0000u16.to_be_bytes());
        resp.extend_from_slice(&0x0000_0001u32.to_be_bytes());
        resp.push(TAG_JOB_ATTRS);
        push_int_attr(&mut resp, "job-state", 3);
        resp.push(TAG_END_OF_ATTRS);
        assert_eq!(decode_job_id(&resp), None);
    }

    /// 错误状态码可被解出，供传输层判定 IPP 操作失败。
    #[test]
    fn decodes_error_status_code() {
        let mut resp = vec![VERSION_MAJOR, VERSION_MINOR];
        resp.extend_from_slice(&0x0400u16.to_be_bytes()); // client-error 区段
        assert_eq!(decode_status_code(&resp), Some(0x0400));
    }

    /// fnv_hash 对相同输入确定、且落在非负 31-bit 区间。
    #[test]
    fn fnv_hash_is_deterministic_and_non_negative() {
        let a = fnv_hash(b"JOB-1");
        let b = fnv_hash(b"JOB-1");
        assert_eq!(a, b);
        assert!(a & 0x8000_0000 == 0);
    }
}

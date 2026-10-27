/**
 * 标志（logo）图片文件的前端判定工具。
 *
 * 背景：浏览器 File.type 是**按扩展名查 MIME 表**的结果，不是内容判定——
 * `.PNG` 大写、系统 MIME 表异常（返回 image/x-png 或空串）都会让按 `file.type`
 * 的白名单误拒真实 PNG；改名/转码遗留的伪 PNG（扩展名 png 但内容 JPEG 等）
 * 则会漏进网络请求、由后端按魔数拒绝，用户看到的是"上传的就是 PNG 却被拒"。
 * 判定统一改为**按文件头魔数**（与后端 company_profile_service 的 PNG 魔数判据同源），
 * 错误信息如实说明检测到的真实格式。
 */

/** 图片格式 token 词表（与后端魔数判据及提示文案键对应） */
export type ImageFormatToken = 'png' | 'jpeg' | 'gif' | 'webp' | 'bmp' | 'ico' | 'unknown';

/** PNG 签名 8 字节（RFC 2048 / W3C PNG 规范固定文件头） */
const PNG_SIGNATURE = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

/**
 * 按文件头魔数识别图片格式。
 * 入参：文件起始字节切片（≥12 字节足够覆盖全部词表签名）；
 * 出参：格式 token；不足签名长度或无一命中时为 'unknown'。
 */
export function detectImageFormat(bytes: Uint8Array): ImageFormatToken {
  const starts = (sig: number[]): boolean =>
    bytes.length >= sig.length && sig.every((b, i) => bytes[i] === b);
  if (starts(PNG_SIGNATURE)) return 'png';
  if (bytes.length >= 3 && bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) {
    return 'jpeg';
  }
  if (starts([0x47, 0x49, 0x46, 0x38])) return 'gif';
  if (
    starts([0x52, 0x49, 0x46, 0x46]) &&
    bytes.length >= 12 &&
    bytes[8] === 0x57 &&
    bytes[9] === 0x45 &&
    bytes[10] === 0x42 &&
    bytes[11] === 0x50
  ) {
    return 'webp';
  }
  if (starts([0x42, 0x4d])) return 'bmp';
  if (starts([0x00, 0x00, 0x01, 0x00])) return 'ico';
  return 'unknown';
}

/**
 * 读取文件前 12 字节用于魔数判定（不整读大文件）。
 * 入参：File/Blob（切片接口可用的任意对象）；
 * 出参：起始字节的 Uint8Array；读取失败原样抛出交由调用方提示。
 */
export async function readFileHead(file: Blob, headBytes = 12): Promise<Uint8Array> {
  const head = file.slice(0, headBytes);
  return new Uint8Array(await head.arrayBuffer());
}

/**
 * 把 data URL 归一为 base64 裸串（后端落库形态）。
 * 入参：FileReader.readAsDataURL 产出的 `data:<mime>;base64,<payload>`；
 * 出参：去前缀后的裸 base64；无前缀时原样返回（幂等，供重复调用安全）。
 */
export function stripDataUrlPrefix(dataUrl: string): string {
  return dataUrl.replace(/^data:[^;,]+;base64,/, '');
}

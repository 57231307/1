/**
 * logo-file.ts 单测：钉住"按魔数判类型"的判定面与 data URL 归一面。
 * 正向：真实 PNG 头（含大写后缀/MIME 缺失场景下仅凭字节）判 png；
 * 反向：伪装成 .png 的 JPEG/GIF/WebP/BMP/ICO 与随机字节被点名拒出真实格式。
 */
import { describe, expect, it } from 'vitest';
import { detectImageFormat, readFileHead, stripDataUrlPrefix } from '@/utils/logo-file';

const PNG_HEAD = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 0x0d]);
const JPEG_HEAD = new Uint8Array([0xff, 0xd8, 0xff, 0xe0, 0, 0x10, 0x4a, 0x46, 0x49, 0x46, 0, 1]);
const GIF_HEAD = new Uint8Array([0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 1, 0, 1, 0, 0, 0]);
const WEBP_HEAD = new Uint8Array([
  0x52, 0x49, 0x46, 0x46, 0x00, 0x00, 0x00, 0x00, 0x57, 0x45, 0x42, 0x50,
]);
const BMP_HEAD = new Uint8Array([0x42, 0x4d, 0x36, 0, 0, 0, 0, 0, 0, 0, 0x36, 0]);
const ICO_HEAD = new Uint8Array([0x00, 0x00, 0x01, 0x00, 1, 0, 1, 0, 32, 32, 0, 0]);

describe('detectImageFormat 按文件头魔数判类型', () => {
  it('真实 PNG 头判 png（不看扩展名与 MIME）', () => {
    expect(detectImageFormat(PNG_HEAD)).toBe('png');
  });

  it('伪装 png 的其它格式逐一点名判出真实格式', () => {
    expect(detectImageFormat(JPEG_HEAD)).toBe('jpeg');
    expect(detectImageFormat(GIF_HEAD)).toBe('gif');
    expect(detectImageFormat(WEBP_HEAD)).toBe('webp');
    expect(detectImageFormat(BMP_HEAD)).toBe('bmp');
    expect(detectImageFormat(ICO_HEAD)).toBe('ico');
  });

  it('字节不足或无一命中判 unknown，不猜测', () => {
    expect(detectImageFormat(new Uint8Array([0x89, 0x50]))).toBe('unknown');
    expect(detectImageFormat(new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]))).toBe(
      'unknown'
    );
  });

  it('截断的 PNG 前 4 字节签名仍判 png（与后端 starts_with 前缀判据同宽）', () => {
    expect(detectImageFormat(PNG_HEAD.slice(0, 4))).toBe('png');
  });
});

describe('readFileHead 只读起始字节', () => {
  it('切片长度等于请求头长且内容取自文件头部', async () => {
    const full = new Uint8Array(64).fill(0);
    full.set(PNG_HEAD, 0);
    const blob = {
      slice: (_from: number, to: number) => ({
        arrayBuffer: async () => full.slice(0, to).buffer,
      }),
    } as unknown as Blob;
    const head = await readFileHead(blob, 12);
    expect(head.length).toBe(12);
    expect(detectImageFormat(head)).toBe('png');
  });
});

describe('stripDataUrlPrefix 归一 base64 裸串', () => {
  it('去掉 data:image/png;base64, 前缀', () => {
    expect(stripDataUrlPrefix('data:image/png;base64,iVBORw0KGgo=')).toBe('iVBORw0KGgo=');
  });

  it('MIME 带异常变体（image/x-png）同样剥前缀', () => {
    expect(stripDataUrlPrefix('data:image/x-png;base64,iVBORw0KGgo=')).toBe('iVBORw0KGgo=');
  });

  it('无前置 data: 段时不截 body 中部的伪 base64 子串，原样幂等返回', () => {
    const bare = 'iVBORw0KGgo=';
    expect(stripDataUrlPrefix(bare)).toBe(bare);
    expect(stripDataUrlPrefix('AAdata:image/png;base64,BB')).toBe('AAdata:image/png;base64,BB');
  });
});

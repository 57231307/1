#!/usr/bin/env python3
"""删除 locales 文件中重复的顶层命名空间块，保留第一个。"""
import re
import sys

def process_file(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # 找到所有顶层命名空间的位置（2 空格缩进的 key: {）
    # 策略：找到 "  bpm: {" 的所有位置，保留第一个，删除后续的整个块
    
    # 找到 "  bpm: {" 开头的行
    lines = content.split('\n')
    
    # 找到所有 "  bpm: {" 的行号
    bpm_starts = []
    for i, line in enumerate(lines):
        if line.strip() == 'bpm: {' and line.startswith('  bpm: {'):
            bpm_starts.append(i)
    
    print(f"Found {len(bpm_starts)} bpm blocks at lines: {[i+1 for i in bpm_starts]}")
    
    if len(bpm_starts) <= 1:
        print("No duplicate bpm blocks found, skipping.")
        return
    
    # 保留第一个，删除后续的
    # 需要找到每个 bpm 块的结束位置（匹配的 "  }," 行）
    blocks_to_delete = []
    for start_idx in bpm_starts[1:]:  # 跳过第一个
        # 从 start_idx 开始，找到匹配的 "  }," 或 "  }" 行
        depth = 1
        end_idx = start_idx + 1
        while end_idx < len(lines) and depth > 0:
            line = lines[end_idx]
            # 统计 { 和 } 的数量（忽略字符串中的）
            # 简单方法：检查行首的缩进来判断深度
            stripped = line.strip()
            if stripped.endswith('{') and not stripped.startswith('//'):
                depth += 1
            elif stripped == '},' or stripped == '}':
                depth -= 1
                if depth == 0:
                    break
            end_idx += 1
        
        blocks_to_delete.append((start_idx, end_idx))
        print(f"  Block at line {start_idx+1} ends at line {end_idx+1}")
    
    # 从后往前删除，避免行号偏移
    blocks_to_delete.reverse()
    for start_idx, end_idx in blocks_to_delete:
        # 删除从 start_idx 到 end_idx（包含）的行
        del lines[start_idx:end_idx+1]
    
    new_content = '\n'.join(lines)
    
    with open(file_path, 'w', encoding='utf-8') as f:
        f.write(new_content)
    
    print(f"Deleted {len(blocks_to_delete)} duplicate bpm blocks")
    print(f"File size: {len(content)} -> {len(new_content)} chars")

if __name__ == '__main__':
    for fp in [
        '/workspace/frontend/src/locales/zh-CN.ts',
        '/workspace/frontend/src/locales/en-US.ts',
    ]:
        print(f"\nProcessing {fp}...")
        process_file(fp)

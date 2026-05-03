#!/bin/bash
echo "=== 检查所有handler是否已注册 ==="

ls src/handlers/*_handler.rs | \
  sed 's/.*\///' | \
  sed 's/_handler\.rs//' | \
  sort > /tmp/handlers.txt

grep -oP '\w+_handler::\w+' src/routes/mod.rs | \
  sed 's/_handler::.*//' | \
  sort -u > /tmp/registered.txt

echo "未注册的handler:"
comm -23 /tmp/handlers.txt /tmp/registered.txt

echo ""
echo "已注册但可能不存在的handler:"
comm -13 /tmp/handlers.txt /tmp/registered.txt

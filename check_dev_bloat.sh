#!/bin/bash
echo "--- 🔍 SoloSoul 开发环境占用扫描 ---"

# 1. Go 编译缓存
echo -n "Go Build & Mod Cache: "
du -sh $(go env GOCACHE) $(go env GOPATH)/pkg/mod 2>/dev/null | awk '{print $1}' | paste -sd+ - | bc -s 2>/dev/null || echo "0B"

# 2. Node.js 依赖 (扫描当前目录下所有的 node_modules)
echo -n "node_modules (当前目录及子目录): "
find . -name "node_modules" -type d -prune | xargs du -sh | awk '{sum+=$1} END {print sum "GB"}'

# 3. Android 模拟器
echo -n "Android AVD Images: "
du -sh ~/.android/avd/*.avd/*.img 2>/dev/null | awk '{print $1}' | paste -sd+ - | bc -s 2>/dev/null || echo "0B"

# 4. OCR/Python 缓存 (PaddleOCR 等)
echo -n "PaddleOCR/Python Models: "
du -sh ~/.paddleocr ~/.cache/pip 2>/dev/null | awk '{print $1}' | paste -sd+ - | bc -s 2>/dev/null || echo "0B"

# 5. Xcode 派生数据 (如果你用过它编译)
echo -n "Xcode DerivedData: "
du -sh ~/Library/Developer/Xcode/DerivedData 2>/dev/null | awk '{print $1}'

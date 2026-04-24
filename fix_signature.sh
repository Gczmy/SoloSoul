#!/bin/bash

# 1. 自动定位应用路径
if [ -d "build/macos/Build/Products/Release/SoloSoul.app" ]; then
    APP_PATH="build/macos/Build/Products/Release/SoloSoul.app"
elif [ -d "flutter/build/macos/Build/Products/Release/SoloSoul.app" ]; then
    APP_PATH="flutter/build/macos/Build/Products/Release/SoloSoul.app"
else
    echo "❌ 找不到 SoloSoul.app，请在项目根目录或 flutter 目录下运行此脚本。"
    exit 1
fi

# 2. 配置信息
SHA1="A432EC36C0EF2CD554D9E9679CDAC754F414C072"

echo "🚀 执行原始签名法 (Ad-hoc Style)..."
echo "📍 目标路径: $APP_PATH"

# 3. 彻底清除所有扩展属性、隔离位和旧签名
echo "🧹 正在深度清理文件属性..."
sudo xattr -cr "$APP_PATH"

# 4. 彻底删除内部可能残留的 Entitlements 注入
# 我们不使用 --entitlements 参数，强制 codesign 仅进行身份验证
echo "🎯 正在进行裸签 (No Entitlements)..."
codesign --force --deep --sign "$SHA1" \
         --timestamp=none "$APP_PATH"

# 5. 强制刷新系统安全服务缓存
echo "🔄 刷新系统任务守卫..."
sudo killall taskgated || true

# 6. 最终验证
echo -e "\n🔎 验证签名信息 (Signature 应显示但 Entitlements 应为空):"
codesign -dvvv "$APP_PATH"

echo -e "\n✨ 修复完成！请直接运行测试："
echo "open $APP_PATH"

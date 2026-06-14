#!/usr/bin/env python3
"""
generate-nsis-assets.py
为 SoloSoul Windows NSIS 安装程序生成品牌位图资源。

输出：
- src-tauri/bundles/nsis/assets/welcome-sidebar.png  (164 x 314)
- src-tauri/bundles/nsis/assets/header-install.png   (150 x 57)
- src-tauri/bundles/nsis/assets/header-uninstall.png (150 x 57)

设计风格：Warm Stone + Liquid Glass，与前端设计系统保持一致。
"""

from __future__ import annotations

import math
import os
import re
import sys
from pathlib import Path

try:
    from PIL import Image, ImageDraw, ImageFilter, ImageFont
except ImportError:
    # 在缺少 Pillow 的构建环境（如干净 Windows 机器）中，若已有生成好的位图资源，
    # 则跳过重新生成，避免构建失败。需要重新生成时请安装 Pillow：
    #   python3 -m pip install Pillow
    ASSETS_DIR = Path(__file__).resolve().parent.parent / "src-tauri" / "bundles" / "nsis" / "assets"
    required = [
        ASSETS_DIR / "welcome-sidebar.bmp",
        ASSETS_DIR / "header-install.bmp",
        ASSETS_DIR / "header-uninstall.bmp",
    ]
    if all(p.exists() for p in required):
        print("Pillow not installed; using existing NSIS assets.")
        sys.exit(0)
    print(
        "Error: Pillow is required to generate NSIS assets and no existing assets were found.\n"
        "Install it with: python3 -m pip install Pillow",
        file=sys.stderr,
    )
    sys.exit(1)

# ── 项目色板（与 src/styles/tokens.css / themes.css 一致）───────────────────────
STONE_25 = (253, 252, 249)
STONE_50 = (250, 250, 246)
STONE_100 = (242, 240, 233)
STONE_200 = (229, 225, 214)
STONE_400 = (184, 176, 160)
STONE_600 = (122, 114, 101)
STONE_800 = (61, 56, 49)
CLAY_400 = (196, 146, 92)
SLATE_400 = (91, 124, 153)
WHITE = (255, 255, 255)

# ── 路径 ──────────────────────────────────────────────────────────────────────
ROOT = Path(__file__).resolve().parent.parent
ASSETS_DIR = ROOT / "src-tauri" / "bundles" / "nsis" / "assets"


def hex_to_rgb(hex_color: str) -> tuple[int, int, int]:
    hex_color = hex_color.lstrip("#")
    return tuple(int(hex_color[i : i + 2], 16) for i in (0, 2, 4))  # type: ignore[return-value]


def linear_gradient(
    size: tuple[int, int], color_top: tuple[int, int, int], color_bottom: tuple[int, int, int]
) -> Image.Image:
    """生成垂直线性渐变背景。"""
    width, height = size
    base = Image.new("RGB", size, color_top)
    draw = ImageDraw.Draw(base)
    for y in range(height):
        ratio = y / (height - 1) if height > 1 else 0
        r = int(color_top[0] + (color_bottom[0] - color_top[0]) * ratio)
        g = int(color_top[1] + (color_bottom[1] - color_top[1]) * ratio)
        b = int(color_top[2] + (color_bottom[2] - color_top[2]) * ratio)
        draw.line([(0, y), (width, y)], fill=(r, g, b))
    return base


def draw_rounded_rect(
    draw: ImageDraw.Draw,
    bbox: tuple[int, int, int, int],
    radius: int,
    fill: tuple[int, int, int, int] | None = None,
    outline: tuple[int, int, int, int] | None = None,
    width: int = 1,
) -> None:
    """绘制圆角矩形，支持透明填充。"""
    x1, y1, x2, y2 = bbox
    draw.rounded_rectangle(bbox, radius=radius, fill=fill, outline=outline, width=width)


# SoloSoul SVG 图标中的盾牌路径（与 src-tauri/icons/icon.svg 一致）
SVG_SHIELD_PATH = (
    "M480-80q-139-35-229.5-159.5T160-516v-244l320-120 320 120v244q0 152-90.5 276.5T480-80Z"
    "m0-84q104-33 172-132t68-220v-189l-240-90-240 90v189q0 121 68 220t172 132Z"
    "m0-316Z"
)
# icon.svg 中用于把 Material Symbols 路径适配到 1024x1024 viewBox 的矩阵。
SVG_SHIELD_TRANSFORM = (0.8, 0, 0, 0.8, 128, 897)


def _sample_quadratic_bezier(
    p0: tuple[float, float],
    p1: tuple[float, float],
    p2: tuple[float, float],
    steps: int = 24,
) -> list[tuple[float, float]]:
    """把二次贝塞尔曲线采样为折线点。"""
    pts: list[tuple[float, float]] = []
    for i in range(1, steps + 1):
        t = i / steps
        a = (1 - t) * (1 - t)
        b = 2 * (1 - t) * t
        c = t * t
        pts.append((a * p0[0] + b * p1[0] + c * p2[0], a * p0[1] + b * p1[1] + c * p2[1]))
    return pts


def _tokenize_svg_path(d: str) -> list[str]:
    """把 SVG path 的 d 属性拆分为命令字母与数字 token。"""
    return re.findall(r"[MmLlHhVvQqTtZz]|-?\d*\.?\d+(?:[eE][-+]?\d+)?", d)


def flatten_svg_path(
    d: str, transform: tuple[float, float, float, float, float, float]
) -> list[list[tuple[float, float]]]:
    """解析 SVG path 并返回若干子路径（已做矩阵变换）。"""
    tokens = _tokenize_svg_path(d)
    a, b, c, d_, e, f = transform

    def apply(px: float, py: float) -> tuple[float, float]:
        return (a * px + c * py + e, b * px + d_ * py + f)

    subpaths: list[list[tuple[float, float]]] = []
    current: list[tuple[float, float]] = []
    x = y = 0.0
    start_x = start_y = 0.0
    last_cx = last_cy = 0.0
    cmd: str | None = None
    i = 0

    def take(count: int) -> list[float]:
        nonlocal i
        vals: list[float] = []
        while len(vals) < count and i < len(tokens):
            tok = tokens[i]
            if tok[0].isalpha():
                break
            vals.append(float(tok))
            i += 1
        return vals

    while i < len(tokens):
        tok = tokens[i]
        if tok[0].isalpha():
            cmd = tok
            i += 1
        if cmd is None:
            break

        if cmd in ("M", "m"):
            vals = take(2)
            if len(vals) < 2:
                break
            nx, ny = vals
            if cmd == "m":
                x += nx
                y += ny
            else:
                x, y = nx, ny
            if current:
                subpaths.append(current)
            current = [apply(x, y)]
            start_x, start_y = x, y
            # 后续无命令的数字按直线处理
            cmd = "L" if cmd == "M" else "l"
        elif cmd in ("L", "l"):
            vals = take(2)
            if len(vals) < 2:
                break
            nx, ny = vals
            if cmd == "l":
                x += nx
                y += ny
            else:
                x, y = nx, ny
            current.append(apply(x, y))
        elif cmd in ("H", "h"):
            vals = take(1)
            if not vals:
                break
            nx = vals[0]
            x = x + nx if cmd == "h" else nx
            current.append(apply(x, y))
        elif cmd in ("V", "v"):
            vals = take(1)
            if not vals:
                break
            ny = vals[0]
            y = y + ny if cmd == "v" else ny
            current.append(apply(x, y))
        elif cmd in ("Q", "q"):
            vals = take(4)
            if len(vals) < 4:
                break
            qx, qy, nx, ny = vals
            if cmd == "q":
                qx += x
                qy += y
                nx += x
                ny += y
            pts = _sample_quadratic_bezier((x, y), (qx, qy), (nx, ny))
            current.extend(apply(px, py) for px, py in pts)
            x, y = nx, ny
            last_cx, last_cy = qx, qy
        elif cmd in ("T", "t"):
            vals = take(2)
            if len(vals) < 2:
                break
            nx, ny = vals
            if cmd == "t":
                nx += x
                ny += y
            # 对称控制点
            qx = x + (x - last_cx)
            qy = y + (y - last_cy)
            pts = _sample_quadratic_bezier((x, y), (qx, qy), (nx, ny))
            current.extend(apply(px, py) for px, py in pts)
            x, y = nx, ny
            last_cx, last_cy = qx, qy
        elif cmd in ("Z", "z"):
            if current:
                current.append(current[0])
                subpaths.append(current)
                current = []
            x, y = start_x, start_y
        else:
            i += 1

    if current:
        subpaths.append(current)

    # 过滤掉退化子路径（如 icon.svg 末尾的 m0-316Z 点）
    return [sp for sp in subpaths if len(sp) >= 3]


def draw_svg_shield(target: Image.Image, bbox: tuple[int, int, int, int]) -> None:
    """在 target（RGBA）的指定区域内绘制与 icon.svg 一致的白色盾牌。"""
    x1, y1, x2, y2 = bbox
    width = x2 - x1
    height = y2 - y1

    subpaths = flatten_svg_path(SVG_SHIELD_PATH, SVG_SHIELD_TRANSFORM)
    if not subpaths:
        return

    all_points = [p for sp in subpaths for p in sp]
    min_x = min(p[0] for p in all_points)
    max_x = max(p[0] for p in all_points)
    min_y = min(p[1] for p in all_points)
    max_y = max(p[1] for p in all_points)
    path_w = max_x - min_x
    path_h = max_y - min_y
    if path_w <= 0 or path_h <= 0:
        return

    scale = min(width / path_w, height / path_h)
    offset_x = x1 + (width - path_w * scale) / 2
    offset_y = y1 + (height - path_h * scale) / 2

    mapped: list[list[tuple[int, int]]] = []
    for sp in subpaths:
        mapped.append(
            [
                (int((px - min_x) * scale + offset_x), int((py - min_y) * scale + offset_y))
                for px, py in sp
            ]
        )

    # 用灰度蒙版实现外轮廓白、内镂空透背景
    mask = Image.new("L", target.size, 0)
    draw = ImageDraw.Draw(mask)
    draw.polygon(mapped[0], fill=255)
    for sp in mapped[1:]:
        draw.polygon(sp, fill=0)

    white_layer = Image.new("RGBA", target.size, WHITE + (255,))
    white_layer.putalpha(mask)
    target.alpha_composite(white_layer)


def draw_glass_panel(
    img: Image.Image,
    bbox: tuple[int, int, int, int],
    radius: int,
    fill_rgba: tuple[int, int, int, int],
    highlight_rgba: tuple[int, int, int, int],
    shadow_rgba: tuple[int, int, int, int],
) -> None:
    """绘制带高光和阴影的玻璃拟态面板。"""
    overlay = Image.new("RGBA", img.size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(overlay)

    # 面板本体
    draw.rounded_rectangle(bbox, radius=radius, fill=fill_rgba)

    # 顶部内高光
    x1, y1, x2, y2 = bbox
    highlight_height = max(1, (y2 - y1) // 4)
    highlight_box = (x1 + 1, y1 + 1, x2 - 1, y1 + highlight_height)
    draw.rounded_rectangle(highlight_box, radius=max(1, radius - 1), fill=highlight_rgba)

    # 外阴影（通过模糊实现）
    shadow_layer = Image.new("RGBA", img.size, (0, 0, 0, 0))
    shadow_draw = ImageDraw.Draw(shadow_layer)
    shadow_draw.rounded_rectangle(bbox, radius=radius, fill=shadow_rgba)
    shadow_layer = shadow_layer.filter(ImageFilter.GaussianBlur(radius=8))

    img.paste(shadow_layer, (0, 0), shadow_layer)
    img.paste(overlay, (0, 0), overlay)


def get_font(size: int, bold: bool = False) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    """加载同时支持中英文的系统字体，失败则回退到默认字体。"""
    # 按平台/美观度排序；TTF/OTC/TTC 集合需要指定索引时由 Pillow 自动处理。
    candidates = [
        # macOS — 冬青黑体简体中文，与 SF Pro 风格接近
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        # macOS — 黑体-简
        "/System/Library/Fonts/STHeiti Medium.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        # Linux — Noto Sans CJK
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        # Windows — 微软雅黑（包含英文，但优先用系统 UI 字体）
        "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/msyhbd.ttc",
        # Windows — Segoe UI Variable（仅英文，若前面都失败则回退）
        "C:/Windows/Fonts/segoeui.ttf",
        "C:/Windows/Fonts/segoeuib.ttf",
        # 最终兜底：Arial Unicode 支持多语言但较宽
        "/Library/Fonts/Arial Unicode.ttf",
    ]
    # 优先顺序与 weight：把 Bold 版本放在前面
    if bold:
        candidates = [
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/System/Library/Fonts/STHeiti Medium.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Bold.ttc",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Bold.ttc",
            "C:/Windows/Fonts/msyhbd.ttc",
            "C:/Windows/Fonts/msyh.ttc",
            "C:/Windows/Fonts/segoeuib.ttf",
            "C:/Windows/Fonts/segoeui.ttf",
            "/Library/Fonts/Arial Unicode.ttf",
        ]
    for path in candidates:
        if os.path.exists(path):
            try:
                return ImageFont.truetype(path, size)
            except Exception:
                continue
    return ImageFont.load_default()


def generate_welcome_sidebar() -> Image.Image:
    """生成 164x314 的欢迎/完成页侧边栏位图。"""
    width, height = 164, 314
    # 暖石渐变背景：与 MUI_BGCOLOR (#FDFCF9) 融为一体的柔和过渡
    img = linear_gradient((width, height), STONE_25, STONE_50)
    img = img.convert("RGBA")

    # 玻璃拟态卡片
    margin = 12
    card_bbox = (margin, margin, width - margin, height - margin)
    draw_glass_panel(
        img,
        card_bbox,
        radius=16,
        fill_rgba=(255, 255, 255, 160),
        highlight_rgba=(255, 255, 255, 80),
        shadow_rgba=(31, 28, 24, 25),
    )

    draw = ImageDraw.Draw(img)

    # 品牌渐变圆角方块（作为 logo 背景）
    logo_bg_size = 64
    logo_bg_bbox = (
        (width - logo_bg_size) // 2,
        56,
        (width + logo_bg_size) // 2,
        56 + logo_bg_size,
    )
    # 创建渐变 logo 背景
    logo_bg = linear_gradient((logo_bg_size, logo_bg_size), SLATE_400, CLAY_400)
    logo_bg = logo_bg.convert("RGBA")
    # 圆角蒙版
    mask = Image.new("L", (logo_bg_size, logo_bg_size), 0)
    mask_draw = ImageDraw.Draw(mask)
    mask_draw.rounded_rectangle((0, 0, logo_bg_size, logo_bg_size), radius=18, fill=255)
    logo_bg.putalpha(mask)
    # 内部高光
    highlight = Image.new("RGBA", (logo_bg_size, logo_bg_size), (255, 255, 255, 0))
    h_draw = ImageDraw.Draw(highlight)
    h_draw.rounded_rectangle((0, 0, logo_bg_size, logo_bg_size // 3), radius=18, fill=(255, 255, 255, 40))
    logo_bg = Image.alpha_composite(logo_bg, highlight)

    # 在 logo 背景上绘制白色盾牌
    draw_svg_shield(logo_bg, (0, 0, logo_bg_size, logo_bg_size))

    img.paste(logo_bg, (logo_bg_bbox[0], logo_bg_bbox[1]), logo_bg)

    # 品牌名
    font_brand = get_font(18, bold=True)
    brand_text = "SoloSoul"
    bbox = draw.textbbox((0, 0), brand_text, font=font_brand)
    text_w = bbox[2] - bbox[0]
    draw.text(((width - text_w) / 2, 138), brand_text, font=font_brand, fill=STONE_800)

    # 中文品牌名
    font_cn = get_font(12)
    cn_text = "独 灵"
    bbox = draw.textbbox((0, 0), cn_text, font=font_cn)
    text_w = bbox[2] - bbox[0]
    draw.text(((width - text_w) / 2, 164), cn_text, font=font_cn, fill=STONE_600)

    # 分隔线
    line_y = 200
    draw.line([(42, line_y), (width - 42, line_y)], fill=STONE_200, width=1)

    # 标语
    font_tag = get_font(10)
    taglines = ["Local-first", "Privacy-first"]
    y = 218
    for line in taglines:
        bbox = draw.textbbox((0, 0), line, font=font_tag)
        text_w = bbox[2] - bbox[0]
        draw.text(((width - text_w) / 2, y), line, font=font_tag, fill=STONE_600)
        y += 18

    return img


def generate_header(title: str, subtitle: str | None = None) -> Image.Image:
    """生成 150x57 的顶部 header 位图。"""
    width, height = 150, 57
    img = linear_gradient((width, height), STONE_25, STONE_50)
    img = img.convert("RGBA")

    draw = ImageDraw.Draw(img)

    # 左侧小 logo 背景
    logo_size = 34
    logo_bbox = (12, (height - logo_size) // 2, 12 + logo_size, (height + logo_size) // 2)
    logo_bg = linear_gradient((logo_size, logo_size), SLATE_400, CLAY_400)
    logo_bg = logo_bg.convert("RGBA")
    mask = Image.new("L", (logo_size, logo_size), 0)
    mask_draw = ImageDraw.Draw(mask)
    mask_draw.rounded_rectangle((0, 0, logo_size, logo_size), radius=9, fill=255)
    logo_bg.putalpha(mask)

    # 在 logo 背景上绘制白色盾牌
    draw_svg_shield(logo_bg, (0, 0, logo_size, logo_size))

    img.paste(logo_bg, (logo_bbox[0], logo_bbox[1]), logo_bg)

    # 标题
    font_title = get_font(14, bold=True)
    draw.text((logo_bbox[2] + 10, 12), title, font=font_title, fill=STONE_800)

    # 副标题
    if subtitle:
        font_sub = get_font(9)
        draw.text((logo_bbox[2] + 10, 32), subtitle, font=font_sub, fill=STONE_600)

    return img


def save_bmp(img: Image.Image, path: Path) -> None:
    """保存为 NSIS/MUI2 兼容的 24-bit BMP（无 alpha，背景需与 MUI_BGCOLOR 一致）。"""
    # 合并到不透明暖石背景，确保圆角/透明区域不会显示为黑色
    background = Image.new("RGB", img.size, STONE_25)
    background.paste(img.convert("RGBA"), mask=img.split()[3])
    background.save(path, "BMP")


def main() -> None:
    ASSETS_DIR.mkdir(parents=True, exist_ok=True)

    sidebar = generate_welcome_sidebar()
    sidebar_path = ASSETS_DIR / "welcome-sidebar.bmp"
    save_bmp(sidebar, sidebar_path)
    print(f"Generated: {sidebar_path}")

    header_install = generate_header("SoloSoul", "Setup")
    header_install_path = ASSETS_DIR / "header-install.bmp"
    save_bmp(header_install, header_install_path)
    print(f"Generated: {header_install_path}")

    header_uninstall = generate_header("SoloSoul", "Uninstall")
    header_uninstall_path = ASSETS_DIR / "header-uninstall.bmp"
    save_bmp(header_uninstall, header_uninstall_path)
    print(f"Generated: {header_uninstall_path}")


if __name__ == "__main__":
    main()

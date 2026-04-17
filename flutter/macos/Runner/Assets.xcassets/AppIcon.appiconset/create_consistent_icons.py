import os
import io
import cairosvg
import numpy as np
from PIL import Image, ImageDraw, ImageFilter

# =========================
# 插件定义区
# =========================

def plugin_gradient_background(ctx):
    size = ctx["size"]
    radius = ctx["radius"]

    gradient = np.zeros((size, size, 3), dtype=np.uint8)

    color_top = np.array([167, 85, 247])
    color_bottom = np.array([99, 102, 241])

    for y in range(size):
        t = y / size
        t = t * t * (3 - 2 * t)  # ease
        color = (1 - t) * color_top + t * color_bottom
        gradient[y, :] = color

    gradient_img = Image.fromarray(gradient, 'RGB')

    mask = Image.new('L', (size, size), 0)
    draw = ImageDraw.Draw(mask)
    draw.rounded_rectangle([0, 0, size, size], radius=radius, fill=255)

    ctx["img"].paste(gradient_img, (0, 0), mask)


def plugin_highlight(ctx):
    size = ctx["size"]

    highlight = Image.new('RGBA', (size, size), (255, 255, 255, 0))
    draw = ImageDraw.Draw(highlight)

    h = int(size * 0.45)
    for y in range(h):
        alpha = int(80 * (1 - y / h))
        draw.rectangle([0, y, size, y+1], fill=(255, 255, 255, alpha))

    highlight = highlight.filter(ImageFilter.GaussianBlur(radius=size*0.02))
    ctx["img"] = Image.alpha_composite(ctx["img"], highlight)


def plugin_inner_shadow(ctx):
    size = ctx["size"]
    radius = ctx["radius"]

    shadow = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(shadow)

    for i in range(8):
        alpha = int(20 * (1 - i/8))
        draw.rounded_rectangle(
            [i, i, size-i, size-i],
            radius=radius,
            outline=(0, 0, 0, alpha)
        )

    ctx["img"] = Image.alpha_composite(ctx["img"], shadow)


def plugin_outer_shadow(ctx):
    size = ctx["size"]
    radius = ctx["radius"]

    shadow = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(shadow)

    draw.rounded_rectangle(
        [int(size*0.05), int(size*0.05), int(size*0.95), int(size*0.95)],
        radius=radius,
        fill=(0, 0, 0, 80)
    )

    shadow = shadow.filter(ImageFilter.GaussianBlur(radius=size*0.06))
    ctx["img"] = Image.alpha_composite(shadow, ctx["img"])


def plugin_load_svg(ctx):
    size = ctx["size"]
    icon_size = int(size * 0.58)

    with open(ctx["svg_path"], 'r') as f:
        svg = f.read()

    ctx["svg_content"] = svg
    ctx["icon_size"] = icon_size


def plugin_force_white(ctx):
    svg = ctx["svg_content"]

    svg = svg.replace('#1f1f1f', '#FFFFFF')
    svg = svg.replace('fill="#1f1f1f"', 'fill="#FFFFFF"')
    svg = svg.replace('fill:#1f1f1f', 'fill:#FFFFFF')

    ctx["svg_content"] = svg


def plugin_render_svg(ctx):
    png_data = cairosvg.svg2png(
        bytestring=ctx["svg_content"].encode('utf-8'),
        output_width=ctx["icon_size"],
        output_height=ctx["icon_size"]
    )

    ctx["glyph"] = Image.open(io.BytesIO(png_data)).convert('RGBA')


def plugin_glyph_shadow(ctx):
    size = ctx["size"]
    glyph = ctx["glyph"]

    shadow = glyph.filter(ImageFilter.GaussianBlur(radius=size*0.01))
    ctx["glyph_shadow"] = shadow


def plugin_composite_glyph(ctx):
    size = ctx["size"]
    icon_size = ctx["icon_size"]

    gx = (size - icon_size) // 2
    gy = (size - icon_size) // 2

    if "glyph_shadow" in ctx:
        ctx["img"].paste(
            ctx["glyph_shadow"],
            (gx, gy + int(size*0.01)),
            ctx["glyph_shadow"]
        )

    ctx["img"].paste(ctx["glyph"], (gx, gy), ctx["glyph"])


# =========================
# 渲染管线（可自由开关）
# =========================

PIPELINE = [
    plugin_gradient_background,

    # plugin_highlight,       # ❌ 注释掉可关闭高光
    plugin_inner_shadow,    # ❌ 注释掉可关闭内阴影
    plugin_outer_shadow,    # ❌ 注释掉可关闭外阴影

    plugin_load_svg,
    plugin_force_white,     # ❌ 注释掉保留原色
    plugin_render_svg,

    # plugin_glyph_shadow,    # ❌ 注释掉无glyph阴影
    plugin_composite_glyph,
]


# =========================
# 主函数
# =========================

def render_icon(svg_path, size):
    ctx = {
        "size": size,
        "radius": int(size * 0.225),
        "svg_path": svg_path,
        "img": Image.new('RGBA', (size, size), (0, 0, 0, 0))
    }

    for plugin in PIPELINE:
        plugin(ctx)

    return ctx["img"]


# =========================
# 批量导出
# =========================

def export(svg_path):
    sizes = [16, 32, 64, 128, 256, 512, 1024]

    for s in sizes:
        img = render_icon(svg_path, s)
        img.save(f"app_icon_{s}.png")

    print("完成")


# =========================
# 使用
# =========================

export("./shield_24dp_1F1F1F_FILL0_wght400_GRAD0_opsz24.svg")
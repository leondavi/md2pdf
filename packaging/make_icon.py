#!/usr/bin/env python3
"""Generate a 1024x1024 app icon PNG for md2pdf.

The icon is a rounded blue squircle with a large white digit "2" as the hero,
and a small "md  pdf" caption underneath.
Output: packaging/icon-master.png
"""
import os
from PIL import Image, ImageDraw, ImageFont

SIZE = 1024
OUT = os.path.join(os.path.dirname(__file__), "icon-master.png")


def load_font(size, bold=True):
    candidates = [
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf" if bold
        else "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/System/Library/Fonts/HelveticaNeue.ttc",
        "/System/Library/Fonts/Helvetica.ttc",
    ]
    for path in candidates:
        if os.path.exists(path):
            try:
                return ImageFont.truetype(path, size)
            except Exception:
                pass
    return ImageFont.load_default()


def vertical_gradient(size, top, bottom):
    base = Image.new("RGB", (1, size), 0)
    for y in range(size):
        t = y / (size - 1)
        base.putpixel((0, y), tuple(
            int(top[i] + (bottom[i] - top[i]) * t) for i in range(3)
        ))
    return base.resize((size, size))


def main():
    img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))

    # Rounded squircle background with a blue gradient.
    grad = vertical_gradient(SIZE, (61, 130, 246), (29, 78, 200)).convert("RGBA")
    mask = Image.new("L", (SIZE, SIZE), 0)
    md = ImageDraw.Draw(mask)
    margin = int(SIZE * 0.07)
    md.rounded_rectangle([margin, margin, SIZE - margin, SIZE - margin],
                         radius=int(SIZE * 0.235), fill=255)
    img.paste(grad, (0, 0), mask)

    draw = ImageDraw.Draw(img)

    # Soft inner highlight at the top for a glossy feel.
    hl = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    hd = ImageDraw.Draw(hl)
    hd.ellipse([int(SIZE * 0.10), int(-SIZE * 0.55),
                int(SIZE * 0.90), int(SIZE * 0.45)],
               fill=(255, 255, 255, 38))
    img = Image.alpha_composite(img, Image.composite(
        hl, Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0)), mask))
    draw = ImageDraw.Draw(img)

    # Hero digit "2".
    font = load_font(int(SIZE * 0.66))
    digit = "2"
    bb = draw.textbbox((0, 0), digit, font=font)
    tw, th = bb[2] - bb[0], bb[3] - bb[1]
    tx = (SIZE - tw) // 2 - bb[0]
    ty = (SIZE - th) // 2 - bb[1] - int(SIZE * 0.045)
    # Drop shadow then the digit.
    draw.text((tx + 10, ty + 12), digit, font=font, fill=(12, 30, 80, 110))
    draw.text((tx, ty), digit, font=font, fill=(255, 255, 255, 255))

    # Caption: md • pdf
    cap_font = load_font(int(SIZE * 0.105))
    caption = "md  pdf"
    cb = draw.textbbox((0, 0), caption, font=cap_font)
    cw = cb[2] - cb[0]
    cx = (SIZE - cw) // 2 - cb[0]
    cy = int(SIZE * 0.785)
    draw.text((cx, cy), caption, font=cap_font, fill=(255, 255, 255, 235))
    # Arrow dot between md and pdf.
    dot_r = int(SIZE * 0.016)
    draw.ellipse([SIZE // 2 - dot_r, cy + int(SIZE * 0.055) - dot_r,
                  SIZE // 2 + dot_r, cy + int(SIZE * 0.055) + dot_r],
                 fill=(255, 255, 255, 200))

    img.save(OUT)
    print("wrote", OUT)


if __name__ == "__main__":
    main()

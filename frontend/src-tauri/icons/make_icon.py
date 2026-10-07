"""Draws the Cornflake app icon (one flake on a dark rounded square) at 1024 px.
Regenerate the platform icon set with:  npx pnpm@9 tauri icon src-tauri/icons/cornflake-1024.png
"""
from PIL import Image, ImageDraw

S = 4096  # draw large, downsample for smooth edges
OUT = 1024
img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(img)

d.rounded_rectangle([0, 0, S - 1, S - 1], radius=int(S * 0.22), fill=(20, 33, 61, 255))

# An irregular flake: a slightly lopsided polygon reads as "cornflake" rather than a generic blob
flake = [
    (0.30, 0.34), (0.43, 0.24), (0.58, 0.27), (0.71, 0.22), (0.78, 0.36),
    (0.74, 0.50), (0.80, 0.63), (0.68, 0.76), (0.52, 0.73), (0.38, 0.80),
    (0.26, 0.68), (0.28, 0.54), (0.20, 0.44),
]
d.polygon([(x * S, y * S) for x, y in flake], fill=(242, 183, 5, 255))

img.resize((OUT, OUT), Image.LANCZOS).save("cornflake-1024.png")
print("wrote cornflake-1024.png")

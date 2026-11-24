"""Génère l'icône de l'app « Bilan » (PNG + ICO multi-tailles).

Carré arrondi bleu nuit plein, 4 barres d'histogramme crème aux hauteurs croissantes,
la dernière en orange. Pas de texte.
Usage : python assets/make_icon.py
"""
from pathlib import Path
from PIL import Image, ImageDraw

NAVY = (19, 34, 58, 255)      # #13223A
CREAM = (242, 230, 204, 255)  # #F2E6CC
ORANGE = (242, 140, 50, 255)  # #F28C32

S = 1024                      # repère de dessin
SS = 4                        # suréchantillonnage pour des bords nets
BG_RADIUS = 220
BAR_W = 128
BAR_GAP = 56
BAR_RADIUS = 24
BASELINE = 824
BAR_HEIGHTS = (240, 380, 520, 660)  # croissantes, de gauche à droite

ICO_SIZES = [16, 24, 32, 48, 64, 128, 256]


def draw(size_px: int) -> Image.Image:
    k = SS * size_px / S
    W = int(size_px * SS)
    img = Image.new("RGBA", (W, W), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    s = lambda v: int(round(v * k))

    # Fond : carré arrondi plein
    d.rounded_rectangle([0, 0, s(S) - 1, s(S) - 1], radius=s(BG_RADIUS), fill=NAVY)

    # Barres d'histogramme, centrées horizontalement
    total_w = len(BAR_HEIGHTS) * BAR_W + (len(BAR_HEIGHTS) - 1) * BAR_GAP
    x = (S - total_w) // 2
    for i, h in enumerate(BAR_HEIGHTS):
        color = ORANGE if i == len(BAR_HEIGHTS) - 1 else CREAM
        d.rounded_rectangle(
            [s(x), s(BASELINE - h), s(x + BAR_W), s(BASELINE)],
            radius=s(BAR_RADIUS),
            fill=color,
        )
        x += BAR_W + BAR_GAP

    return img.resize((size_px, size_px), Image.LANCZOS)


def main() -> None:
    out = Path(__file__).resolve().parent
    draw(256).save(out / "icon.png")
    draw(64).save(out / "icon64.png")
    frames = [draw(n) for n in ICO_SIZES]
    frames[-1].save(
        out / "icon.ico",
        format="ICO",
        sizes=[(n, n) for n in ICO_SIZES],
        append_images=frames[:-1],
    )
    print("ok:", out)


if __name__ == "__main__":
    main()

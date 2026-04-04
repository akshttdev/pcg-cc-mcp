#!/usr/bin/env python3
"""
Generate Section 6 transparent overlay PNGs for Sami Satoshi Ep.01
using Pillow — no browser capture, guaranteed alpha transparency.
All dimensions derived from CSS: 720x1280 canvas, 1cqw = 7.2px
"""
from PIL import Image, ImageDraw, ImageFont, ImageFilter
import os, math

W, H = 720, 1280
CQW = 7.2  # 1cqw in px at 720px wide

# Colors
GOLD      = (175, 144,  65, 255)
GOLD_LT   = (212, 185, 106, 255)
GOLD_DK   = (122,  98,  40, 255)
WHITE     = (255, 255, 255, 255)
OFF_WHITE = (255, 255, 255, 210)   # rgba(255,255,255,0.82)
MUTED     = (255, 255, 255, 127)   # rgba(255,255,255,0.5)
BLACK_92  = (  0,   0,   0, 234)   # rgba(0,0,0,0.92)
BLACK_74  = (  0,   0,   0, 189)   # rgba(0,0,0,0.74)
BLACK_75  = (  0,   0,   0, 191)   # rgba(0,0,0,0.75)
TRANSP    = (  0,   0,   0,   0)

OUT = '/tmp/pcg_ep01_overlays'
os.makedirs(OUT, exist_ok=True)

LOGO_PATH   = '/home/pythia/pcg-cc-mcp/docs/brand-assets/lower-thirds/pcg-logo.png'
CINZEL      = '/home/pythia/pcg-cc-mcp/dev_assets/fonts/cinzel_bold.ttf'
INTER_PATH  = '/usr/share/fonts/opentype/fira/FiraSansCondensed-Medium.otf'

def font(path, size):
    try:
        return ImageFont.truetype(path, int(size))
    except:
        return ImageFont.load_default()

# Load fonts at common sizes (cqw-derived)
f_cinzel_pcg  = font(CINZEL,     int(2.8 * CQW))   # 20px  — "PCG"
f_cinzel_name = font(CINZEL,     int(5.0 * CQW))   # 36px  — person name
f_cinzel_ep   = font(CINZEL,     int(2.0 * CQW))   # 14px  — "EP. 01"
f_inter_sm    = font(INTER_PATH, int(1.6 * CQW))   # 11px  — "Tech Briefing" / date
f_inter_title = font(INTER_PATH, int(2.8 * CQW))   # 20px  — person title
f_inter_tag   = font(INTER_PATH, int(1.7 * CQW))   # 12px  — broll ctx tag
f_inter_ctx   = font(INTER_PATH, int(2.5 * CQW))   # 18px  — broll ctx line

def load_logo(size_px):
    logo = Image.open(LOGO_PATH).convert('RGBA')
    logo = logo.resize((size_px, size_px), Image.LANCZOS)
    return logo

def new_canvas():
    return Image.new('RGBA', (W, H), TRANSP)

def draw_pcg_bug(img):
    """PCG logo bug — top-left: logo + 'PCG' + 'Tech Briefing'"""
    d = ImageDraw.Draw(img)
    logo_size = int(9 * CQW)   # 65px
    x = int(4 * CQW)           # 29px
    y = int(0.05 * H)          # 64px
    logo = load_logo(logo_size)
    img.alpha_composite(logo, (x, y))
    tx = x + logo_size + int(2 * CQW)
    # "PCG" in gold
    d.text((tx, y + 2), 'PCG', font=f_cinzel_pcg, fill=GOLD)
    pcg_h = f_cinzel_pcg.getbbox('PCG')[3]
    # "Tech Briefing" muted white
    d.text((tx, y + pcg_h + int(0.3 * CQW) + 2), 'TECH BRIEFING', font=f_inter_sm, fill=MUTED)

def draw_ep_badge(img):
    """EP badge — top-right: gold pill 'EP. 01' + dark glass 'Mar 2026'"""
    d = ImageDraw.Draw(img)
    right_edge = W - int(0.04 * W)    # 4% from right = 691px
    top_y      = int(0.05 * H)        # 64px

    ep_text  = 'EP. 01'
    date_text = 'MAR 2026'
    pad_v = int(1 * CQW)    # 7px
    pad_h = int(2 * CQW)    # 14px
    pad_h2 = int(1.8 * CQW) # 13px

    ep_bb   = f_cinzel_ep.getbbox(ep_text)
    date_bb = f_inter_sm.getbbox(date_text)
    ep_w    = ep_bb[2] + 2 * pad_h
    date_w  = date_bb[2] + 2 * pad_h2
    ep_h    = max(ep_bb[3], date_bb[3]) + 2 * pad_v

    x1 = right_edge - ep_w - date_w
    # Gold EP pill
    ep_pill = Image.new('RGBA', (ep_w, ep_h), TRANSP)
    pd = ImageDraw.Draw(ep_pill)
    for i in range(ep_h):
        t = i / ep_h
        r = int(212 + (175 - 212) * t)
        g = int(185 + (144 - 185) * t)
        b = int(106 + ( 65 - 106) * t)
        pd.line([(0, i), (ep_w, i)], fill=(r, g, b, 255))
    pd.text((pad_h, pad_v), ep_text, font=f_cinzel_ep, fill=(0, 0, 0, 255))
    img.alpha_composite(ep_pill, (x1, top_y))
    # Dark glass date pill
    date_pill = Image.new('RGBA', (date_w, ep_h), BLACK_75)
    dd = ImageDraw.Draw(date_pill)
    # gold border
    dd.rectangle([(0,0),(date_w-1, ep_h-1)], outline=(175,144,65,64))
    dd.text((pad_h2, pad_v), date_text, font=f_inter_sm, fill=GOLD)
    img.alpha_composite(date_pill, (x1 + ep_w, top_y))

def draw_person_id(img):
    """Person ID strip — full width at bottom above safe zone.
    person-id: left=0, right=0, bottom=16.7%, bg=rgba(0,0,0,0.92→transparent)
    """
    d = ImageDraw.Draw(img)
    bottom_safe = int(0.167 * H)  # 214px
    pad_v = int(3 * CQW)           # 22px
    pad_l = int(4 * CQW)           # 29px
    pad_r = int(0.12 * W)          # 86px
    circle_d = int(10 * CQW)       # 72px
    bar_w = max(3, int(0.5 * CQW)) # 4px
    bar_h = int(9 * CQW)           # 65px

    # Row height
    name_bb = f_cinzel_name.getbbox('SAMI SATOSHI')
    title_bb = f_inter_title.getbbox('HOST · PCG TECHNOLOGY BRIEFING')
    row_h = name_bb[3] + int(1.2 * CQW) + title_bb[3] + 2 * pad_v
    strip_h = max(circle_d + 2 * pad_v, row_h)

    y_top = H - bottom_safe - strip_h

    # Background strip: gradient from rgba(0,0,0,0.92) to transparent
    strip = Image.new('RGBA', (W, strip_h), TRANSP)
    for x in range(W):
        t = min(1.0, x / (W * 0.7))
        a = int(234 * (1.0 - t))
        strip.paste((0, 0, 0, a), (x, 0, x+1, strip_h))
    img.alpha_composite(strip, (0, y_top))

    # Gold border top line
    d.line([(0, y_top), (W, y_top)], fill=(212, 185, 106, 115), width=1)

    # Circle (pid-circle) at left
    cx = pad_l + circle_d // 2
    cy = y_top + strip_h // 2
    # Draw circle background + gold ring
    circle_img = Image.new('RGBA', (circle_d, circle_d), TRANSP)
    cd = ImageDraw.Draw(circle_img)
    # Dark fill
    cd.ellipse([(0,0),(circle_d-1,circle_d-1)], fill=(0,0,0,178))
    # Gold ring
    cd.ellipse([(0,0),(circle_d-1,circle_d-1)], outline=(212,185,106,230), width=2)
    # PCG logo inside circle (60% of circle)
    logo_inner = int(circle_d * 0.6)
    logo = load_logo(logo_inner)
    lx = (circle_d - logo_inner) // 2
    circle_img.alpha_composite(logo, (lx, lx))
    img.alpha_composite(circle_img, (pad_l, y_top + (strip_h - circle_d)//2))

    # Gold vertical bar
    bx = pad_l + circle_d + int(3 * CQW)
    by = y_top + (strip_h - bar_h) // 2
    bar = Image.new('RGBA', (bar_w, bar_h), TRANSP)
    bd = ImageDraw.Draw(bar)
    for i in range(bar_h):
        t = i / bar_h
        r = int(212 + (122 - 212) * t)
        g = int(185 + ( 98 - 185) * t)
        b_ = int(106 + ( 40 - 106) * t)
        bd.line([(0, i), (bar_w, i)], fill=(r, g, b_, 255))
    img.alpha_composite(bar, (bx, by))

    # Text
    tx = bx + bar_w + int(3 * CQW)
    ty = y_top + pad_v
    d.text((tx, ty), 'SAMI SATOSHI', font=f_cinzel_name, fill=WHITE)
    ty2 = ty + name_bb[3] + int(1.2 * CQW)
    d.text((tx, ty2), 'HOST · PCG TECHNOLOGY BRIEFING', font=f_inter_title, fill=GOLD)

def draw_broll_ctx(img, tag, line1, line2=''):
    """B-Roll context panel — right of PiP position.
    broll-ctx: bottom=18%, left=31cqw=223px, right=12%=86px
    """
    d = ImageDraw.Draw(img)
    bottom_pct = int(0.18 * H)   # 230px from bottom
    pad_v = int(2 * CQW)          # 14px
    pad_h = int(2.5 * CQW)        # 18px
    bar_w = max(2, int(0.35 * CQW))  # 3px
    left_x = int(31 * CQW)        # 223px
    right_x = W - int(0.12 * W)   # 634px

    tag_h = f_inter_tag.getbbox(tag)[3]
    l1_h  = f_inter_ctx.getbbox(line1)[3]
    l2_h  = f_inter_ctx.getbbox(line2)[3] if line2 else 0
    gap   = int(1.2 * CQW)

    ctx_h = pad_v + tag_h + gap + l1_h + (gap + l2_h if line2 else 0) + pad_v
    ctx_w = right_x - left_x

    y_top = H - bottom_pct - ctx_h

    # Background
    ctx_bg = Image.new('RGBA', (ctx_w, ctx_h), BLACK_74)
    # Gold left border
    cdb = ImageDraw.Draw(ctx_bg)
    cdb.rectangle([(0, 0), (bar_w-1, ctx_h-1)], fill=GOLD)
    img.alpha_composite(ctx_bg, (left_x, y_top))

    # Text
    tx = left_x + bar_w + pad_h
    ty = y_top + pad_v
    d.text((tx, ty), tag.upper(), font=f_inter_tag, fill=GOLD)
    ty += tag_h + gap
    d.text((tx, ty), line1, font=f_inter_ctx, fill=OFF_WHITE)
    if line2:
        ty += l1_h + gap
        d.text((tx, ty), line2, font=f_inter_ctx, fill=OFF_WHITE)

# ─── Generate each overlay ───────────────────────────────────────────────

def make_bug_B():
    img = new_canvas()
    draw_pcg_bug(img)
    draw_ep_badge(img)
    img.save(f'{OUT}/bug_B.png')
    print(f'✓ bug_B.png [{img.mode}]')

def make_hostid_C():
    img = new_canvas()
    draw_pcg_bug(img)
    draw_ep_badge(img)
    draw_person_id(img)
    img.save(f'{OUT}/hostid_C.png')
    print(f'✓ hostid_C.png [{img.mode}]')

def make_broll(tag, line1, line2, filename):
    img = new_canvas()
    draw_pcg_bug(img)
    draw_ep_badge(img)
    draw_broll_ctx(img, tag, line1, line2)
    img.save(f'{OUT}/{filename}')
    print(f'✓ {filename} [{img.mode}]')

make_bug_B()
make_hostid_C()
# signoff_H is same as hostid_C
import shutil
shutil.copy(f'{OUT}/hostid_C.png', f'{OUT}/signoff_H.png')
print(f'✓ signoff_H.png [copy of hostid_C]')

make_broll('Artificial Intelligence',
           'Next-Gen Inference Chips',
           '3× Compute Density',
           'broll_ai_E.png')

make_broll('Blockchain',
           'Tokenized Treasury Settlement',
           'Top 10 Asset Managers · Live Capital',
           'broll_crypto_F.png')

make_broll('Power Club Global',
           'Sovereign Infrastructure',
           'Agents · Tokenized Incentive Design',
           'broll_pcg_G.png')

# Verify all have RGBA
print()
from PIL import Image as _I
import numpy as np
for f in ['bug_B.png','hostid_C.png','broll_ai_E.png','broll_crypto_F.png','broll_pcg_G.png','signoff_H.png']:
    img = _I.open(f'{OUT}/{f}')
    arr = np.array(img)
    t = (arr[:,:,3] == 0).sum()
    total = arr.shape[0]*arr.shape[1]
    print(f'  {f}: {img.mode}, {100*t/total:.1f}% transparent')

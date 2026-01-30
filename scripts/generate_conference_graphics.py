#!/usr/bin/env python3
"""
Generate Conference Graphics
Composes thumbnails for conference articles using speaker headshots and sponsor logos.
"""

import sqlite3
import os
import sys
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont, ImageFilter
from typing import List, Tuple, Optional
import uuid
from datetime import datetime

# Project root
PROJECT_ROOT = Path(__file__).parent.parent
DB_PATH = PROJECT_ROOT / "dev_assets" / "db.sqlite"
OUTPUT_DIR = PROJECT_ROOT / "dev_assets" / "generated_graphics"
HEADSHOTS_DIR = PROJECT_ROOT / "dev_assets" / "speaker_headshots"
LOGOS_DIR = PROJECT_ROOT / "dev_assets" / "sponsor_logos"


def create_gradient_background(width: int, height: int, colors: List[str]) -> Image.Image:
    """Create a diagonal gradient background."""
    img = Image.new('RGB', (width, height))
    draw = ImageDraw.Draw(img)

    # Parse hex colors
    def hex_to_rgb(hex_color: str) -> Tuple[int, int, int]:
        hex_color = hex_color.lstrip('#')
        return tuple(int(hex_color[i:i+2], 16) for i in (0, 2, 4))

    c1 = hex_to_rgb(colors[0])
    c2 = hex_to_rgb(colors[-1])

    # Create diagonal gradient
    for y in range(height):
        for x in range(width):
            # Diagonal position (0 to 1)
            pos = (x / width + y / height) / 2
            r = int(c1[0] * (1 - pos) + c2[0] * pos)
            g = int(c1[1] * (1 - pos) + c2[1] * pos)
            b = int(c1[2] * (1 - pos) + c2[2] * pos)
            draw.point((x, y), fill=(r, g, b))

    return img


def create_circular_mask(size: int) -> Image.Image:
    """Create a circular mask for headshots."""
    mask = Image.new('L', (size, size), 0)
    draw = ImageDraw.Draw(mask)
    draw.ellipse((0, 0, size-1, size-1), fill=255)
    return mask


def load_and_resize_image(path: str, size: Tuple[int, int]) -> Optional[Image.Image]:
    """Load and resize an image, handling various formats."""
    try:
        if not os.path.exists(path):
            # Try relative to project root
            full_path = PROJECT_ROOT / path
            if not full_path.exists():
                print(f"  Image not found: {path}")
                return None
            path = str(full_path)

        img = Image.open(path)
        img = img.convert('RGBA')
        img = img.resize(size, Image.Resampling.LANCZOS)
        return img
    except Exception as e:
        print(f"  Error loading {path}: {e}")
        return None


def draw_text_with_shadow(draw: ImageDraw.Draw, position: Tuple[int, int],
                          text: str, font: ImageFont.FreeTypeFont,
                          fill: str = "#ffffff", shadow: bool = True):
    """Draw text with optional drop shadow."""
    x, y = position

    if shadow:
        # Draw shadow
        shadow_offset = 2
        draw.text((x + shadow_offset, y + shadow_offset), text,
                  font=font, fill="#000000", anchor="mm")

    # Draw main text
    draw.text((x, y), text, font=font, fill=fill, anchor="mm")


def get_font(size: int, bold: bool = False) -> ImageFont.FreeTypeFont:
    """Get a font, falling back to default if custom fonts not available."""
    try:
        # Try common system fonts
        font_paths = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf" if bold else "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf" if bold else "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
        ]
        for path in font_paths:
            if os.path.exists(path):
                return ImageFont.truetype(path, size)
        return ImageFont.load_default()
    except:
        return ImageFont.load_default()


def compose_speakers_thumbnail(conference_name: str, speakers: List[dict],
                               output_path: Path) -> bool:
    """
    Compose a speakers article thumbnail.
    - Gradient background
    - Row of circular speaker headshots
    - Title and conference name text
    """
    width, height = 1792, 1024

    print(f"\n  Creating speakers thumbnail...")
    print(f"  Available speakers with photos: {len([s for s in speakers if s['photo_url']])}")

    # Create gradient background
    colors = ["#1a1a2e", "#16213e", "#0f3460"]
    img = create_gradient_background(width, height, colors)
    draw = ImageDraw.Draw(img)

    # Add some visual interest - subtle radial gradient overlay
    overlay = Image.new('RGBA', (width, height), (0, 0, 0, 0))
    overlay_draw = ImageDraw.Draw(overlay)
    center_x, center_y = width // 2, height // 2
    for r in range(max(width, height), 0, -10):
        alpha = int(30 * (1 - r / max(width, height)))
        overlay_draw.ellipse(
            (center_x - r, center_y - r, center_x + r, center_y + r),
            fill=(100, 100, 200, alpha)
        )
    img = Image.alpha_composite(img.convert('RGBA'), overlay).convert('RGB')
    draw = ImageDraw.Draw(img)

    # Title text
    title_font = get_font(72, bold=True)
    subtitle_font = get_font(36)

    title = "Meet the Speakers"
    draw_text_with_shadow(draw, (width // 2, 180), title, title_font)
    draw_text_with_shadow(draw, (width // 2, 260), conference_name, subtitle_font, fill="#cccccc")

    # Speaker headshots - circular, in a row
    speakers_with_photos = [s for s in speakers if s['photo_url']][:5]  # Max 5
    if speakers_with_photos:
        photo_size = 180
        spacing = 40
        total_width = len(speakers_with_photos) * photo_size + (len(speakers_with_photos) - 1) * spacing
        start_x = (width - total_width) // 2
        y_pos = 450

        for idx, speaker in enumerate(speakers_with_photos):
            x_pos = start_x + idx * (photo_size + spacing)

            # Load and process headshot
            photo = load_and_resize_image(speaker['photo_url'], (photo_size, photo_size))
            if photo:
                # Apply circular mask
                mask = create_circular_mask(photo_size)

                # Create circular photo with white border
                circular_photo = Image.new('RGBA', (photo_size + 8, photo_size + 8), (255, 255, 255, 255))
                photo_with_mask = Image.new('RGBA', (photo_size, photo_size), (0, 0, 0, 0))
                photo_with_mask.paste(photo, mask=mask)
                circular_photo.paste(photo_with_mask, (4, 4), photo_with_mask)

                # Add drop shadow
                shadow = Image.new('RGBA', (photo_size + 20, photo_size + 20), (0, 0, 0, 0))
                shadow_mask = create_circular_mask(photo_size + 8)
                shadow.paste((0, 0, 0, 100), (6, 6), shadow_mask)
                shadow = shadow.filter(ImageFilter.GaussianBlur(5))

                img.paste(shadow, (x_pos - 6, y_pos - 6), shadow)
                img.paste(circular_photo, (x_pos, y_pos), circular_photo)

                # Speaker name below photo
                name_font = get_font(18)
                name = speaker['name'].split()[0] if ' ' in speaker['name'] else speaker['name']  # First name only
                draw_text_with_shadow(draw, (x_pos + photo_size // 2, y_pos + photo_size + 25),
                                     name, name_font, shadow=False)

        # "And X more speakers" text
        total_speakers = len(speakers)
        if total_speakers > 5:
            more_text = f"+ {total_speakers - 5} more industry leaders"
            more_font = get_font(28)
            draw_text_with_shadow(draw, (width // 2, y_pos + photo_size + 80),
                                 more_text, more_font, fill="#90cdf4", shadow=False)

    # PCG branding
    brand_font = get_font(20)
    draw.text((width - 100, height - 40), "PCG Media", font=brand_font, fill="#666666", anchor="mm")

    # Save
    img.save(output_path, "PNG", quality=95)
    print(f"  Saved: {output_path}")
    return True


def compose_sponsors_thumbnail(conference_name: str, sponsors: List[dict],
                               event_count: int, output_path: Path) -> bool:
    """
    Compose a side events / sponsors thumbnail.
    - Vibrant gradient background
    - Grid of sponsor logos
    - Event count
    """
    width, height = 1792, 1024

    print(f"\n  Creating sponsors/side events thumbnail...")
    print(f"  Available sponsors with logos: {len([s for s in sponsors if s['photo_url']])}")

    # Vibrant gradient for events
    colors = ["#2d1b4e", "#4a1a6b", "#6b2d8a"]
    img = create_gradient_background(width, height, colors)
    draw = ImageDraw.Draw(img)

    # Title
    title_font = get_font(64, bold=True)
    subtitle_font = get_font(32)
    count_font = get_font(120, bold=True)

    draw_text_with_shadow(draw, (width // 2, 120), "Side Events Guide", title_font)
    draw_text_with_shadow(draw, (width // 2, 180), conference_name, subtitle_font, fill="#cccccc")

    # Big event count
    draw_text_with_shadow(draw, (width // 2, 350), str(event_count), count_font, fill="#f0abfc")
    draw_text_with_shadow(draw, (width // 2, 440), "Networking Events", get_font(36), fill="#e9d5ff")

    # Sponsor logos grid
    sponsors_with_logos = [s for s in sponsors if s['photo_url']][:6]  # Max 6
    if sponsors_with_logos:
        logo_size = 100
        cols = 3
        rows = 2
        spacing = 30
        grid_width = cols * logo_size + (cols - 1) * spacing
        grid_height = rows * logo_size + (rows - 1) * spacing
        start_x = (width - grid_width) // 2
        start_y = 550

        for idx, sponsor in enumerate(sponsors_with_logos):
            row = idx // cols
            col = idx % cols
            x_pos = start_x + col * (logo_size + spacing)
            y_pos = start_y + row * (logo_size + spacing)

            logo = load_and_resize_image(sponsor['photo_url'], (logo_size, logo_size))
            if logo:
                # Add white background for logos
                logo_bg = Image.new('RGBA', (logo_size + 10, logo_size + 10), (255, 255, 255, 240))
                logo_bg.paste(logo, (5, 5), logo if logo.mode == 'RGBA' else None)
                img.paste(logo_bg, (x_pos, y_pos), logo_bg)

    # "Presented by" text
    presented_font = get_font(24)
    draw.text((width // 2, 520), "Presented by leading sponsors", font=presented_font,
              fill="#a78bfa", anchor="mm")

    # PCG branding
    brand_font = get_font(20)
    draw.text((width - 100, height - 40), "PCG Media", font=brand_font, fill="#666666", anchor="mm")

    # Save
    img.save(output_path, "PNG", quality=95)
    print(f"  Saved: {output_path}")
    return True


def compose_press_release_thumbnail(conference_name: str, title: str,
                                    output_path: Path) -> bool:
    """
    Compose a press release thumbnail.
    - Professional blue gradient
    - Conference branding prominent
    - Clean typography
    """
    width, height = 1792, 1024

    print(f"\n  Creating press release thumbnail...")

    # Professional blue gradient
    colors = ["#0f172a", "#1e3a5f", "#1e40af"]
    img = create_gradient_background(width, height, colors)
    draw = ImageDraw.Draw(img)

    # Add subtle pattern overlay
    for i in range(0, width, 50):
        draw.line([(i, 0), (i + height, height)], fill=(255, 255, 255, 5), width=1)

    # "PRESS RELEASE" badge
    badge_font = get_font(28, bold=True)
    badge_text = "PRESS RELEASE"
    draw.rounded_rectangle(
        [(width // 2 - 120, 150), (width // 2 + 120, 195)],
        radius=5,
        fill="#3b82f6"
    )
    draw.text((width // 2, 172), badge_text, font=badge_font, fill="#ffffff", anchor="mm")

    # Main title
    title_font = get_font(56, bold=True)
    # Word wrap title if too long
    words = title.split()
    lines = []
    current_line = []
    for word in words:
        current_line.append(word)
        test_line = ' '.join(current_line)
        if len(test_line) > 40:
            if len(current_line) > 1:
                current_line.pop()
                lines.append(' '.join(current_line))
                current_line = [word]
            else:
                lines.append(test_line)
                current_line = []
    if current_line:
        lines.append(' '.join(current_line))

    y_offset = 300
    for line in lines[:3]:  # Max 3 lines
        draw_text_with_shadow(draw, (width // 2, y_offset), line, title_font)
        y_offset += 70

    # Conference name
    conf_font = get_font(40)
    draw_text_with_shadow(draw, (width // 2, y_offset + 50), conference_name, conf_font, fill="#93c5fd")

    # Date
    date_font = get_font(28)
    date_text = datetime.now().strftime("%B %Y")
    draw.text((width // 2, height - 100), date_text, font=date_font, fill="#64748b", anchor="mm")

    # PCG branding
    brand_font = get_font(20)
    draw.text((width - 100, height - 40), "PCG Media", font=brand_font, fill="#475569", anchor="mm")

    # Save
    img.save(output_path, "PNG", quality=95)
    print(f"  Saved: {output_path}")
    return True


def main():
    """Main entry point."""
    print("=" * 60)
    print("Conference Graphics Generator")
    print("=" * 60)

    # Ensure output directory exists
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    # Connect to database
    if not DB_PATH.exists():
        print(f"ERROR: Database not found at {DB_PATH}")
        sys.exit(1)

    conn = sqlite3.connect(str(DB_PATH))
    conn.row_factory = sqlite3.Row
    c = conn.cursor()

    # Get the most recent conference workflow
    c.execute("""
        SELECT id, conference_name, speakers_count, sponsors_count, side_events_count
        FROM conference_workflows
        ORDER BY created_at DESC
        LIMIT 1
    """)
    workflow = c.fetchone()

    if not workflow:
        print("ERROR: No conference workflow found in database")
        sys.exit(1)

    conference_name = workflow['conference_name']
    workflow_id = workflow['id'].hex() if isinstance(workflow['id'], bytes) else workflow['id']

    print(f"\nConference: {conference_name}")
    print(f"Workflow ID: {workflow_id}")
    print(f"Speakers: {workflow['speakers_count']}, Sponsors: {workflow['sponsors_count']}, Side Events: {workflow['side_events_count']}")

    # Get speakers with photos
    c.execute("""
        SELECT canonical_name, photo_url, company, title
        FROM entities
        WHERE entity_type = 'speaker' AND photo_url IS NOT NULL AND photo_url != ''
        ORDER BY RANDOM()
        LIMIT 20
    """)
    speakers = [{'name': r['canonical_name'], 'photo_url': r['photo_url'],
                 'company': r['company'], 'title': r['title']} for r in c.fetchall()]

    # Get sponsors with logos
    c.execute("""
        SELECT canonical_name, photo_url
        FROM entities
        WHERE entity_type = 'sponsor' AND photo_url IS NOT NULL AND photo_url != ''
        ORDER BY RANDOM()
        LIMIT 10
    """)
    sponsors = [{'name': r['canonical_name'], 'photo_url': r['photo_url']} for r in c.fetchall()]

    print(f"\nLoaded {len(speakers)} speakers with photos, {len(sponsors)} sponsors with logos")

    # Generate timestamp for filenames
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    conf_slug = conference_name.lower().replace(' ', '_')[:30]

    # Generate thumbnails
    print("\n" + "-" * 40)
    print("Generating thumbnails...")

    # 1. Speakers thumbnail
    speakers_path = OUTPUT_DIR / f"{conf_slug}_speakers_{timestamp}.png"
    compose_speakers_thumbnail(conference_name, speakers, speakers_path)

    # 2. Side events / sponsors thumbnail
    side_events_path = OUTPUT_DIR / f"{conf_slug}_side_events_{timestamp}.png"
    event_count = workflow['side_events_count'] or 22
    compose_sponsors_thumbnail(conference_name, sponsors, event_count, side_events_path)

    # 3. Press release thumbnail
    press_path = OUTPUT_DIR / f"{conf_slug}_press_release_{timestamp}.png"
    press_title = f"{conference_name} Announces Record Attendance"
    compose_press_release_thumbnail(conference_name, press_title, press_path)

    print("\n" + "=" * 60)
    print("COMPLETE!")
    print(f"Generated graphics saved to: {OUTPUT_DIR}")
    print("=" * 60)

    conn.close()


if __name__ == "__main__":
    main()

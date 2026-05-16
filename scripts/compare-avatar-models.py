#!/usr/bin/env python3
"""
A/B comparison of image-gen models for the avatar engine.

Runs the same 3 identity-sheet prompts (front, 3/4 left, 3/4 right) against
multiple models, using the same reference image + bible-grounded prompts that
the production pipeline used. Saves each model's output under
dev_assets/avatars/<id>/comparison/<model>/<slot>.png so you can flip through
all results in Finder side-by-side.

Requires: FAL_API_KEY in env. Reads the existing avatar's portrait_set + bible
from the sqlite db so prompts are identical across models.
"""
from __future__ import annotations

import asyncio
import base64
import json
import os
import sqlite3
import sys
import time
from pathlib import Path

import aiohttp

REPO = Path(__file__).resolve().parent.parent
DB_PATH = REPO / "dev_assets" / "db.sqlite"
AVATAR_DIR = REPO / "dev_assets" / "avatars"

FAL_KEY = os.environ.get("FAL_API_KEY", "").strip()
if not FAL_KEY:
    sys.exit("FAL_API_KEY not set in env")

MODELS = [
    {
        "name": "flux-pro-ultra-redux",
        "endpoint": "https://fal.run/fal-ai/flux-pro/v1.1-ultra/redux",
        "build": lambda ref_url, prompt: {
            "image_url": ref_url,
            "prompt": prompt,
            "image_prompt_strength": 0.35,
            "aspect_ratio": "1:1",
            "raw": True,
            "num_images": 1,
            "enable_safety_checker": False,
            "output_format": "png",
        },
    },
    {
        "name": "nano-banana-edit",
        "endpoint": "https://fal.run/fal-ai/nano-banana/edit",
        "build": lambda ref_url, prompt: {
            "image_urls": [ref_url],
            "prompt": prompt,
            "num_images": 1,
            "output_format": "png",
        },
    },
    {
        "name": "flux-kontext-pro",
        "endpoint": "https://fal.run/fal-ai/flux-pro/kontext",
        "build": lambda ref_url, prompt: {
            "image_url": ref_url,
            "prompt": prompt,
            "num_images": 1,
            "guidance_scale": 3.5,
            "output_format": "png",
            "safety_tolerance": "6",
        },
    },
    {
        "name": "seedream-v3-edit",
        "endpoint": "https://fal.run/fal-ai/bytedance/seedream/v3/edit",
        "build": lambda ref_url, prompt: {
            "image_url": ref_url,
            "prompt": prompt,
            "num_images": 1,
            "guidance_scale": 5.0,
        },
    },
]


def load_first_three_shots() -> tuple[str, list[dict]]:
    """Returns (avatar_id, shots[]) where shots include the original prompts."""
    con = sqlite3.connect(DB_PATH)
    cur = con.cursor()
    cur.execute(
        "SELECT lower(hex(id)), portrait_set FROM avatar_profiles WHERE portrait_set != '[]' LIMIT 1"
    )
    row = cur.fetchone()
    if not row:
        sys.exit("no avatar with portrait_set found in db")
    avatar_hex, portrait_json = row
    avatar_uuid = (
        f"{avatar_hex[0:8]}-{avatar_hex[8:12]}-{avatar_hex[12:16]}-"
        f"{avatar_hex[16:20]}-{avatar_hex[20:32]}"
    )
    shots = json.loads(portrait_json)
    return avatar_uuid, [s for s in shots if s["slot"] in ("front", "three_quarter_left", "three_quarter_right")]


async def call_model(
    session: aiohttp.ClientSession,
    model: dict,
    slot: str,
    ref_url: str,
    prompt: str,
    out_dir: Path,
) -> tuple[str, str, bool, str]:
    payload = model["build"](ref_url, prompt)
    headers = {
        "Authorization": f"Key {FAL_KEY}",
        "Content-Type": "application/json",
    }
    started = time.time()
    try:
        async with session.post(
            model["endpoint"],
            json=payload,
            headers=headers,
            timeout=aiohttp.ClientTimeout(total=180),
        ) as r:
            text_preview = ""
            if r.status != 200:
                body = await r.text()
                return (
                    model["name"],
                    slot,
                    False,
                    f"HTTP {r.status}: {body[:300]}",
                )
            data = await r.json()
        # Try every known response shape
        url = None
        images = data.get("images") if isinstance(data, dict) else None
        if isinstance(images, list) and images:
            first = images[0]
            if isinstance(first, dict):
                url = first.get("url") or first.get("image_url")
            elif isinstance(first, str):
                url = first
        if not url and isinstance(data, dict):
            url = data.get("image", {}).get("url") if isinstance(data.get("image"), dict) else None
        if not url:
            return model["name"], slot, False, f"no image url in response: {json.dumps(data)[:300]}"

        async with session.get(url, timeout=aiohttp.ClientTimeout(total=120)) as imr:
            png = await imr.read()
        out_dir.mkdir(parents=True, exist_ok=True)
        (out_dir / f"{slot}.png").write_bytes(png)
        elapsed = time.time() - started
        return model["name"], slot, True, f"{len(png)//1024} KB in {elapsed:.1f}s"
    except asyncio.TimeoutError:
        return model["name"], slot, False, "timeout (180s)"
    except Exception as e:
        return model["name"], slot, False, f"{type(e).__name__}: {e}"


async def main() -> int:
    avatar_uuid, shots = load_first_three_shots()
    avatar_root = AVATAR_DIR / avatar_uuid
    reference = avatar_root / "reference.png"
    if not reference.exists():
        sys.exit(f"no reference at {reference}")

    print(f"avatar: {avatar_uuid}")
    print(f"reference: {reference.relative_to(REPO)}  ({reference.stat().st_size // 1024} KB)")
    ref_b64 = base64.b64encode(reference.read_bytes()).decode("ascii")
    ref_url = f"data:image/png;base64,{ref_b64}"
    print(f"ref data-uri: {len(ref_url)//1024} KB")
    print()

    comparison_root = avatar_root / "comparison"

    # Stage existing gpt-image-1 outputs for direct comparison.
    gpt_dir = comparison_root / "gpt-image-1"
    gpt_dir.mkdir(parents=True, exist_ok=True)
    for s in shots:
        src = avatar_root / "shots" / f"{s['slot']}.png"
        if src.exists():
            (gpt_dir / f"{s['slot']}.png").write_bytes(src.read_bytes())
    print(f"staged gpt-image-1 outputs → {gpt_dir.relative_to(REPO)}")
    print()

    tasks = []
    async with aiohttp.ClientSession() as session:
        for model in MODELS:
            out_dir = comparison_root / model["name"]
            for s in shots:
                tasks.append(
                    call_model(session, model, s["slot"], ref_url, s["prompt"], out_dir)
                )
        print(f"firing {len(tasks)} requests against {len(MODELS)} fal.ai models…")
        print()
        results = await asyncio.gather(*tasks)

    by_model: dict[str, list[tuple[str, bool, str]]] = {}
    for model_name, slot, ok, msg in results:
        by_model.setdefault(model_name, []).append((slot, ok, msg))

    print("┌─ Results ─────────────────────────────────────────────────────")
    for model_name, rows in by_model.items():
        passed = sum(1 for _, ok, _ in rows if ok)
        print(f"│ {model_name}: {passed}/{len(rows)}")
        for slot, ok, msg in rows:
            mark = "✓" if ok else "✗"
            print(f"│   {mark} {slot:30s}  {msg}")
    print("└───────────────────────────────────────────────────────────────")
    print()
    print(f"open {comparison_root.relative_to(REPO)} in Finder to compare.")
    return 0


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))

#!/usr/bin/env python3
"""Run all 16 avatar slots through fal-ai/flux-pro/kontext using the same
bible + prompts the production pipeline stored. Saves to
dev_assets/avatars/<id>/shots-kontext/<slot>.png.
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
DB = REPO / "dev_assets" / "db.sqlite"
FAL_KEY = os.environ.get("FAL_API_KEY", "").strip()
if not FAL_KEY:
    sys.exit("FAL_API_KEY not set")

# Slots in the same order as avatar_engine.rs
SLOTS = [
    "front", "three_quarter_left", "three_quarter_right",
    "profile_left", "profile_right", "full_body_front",
    "smile_warm", "serious_neutral",
    "studio_black", "editorial_white", "casual_outdoor", "evening_dressed",
    "laughing", "looking_down", "walking_three_quarter", "thinking_pose",
]

ENDPOINT = "https://fal.run/fal-ai/flux-pro/kontext"
CONCURRENCY = 4


def load_avatar() -> tuple[str, dict[str, str]]:
    con = sqlite3.connect(DB)
    row = con.execute(
        "SELECT lower(hex(id)), portrait_set FROM avatar_profiles WHERE portrait_set != '[]' LIMIT 1"
    ).fetchone()
    if not row:
        sys.exit("no avatar with portrait_set found")
    avatar_hex, portrait_json = row
    avatar_uuid = (
        f"{avatar_hex[0:8]}-{avatar_hex[8:12]}-{avatar_hex[12:16]}-"
        f"{avatar_hex[16:20]}-{avatar_hex[20:32]}"
    )
    shots = json.loads(portrait_json)
    prompts = {s["slot"]: s["prompt"] for s in shots}
    return avatar_uuid, prompts


async def call(
    session: aiohttp.ClientSession,
    sem: asyncio.Semaphore,
    slot: str,
    ref_url: str,
    prompt: str,
    out_dir: Path,
) -> tuple[str, bool, str]:
    async with sem:
        started = time.time()
        try:
            async with session.post(
                ENDPOINT,
                headers={"Authorization": f"Key {FAL_KEY}", "Content-Type": "application/json"},
                json={
                    "image_url": ref_url,
                    "prompt": prompt,
                    "num_images": 1,
                    "guidance_scale": 3.5,
                    "output_format": "png",
                    "safety_tolerance": "6",
                },
                timeout=aiohttp.ClientTimeout(total=240),
            ) as r:
                if r.status != 200:
                    body = await r.text()
                    return slot, False, f"HTTP {r.status}: {body[:200]}"
                data = await r.json()
            url = data["images"][0]["url"]
            async with session.get(url, timeout=aiohttp.ClientTimeout(total=120)) as ir:
                png = await ir.read()
            (out_dir / f"{slot}.png").write_bytes(png)
            return slot, True, f"{len(png)//1024} KB in {time.time()-started:.1f}s"
        except Exception as e:
            return slot, False, f"{type(e).__name__}: {e}"


async def main() -> int:
    avatar_uuid, prompts = load_avatar()
    avatar_root = REPO / "dev_assets" / "avatars" / avatar_uuid
    reference = avatar_root / "reference.png"
    out_dir = avatar_root / "shots-kontext"
    out_dir.mkdir(parents=True, exist_ok=True)

    ref_b64 = base64.b64encode(reference.read_bytes()).decode()
    ref_url = f"data:image/png;base64,{ref_b64}"

    print(f"avatar: {avatar_uuid}")
    print(f"endpoint: {ENDPOINT}")
    print(f"output: {out_dir.relative_to(REPO)}")
    print(f"running {len(SLOTS)} shots with concurrency={CONCURRENCY}…")
    print()

    sem = asyncio.Semaphore(CONCURRENCY)
    async with aiohttp.ClientSession() as session:
        tasks = [
            call(session, sem, slot, ref_url, prompts[slot], out_dir)
            for slot in SLOTS
            if slot in prompts
        ]
        results = await asyncio.gather(*tasks)

    passed = sum(1 for _, ok, _ in results if ok)
    print(f"┌─ Results ({passed}/{len(results)}) ───────────────")
    for slot, ok, msg in results:
        mark = "✓" if ok else "✗"
        print(f"│ {mark} {slot:30s}  {msg}")
    print("└─────────────────────────────────────────")
    print()
    print(f"open {out_dir.relative_to(REPO)} in Finder")
    return 0 if passed == len(results) else 1


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))

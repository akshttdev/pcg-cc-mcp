#!/usr/bin/env python3
"""
PCG Tech Briefing — Generic Build Pipeline v2
=============================================
Produces a fully-composited Sami Satoshi Tech Briefing video from any raw
HeyGen portrait MP4 (720×1280 native). Supports smart cut points from
ElevenLabs word timing data and segment-aware B-roll category overlays.

Usage:
    python3 build_techbrief_generic.py --raw <path_to_raw.mp4> --job-id <uuid>
        [--cuts '[12.4, 27.1, 45.8]']         # timestamps in seconds
        [--segments '[{"label":"AI News","broll_category":"ai","broll_duration":10}, ...]']

Output:
    dev_assets/video_gen/video/<job_id>_final.mp4

Edit structure (N cut points → N+1 Sami segments, N B-roll windows):
    [thumb 3s] [logo] [sami_s1] [broll_1] [sami_s2] [broll_2] ... [sami_sN+1] [pause 2s] [outro 3s]

B-Roll windows: dark-grid placeholder + category-matched context overlay + Sami PiP circle.
Overlays sourced from dev_assets/video_gen/overlays/ep01/ (shared brand identity library).
broll_category → overlay:  ai → 03_broll_ctx_ai.png
                           crypto → 04_broll_ctx_crypto.png
                           pcg → 05_broll_ctx_pcg.png  (default)
"""

import argparse
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

# ─── Arg parsing ──────────────────────────────────────────────────────────────
parser = argparse.ArgumentParser()
parser.add_argument("--raw",      required=True, help="Path to raw HeyGen MP4 (720x1280 portrait)")
parser.add_argument("--job-id",   required=True, help="Video job UUID (used for output naming)")
parser.add_argument("--cuts",     default=None,  help='JSON array of cut timestamps in seconds, e.g. [12.4, 27.1]')
parser.add_argument("--segments", default=None,  help='JSON array of segment objects with broll_category + broll_duration')
args = parser.parse_args()

RAW    = Path(args.raw).resolve()
JOB_ID = args.job_id

if not RAW.exists():
    print(f"ERROR: raw video not found: {RAW}", file=sys.stderr)
    sys.exit(1)

# Parse cuts
cut_points = []
if args.cuts:
    try:
        cut_points = [float(x) for x in json.loads(args.cuts)]
    except Exception as e:
        print(f"WARNING: could not parse --cuts: {e} — will use auto-split", file=sys.stderr)

# Parse segments
segments = []
if args.segments:
    try:
        segments = json.loads(args.segments)
    except Exception as e:
        print(f"WARNING: could not parse --segments: {e} — using defaults", file=sys.stderr)

# ─── Paths ────────────────────────────────────────────────────────────────────
REPO    = Path(__file__).resolve().parent.parent.parent
VDIR    = REPO / "dev_assets/video_gen/video"
# Overlays: prefer pipeline/video/overlays (tracked), fallback to dev_assets
_OVL_PIPELINE = Path(__file__).resolve().parent / "overlays/ep01"
_OVL_DEVASSETS = REPO / "dev_assets/video_gen/overlays/ep01"
OVL     = _OVL_PIPELINE if _OVL_PIPELINE.exists() else _OVL_DEVASSETS
OUT     = VDIR / f"{JOB_ID}_final.mp4"
TMP     = Path(f"/tmp/techbrief_{JOB_ID}")
TMP.mkdir(exist_ok=True)

# Overlay library
BUG    = OVL / "01_show_bug.png"
HOSTID = OVL / "02_host_id_sami.png"
B_AI   = OVL / "03_broll_ctx_ai.png"
B_CRY  = OVL / "04_broll_ctx_crypto.png"
B_PCG  = OVL / "05_broll_ctx_pcg.png"
THUMB  = OVL / "07_thumbnail.png"
OUTRO  = OVL / "08_outro.png"

CATEGORY_OVERLAY = {
    "ai":     B_AI,
    "crypto": B_CRY,
    "pcg":    B_PCG,
}

# Logo (pick logo_intro.mp4 or first matching file)
LOGO_SRC = VDIR / "logo_intro.mp4"
if not LOGO_SRC.exists():
    for candidate in sorted(VDIR.glob("*logo*")):
        LOGO_SRC = candidate
        break

# Fixed durations
D_THUMB  = 3.0
D_PAUSE  = 2.0
D_OUTRO  = 3.0

# ─── Helpers ──────────────────────────────────────────────────────────────────
def ff(*fargs, check=True):
    cmd = ["ffmpeg", "-y"] + [str(a) for a in fargs]
    r = subprocess.run(cmd, capture_output=True, text=True)
    if check and r.returncode != 0:
        print(f"\nFFMPEG ERROR: {' '.join(cmd[:8])}...")
        print(r.stderr[-1200:])
        sys.exit(1)
    return r

def probe(path):
    r = subprocess.run(
        ["ffprobe", "-v", "quiet", "-show_entries", "format=duration",
         "-of", "csv=p=0", str(path)],
        capture_output=True, text=True
    )
    s = r.stdout.strip()
    return round(float(s), 3) if s else 0.0

def banner(step, total, msg):
    print(f"\n=== [{step}/{total}] {msg} ===")

# Encode settings — portrait-native, no scale squish needed (HeyGen outputs 720x1280)
CUT = ["-c:v", "libx264", "-preset", "fast", "-crf", "17",
       "-vf", "scale=720:1280,setsar=1", "-r", "30", "-an"]

# ─── Step 1: Probe raw + plan segment cuts ────────────────────────────────────
banner(1, 9, "Probing raw video + planning segments")

total_dur = probe(RAW)
print(f"  Raw duration: {total_dur:.3f}s")

# Determine cut points
if not cut_points:
    # Fallback: auto 40%/80% split → 2 B-roll windows
    cut_points = [round(total_dur * 0.40, 3), round(total_dur * 0.80, 3)]
    print(f"  No cuts provided — using auto 40/80 split: {cut_points}")
else:
    # Clamp cuts to valid range
    cut_points = sorted([c for c in cut_points if 0 < c < total_dur])
    print(f"  Using {len(cut_points)} cut point(s): {cut_points}")

N_BROLL   = len(cut_points)
N_SAMI    = N_BROLL + 1

# Sami segment boundaries: [0..cut[0]], [cut[0]..cut[1]], ..., [cut[-1]..end]
sami_bounds = list(zip([0.0] + cut_points, cut_points + [total_dur]))
print(f"  Sami segments: {N_SAMI}  B-roll windows: {N_BROLL}")
for i, (s, e) in enumerate(sami_bounds):
    print(f"    s{i+1}: {s:.3f} → {e:.3f}s ({e-s:.1f}s)")

# B-roll durations and categories from segments metadata
# segments[0] = intro, segments[1..N_BROLL] map to each B-roll window
# B-roll i (0-indexed) represents the topic of the segment BEFORE the cut
broll_durations = []
broll_categories = []
for i in range(N_BROLL):
    # Segment metadata index: B-roll i follows sami segment i (0-indexed)
    seg_idx = i  # use the segment before this B-roll
    if seg_idx < len(segments):
        dur  = float(segments[seg_idx].get("broll_duration", 10.0))
        cat  = segments[seg_idx].get("broll_category", "pcg")
    else:
        dur  = 10.0
        cat  = "ai" if i == 0 else ("crypto" if i == 1 else "pcg")
    broll_durations.append(dur)
    broll_categories.append(cat)
    print(f"  B-roll {i+1}: {dur}s  category={cat}")

# PiP windows: last D_BROLL seconds of each Sami segment before the cut
pip_windows = []
for i, (ss, to) in enumerate(sami_bounds[:-1]):  # all but the last segment
    d_br = broll_durations[i]
    pip_start = max(ss, to - d_br)
    pip_windows.append((f"br{i+1}", pip_start, to))
    print(f"  PiP {i+1} window: {pip_start:.2f}–{to:.2f}s")

# ─── Step 2: Cut Sami segments ────────────────────────────────────────────────
banner(2, 9, "Cutting Sami segments")

for i, (ss, to) in enumerate(sami_bounds):
    name = f"s{i+1}"
    path = TMP / f"{name}.mp4"
    cmd_in = ["-i", RAW, "-ss", ss]
    if to < total_dur:
        cmd_in += ["-to", to]
    ff(*cmd_in, *CUT, path)
    print(f"  {name}: {probe(path):.3f}s")

# ─── Step 3: Logo ─────────────────────────────────────────────────────────────
banner(3, 9, "Logo")
d_logo = probe(LOGO_SRC)
shutil.copy(LOGO_SRC, TMP / "logo.mp4")
print(f"  Logo: {d_logo:.3f}s")

# ─── Step 4: Build timeline ───────────────────────────────────────────────────
banner(4, 9, "Building timeline")

# Probe Sami segment durations
d_sami = [probe(TMP / f"s{i+1}.mp4") for i in range(N_SAMI)]

t = {}
t["thumb_s"] = 0.0
t["thumb_e"] = round(D_THUMB, 3)
t["logo_s"]  = t["thumb_e"]
t["logo_e"]  = round(t["logo_s"] + d_logo, 3)

cursor = t["logo_e"]
for i in range(N_SAMI):
    sk = f"s{i+1}"
    t[f"{sk}_s"] = cursor
    cursor = round(cursor + d_sami[i], 3)
    t[f"{sk}_e"] = cursor
    if i < N_BROLL:
        brk = f"br{i+1}"
        t[f"{brk}_s"] = cursor
        cursor = round(cursor + broll_durations[i], 3)
        t[f"{brk}_e"] = cursor

t["pause_e"] = round(cursor + D_PAUSE, 3)
t["outro_e"] = round(t["pause_e"] + D_OUTRO, 3)

for k, v in t.items():
    print(f"  {k:12s}: {v:.3f}s")

# ─── Step 5: Audio track ──────────────────────────────────────────────────────
banner(5, 9, "Building audio track")

# Silence chunks
ff("-f", "lavfi", "-i", "anullsrc=r=44100:cl=stereo",
   "-t", D_THUMB, "-acodec", "pcm_s16le", TMP/"a_thumb.wav")
ff("-f", "lavfi", "-i", "anullsrc=r=44100:cl=stereo",
   "-t", d_logo, "-acodec", "pcm_s16le", TMP/"a_logo.wav")
ff("-f", "lavfi", "-i", "anullsrc=r=44100:cl=stereo",
   "-t", D_PAUSE + D_OUTRO, "-acodec", "pcm_s16le", TMP/"a_end.wav")
for i in range(N_BROLL):
    ff("-f", "lavfi", "-i", "anullsrc=r=44100:cl=stereo",
       "-t", broll_durations[i], "-acodec", "pcm_s16le", TMP/f"a_br{i+1}.wav")

# Sami audio — split at cut points
for i, (ss, to) in enumerate(sami_bounds):
    name = f"a_s{i+1}.wav"
    cmd = ["-i", RAW, "-ss", ss]
    if to < total_dur:
        cmd += ["-to", to]
    cmd += ["-vn", "-acodec", "pcm_s16le", TMP/name]
    ff(*cmd)

# Concat audio in timeline order: thumb | logo | s1 | br1 | s2 | br2 | ... | sN | end
audio_inputs = [TMP/"a_thumb.wav", TMP/"a_logo.wav"]
for i in range(N_SAMI):
    audio_inputs.append(TMP/f"a_s{i+1}.wav")
    if i < N_BROLL:
        audio_inputs.append(TMP/f"a_br{i+1}.wav")
audio_inputs.append(TMP/"a_end.wav")

n_audio = len(audio_inputs)
concat_filter = "".join(f"[{i}:a]" for i in range(n_audio))
concat_filter += f"concat=n={n_audio}:v=0:a=1[aout]"

ff_audio_cmd = []
for p in audio_inputs:
    ff_audio_cmd += ["-i", p]
ff_audio_cmd += [
    "-filter_complex", concat_filter,
    "-map", "[aout]", "-c:a", "aac", "-b:a", "192k", TMP/"audio.aac"
]
ff(*ff_audio_cmd)
print("  Audio track ready")

# ─── Step 6: B-roll placeholder backgrounds ───────────────────────────────────
banner(6, 9, "Building B-roll placeholder backgrounds")

for i in range(N_BROLL):
    name = f"br{i+1}"
    ff("-f", "lavfi",
       "-i", "color=c=0x0a0e18:size=720x1280:rate=30",
       "-vf", "drawgrid=x=0:y=0:width=72:height=72:color=0xAF9041@0.04:t=1",
       "-t", broll_durations[i], "-c:v", "libx264", "-preset", "fast", "-crf", "20", "-an",
       TMP/f"broll_{name}.mp4")
    print(f"  {name} placeholder ({broll_categories[i]}): {probe(TMP/f'broll_{name}.mp4'):.3f}s")

# ─── Step 7: Thumbnail + cards ────────────────────────────────────────────────
banner(7, 9, "Building thumbnail + cards")

ff("-i", RAW, "-ss", 1.0, "-vframes", 1,
   "-vf", "scale=720:1280,setsar=1", TMP/"sami_frame.png")

ff("-loop", 1, "-i", TMP/"sami_frame.png",
   "-loop", 1, "-i", THUMB,
   "-filter_complex",
   "[0:v]scale=720:1280,setsar=1[bg];"
   "[1:v]format=rgba,scale=720:1280[ovl];"
   "[bg][ovl]overlay=x=0:y=0[vout]",
   "-map", "[vout]", "-t", D_THUMB,
   "-c:v", "libx264", "-preset", "fast", "-crf", "17", "-r", "30", "-an",
   TMP/"thumb.mp4")
print(f"  Thumbnail: {probe(TMP/'thumb.mp4'):.3f}s")

ff("-loop", 1, "-i", OUTRO, "-t", D_OUTRO,
   "-vf", "scale=720:1280,setsar=1", "-r", "30",
   "-c:v", "libx264", "-preset", "fast", "-crf", "17", "-an",
   TMP/"outro.mp4")
print(f"  Outro: {probe(TMP/'outro.mp4'):.3f}s")

ff("-f", "lavfi", "-i", "color=c=0x080b10:size=720x1280:rate=30",
   "-t", D_PAUSE, "-c:v", "libx264", "-preset", "fast", "-crf", "20", "-an",
   TMP/"pause.mp4")

# ─── Step 8: PiP circles ──────────────────────────────────────────────────────
banner(8, 9, "Building PiP circles")

pip_geq = (
    "crop=520:520:100:80,scale=220:220,format=rgba,"
    "geq="
    "r='if(lte(hypot(X-110\\,Y-110)\\,108)\\,r(X\\,Y)\\,0)':"
    "g='if(lte(hypot(X-110\\,Y-110)\\,108)\\,g(X\\,Y)\\,0)':"
    "b='if(lte(hypot(X-110\\,Y-110)\\,108)\\,b(X\\,Y)\\,0)':"
    "a='if(lte(hypot(X-110\\,Y-110)\\,108)\\,255\\,0)'"
)

procs = []
for (name, ss, to) in pip_windows:
    p = subprocess.Popen(
        ["ffmpeg", "-y", "-i", str(RAW), "-ss", str(ss), "-to", str(to),
         "-vf", pip_geq,
         "-c:v", "libx264", "-preset", "fast", "-crf", "15",
         str(TMP/f"pip_{name}.mp4")],
        stderr=subprocess.PIPE, stdout=subprocess.PIPE
    )
    procs.append((name, p))

for name, p in procs:
    p.wait()
    if p.returncode != 0:
        print(f"  WARNING: pip_{name} failed — B-roll will have no PiP")
    else:
        print(f"  pip_{name}: {probe(TMP/f'pip_{name}.mp4'):.3f}s")

# ─── Step 9: Base concat + overlay pass ───────────────────────────────────────
banner(9, 9, "Final composite assembly")

# Timeline concat order
concat_order = [TMP/"thumb.mp4", TMP/"logo.mp4"]
for i in range(N_SAMI):
    concat_order.append(TMP/f"s{i+1}.mp4")
    if i < N_BROLL:
        concat_order.append(TMP/f"broll_br{i+1}.mp4")
concat_order += [TMP/"pause.mp4", TMP/"outro.mp4"]

concat_txt = TMP / "concat.txt"
concat_txt.write_text("\n".join(f"file '{p}'" for p in concat_order))

ff("-f", "concat", "-safe", "0", "-i", concat_txt,
   "-c:v", "libx264", "-preset", "fast", "-crf", "17", "-an",
   TMP/"composite.mp4")
COMP = probe(TMP/"composite.mp4")
print(f"  Composite: {COMP:.3f}s")

# Mux audio
ff("-i", TMP/"composite.mp4", "-i", TMP/"audio.aac",
   "-map", "0:v", "-map", "1:a",
   "-c:v", "copy", "-c:a", "copy", "-t", COMP,
   TMP/"muxed.mp4")
print(f"  Muxed: {probe(TMP/'muxed.mp4'):.3f}s")

FO = round(COMP - 1.5, 2)

# Build enable expressions
sami_enable = "+".join(
    f"between(t,{t[f's{i+1}_s']},{t[f's{i+1}_e']})" for i in range(N_SAMI)
)
bug_on = f"gt(t,{t['logo_e']})"

# Build overlay filter dynamically
filter_lines = []
base_stream  = "[0:v]"
stream_idx   = 1  # next input index

# PiP circles
pip_inputs = []
for i in range(N_BROLL):
    name = f"br{i+1}"
    pip_path = TMP / f"pip_{name}.mp4"
    if pip_path.exists() and probe(pip_path) > 0:
        br_s = t[f"br{i+1}_s"]
        filter_lines.append(
            f"[{stream_idx}:v]setpts=PTS-STARTPTS+{br_s}/TB[pip_{name}_t]"
        )
        pip_inputs.append((name, stream_idx))
        stream_idx += 1

# Overlay PiP onto base
last = base_stream
for j, (name, _) in enumerate(pip_inputs):
    out = f"[va{j+1}]"
    filter_lines.append(
        f"{last}[pip_{name}_t]overlay=x=30:y=740:eof_action=pass:shortest=0{out}"
    )
    last = out
last_after_pip = last

# Bug overlay
bug_idx = stream_idx
filter_lines.append(f"[{bug_idx}:v]format=rgba,scale=720:1280[bug]")
filter_lines.append(f"{last_after_pip}[bug]overlay=x=0:y=0:enable='{bug_on}'[vb]")
stream_idx += 1

# Host ID
hid_idx = stream_idx
filter_lines.append(f"[{hid_idx}:v]format=rgba,scale=720:1280[hostid]")
filter_lines.append(f"[vb][hostid]overlay=x=0:y=0:enable='{sami_enable}'[vc]")
stream_idx += 1

# B-roll context overlays — one per window, category-matched
prev = "[vc]"
for i in range(N_BROLL):
    cat      = broll_categories[i]
    ovl_path = CATEGORY_OVERLAY.get(cat, B_PCG)
    label    = f"bctx{i+1}"
    out      = f"[v{label}]"
    br_s     = t[f"br{i+1}_s"]
    br_e     = t[f"br{i+1}_e"]
    filter_lines.append(f"[{stream_idx}:v]format=rgba,scale=720:1280[{label}]")
    filter_lines.append(
        f"{prev}[{label}]overlay=x=0:y=0:enable='between(t,{br_s},{br_e})'{out}"
    )
    prev = out
    stream_idx += 1

# Fade in/out
filter_lines.append(
    f"{prev}fade=t=in:st=0:d=0.5,fade=t=out:st={FO}:d=1.5[vout]"
)

filter_file = TMP / "filter.txt"
filter_file.write_text(";\n".join(filter_lines) + "\n")

print(f"\n  Timeline: {COMP:.1f}s total | fade-out at {FO}s")
for i in range(N_BROLL):
    print(f"  B-roll {i+1} ({broll_categories[i]:6s}): {t[f'br{i+1}_s']:.1f}–{t[f'br{i+1}_e']:.1f}s")

# Build input list dynamically
ff_inputs = ["-i", str(TMP/"muxed.mp4")]
for (name, _) in pip_inputs:
    ff_inputs += ["-i", str(TMP/f"pip_{name}.mp4")]
ff_inputs += ["-loop", "1", "-i", str(BUG)]
ff_inputs += ["-loop", "1", "-i", str(HOSTID)]
for i in range(N_BROLL):
    cat      = broll_categories[i]
    ovl_path = CATEGORY_OVERLAY.get(cat, B_PCG)
    ff_inputs += ["-loop", "1", "-i", str(ovl_path)]

print("\n  Running final overlay pass...")
r = subprocess.run([
    "ffmpeg", "-y",
    *ff_inputs,
    "-filter_complex_script", str(filter_file),
    "-map",  "[vout]",
    "-map",  "0:a",
    "-af",   f"afade=t=in:st=0:d=0.5,afade=t=out:st={FO}:d=1.5",
    "-t",    str(COMP),
    "-c:v",  "libx264", "-preset", "fast", "-crf", "17",
    "-c:a",  "aac", "-b:a", "192k",
    str(OUT),
], capture_output=True, text=True)

if r.returncode != 0:
    print("FFMPEG ERROR:")
    print(r.stderr[-1200:])
    sys.exit(1)

final_dur  = probe(OUT)
final_size = os.path.getsize(OUT) / 1_000_000
print(f"\n{'='*50}")
print(f"DONE")
print(f"Output:   {OUT}")
print(f"Duration: {final_dur:.1f}s")
print(f"Size:     {final_size:.1f} MB")
print(f"Segments: {N_SAMI} Sami + {N_BROLL} B-roll windows")
print(f"B-roll:   {', '.join(broll_categories)}")

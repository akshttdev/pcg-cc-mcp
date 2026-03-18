#!/usr/bin/env python3
"""
Receive .env file over APN NATS relay.
Uses request-reply: receiver signals READY, sender then transmits.
Cipher: ChaCha20-Poly1305 with HKDF-SHA256 — matches apn_env_send.py on spaceterminal.
"""

import asyncio, base64, hashlib, json, os, sys

try:
    import nats
except ImportError:
    print("Missing: pip install nats-py"); sys.exit(1)

try:
    from cryptography.hazmat.primitives.ciphers.aead import ChaCha20Poly1305
    from cryptography.hazmat.primitives.kdf.hkdf import HKDF
    from cryptography.hazmat.primitives import hashes
except ImportError:
    print("Missing: pip install cryptography"); sys.exit(1)

NATS_URL        = "nats://nonlocal.info:4222"
READY_SUBJECT   = "apn.config.env.ready.apn_814d37f4"
RECV_SUBJECT    = "apn.config.env.apn_814d37f4"
PASSPHRASE      = "pcg2pythia"
EXPECTED_SHA256 = "b8d98f29625500c7a506d849e76481de926210e64c7253e29cf09fd64cde199d"
OUT_PATH        = "/home/pythia/pcg-cc-mcp/.env"


def derive_key(passphrase: str, salt: bytes = None) -> bytes:
    return HKDF(
        algorithm=hashes.SHA256(), length=32,
        salt=salt, info=b"apn-env-transfer",
    ).derive(passphrase.encode())


def try_decrypt(data: bytes) -> str | None:
    attempts = [
        # JSON: {"nonce": b64, "ciphertext": b64, "salt": b64}
        lambda d: (lambda e: ChaCha20Poly1305(derive_key(PASSPHRASE, base64.b64decode(e["salt"]) if e.get("salt") else None))
                   .decrypt(base64.b64decode(e["nonce"]), base64.b64decode(e.get("ciphertext") or e.get("encrypted_payload")), None)
                   .decode())(json.loads(d.decode())),
        # binary: salt(32) + nonce(12) + ciphertext
        lambda d: ChaCha20Poly1305(derive_key(PASSPHRASE, d[:32])).decrypt(d[32:44], d[44:], None).decode() if len(d) > 44 else None,
        # binary: nonce(12) + ciphertext
        lambda d: ChaCha20Poly1305(derive_key(PASSPHRASE, None)).decrypt(d[:12], d[12:], None).decode() if len(d) > 12 else None,
    ]
    for fn in attempts:
        try:
            result = fn(data)
            if result:
                return result
        except Exception:
            pass
    return None


async def receive():
    received = asyncio.Event()
    result   = {}

    nc = await nats.connect(NATS_URL)

    async def handler(msg):
        print(f"  Incoming on {msg.subject} ({len(msg.data)} bytes)")
        with open("/tmp/apn_env_raw.bin", "wb") as f:
            f.write(msg.data)

        plaintext = try_decrypt(msg.data)
        if plaintext:
            result["env"] = plaintext
            received.set()
        else:
            print("  Could not decrypt — raw saved to /tmp/apn_env_raw.bin")

    await nc.subscribe(RECV_SUBJECT, cb=handler)
    print(f"Subscribed to {RECV_SUBJECT}")

    # Signal ready so sender knows to transmit
    await nc.publish(READY_SUBJECT, b"READY")
    print(f"Signalled READY on {READY_SUBJECT}")
    print("Waiting for payload...")

    try:
        await asyncio.wait_for(received.wait(), timeout=180)
    except asyncio.TimeoutError:
        print("Timed out after 3 minutes."); await nc.drain(); sys.exit(1)

    await nc.drain()

    sha256 = hashlib.sha256(result["env"].encode()).hexdigest()
    if sha256 != EXPECTED_SHA256:
        print(f"WARNING: SHA-256 mismatch\n  got:      {sha256}\n  expected: {EXPECTED_SHA256}")
    else:
        print(f"SHA-256 verified.")

    with open(OUT_PATH, "w") as f:
        f.write(result["env"])

    print(f"Written {len(result['env'].splitlines())} lines to {OUT_PATH}")


if __name__ == "__main__":
    asyncio.run(receive())

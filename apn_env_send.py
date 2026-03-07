#!/usr/bin/env python3
"""
Send .env file over APN NATS relay — encrypted with a passphrase.
Run apn_env_receive.py on pythia FIRST, then run this on the sender machine.

Usage:
    python3 apn_env_send.py /path/to/.env
"""

import asyncio
import base64
import getpass
import hashlib
import json
import sys

try:
    import nats
except ImportError:
    print("Missing dependency: pip install nats-py")
    sys.exit(1)

try:
    from cryptography.fernet import Fernet
    from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC
    from cryptography.hazmat.primitives import hashes
except ImportError:
    print("Missing dependency: pip install cryptography")
    sys.exit(1)

NATS_URL = "nats://nonlocal.info:4222"
SUBJECT  = "apn.env.transfer.pythia-master-814d37f4"
SALT     = b"pcg-cc-mcp-env-salt-2026"   # must match receiver


def derive_key(passphrase: str) -> bytes:
    kdf = PBKDF2HMAC(
        algorithm=hashes.SHA256(),
        length=32,
        salt=SALT,
        iterations=390_000,
    )
    return base64.urlsafe_b64encode(kdf.derive(passphrase.encode()))


async def send(env_path: str):
    with open(env_path, "r") as fh:
        plaintext = fh.read()

    lines = len(plaintext.splitlines())
    sha256 = hashlib.sha256(plaintext.encode()).hexdigest()

    passphrase = getpass.getpass("Passphrase (share this with receiver out-of-band): ")
    confirm    = getpass.getpass("Confirm passphrase: ")
    if passphrase != confirm:
        print("Passphrases do not match.")
        sys.exit(1)

    key       = derive_key(passphrase)
    f         = Fernet(key)
    ciphertext = f.encrypt(plaintext.encode())

    envelope = json.dumps({
        "payload": base64.urlsafe_b64encode(ciphertext).decode(),
        "sha256":  sha256,
        "lines":   lines,
    }).encode()

    nc = await nats.connect(NATS_URL)
    await nc.publish(SUBJECT, envelope)
    await nc.flush()
    await nc.drain()

    print(f"Sent {lines} lines ({len(plaintext)} bytes) encrypted to {SUBJECT}")
    print("Done.")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(f"Usage: python3 {sys.argv[0]} /path/to/.env")
        sys.exit(1)

    asyncio.run(send(sys.argv[1]))

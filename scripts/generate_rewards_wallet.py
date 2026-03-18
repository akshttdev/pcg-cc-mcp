#!/usr/bin/env python3
"""
Generate a new Aptos wallet for network reward distribution.
This wallet will be funded with 11% of testnet supply (110M VIBE).
"""

import hashlib
import secrets
from mnemonic import Mnemonic
from nacl.signing import SigningKey
from nacl.encoding import HexEncoder

def generate_aptos_wallet():
    """Generate a new Aptos wallet with mnemonic phrase."""

    # Generate 12-word mnemonic
    mnemo = Mnemonic("english")
    mnemonic_phrase = mnemo.generate(strength=128)

    # Derive seed from mnemonic
    seed = mnemo.to_seed(mnemonic_phrase)

    # Use first 32 bytes as private key
    private_key_bytes = seed[:32]

    # Create signing key
    signing_key = SigningKey(private_key_bytes)
    verify_key = signing_key.verify_key

    # Derive Aptos address (SHA3-256 hash of public key + 0x00)
    public_key_bytes = bytes(verify_key)
    hasher = hashlib.sha3_256()
    hasher.update(public_key_bytes + b'\x00')  # 0x00 for single signature
    address = "0x" + hasher.hexdigest()

    return {
        "mnemonic": mnemonic_phrase,
        "private_key": signing_key.encode(encoder=HexEncoder).decode(),
        "public_key": verify_key.encode(encoder=HexEncoder).decode(),
        "address": address
    }

if __name__ == "__main__":
    print("🔐 Generating Network Rewards Wallet")
    print("=" * 60)

    wallet = generate_aptos_wallet()

    print("\n✅ Network Rewards Wallet Generated!")
    print("\n" + "=" * 60)
    print("WALLET DETAILS - SAVE THESE SECURELY!")
    print("=" * 60)
    print(f"\n📍 Aptos Address:")
    print(f"   {wallet['address']}")
    print(f"\n🔑 Mnemonic (12 words):")
    print(f"   {wallet['mnemonic']}")
    print(f"\n🔐 Private Key (hex):")
    print(f"   {wallet['private_key']}")
    print(f"\n📢 Public Key (hex):")
    print(f"   {wallet['public_key']}")
    print("\n" + "=" * 60)
    print("REWARD WALLET ALLOCATION")
    print("=" * 60)
    print(f"Total Supply:      1,000,000,000 VIBE")
    print(f"Reward Allocation: 11% (110,000,000 VIBE)")
    print(f"USD Value (@$0.01): $1,100,000")
    print("\n⚠️  CRITICAL: Save the mnemonic phrase offline!")
    print("⚠️  This wallet will distribute rewards to all peer nodes.")
    print("=" * 60)

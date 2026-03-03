-- Prepare for Veritwin Bridge stablecoin integration
-- payment_method: 'aptos_vibe' | 'usdc' | 'usdt' | 'veritwin_bridge'
ALTER TABLE vibe_deposits ADD COLUMN payment_method TEXT NOT NULL DEFAULT 'aptos_vibe';
ALTER TABLE vibe_withdrawals ADD COLUMN payment_method TEXT NOT NULL DEFAULT 'aptos_vibe';

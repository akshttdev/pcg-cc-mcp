-- Seed: Share Resonance's default board from Powerclub Global → Sirak Studios as Joint Venture

INSERT OR IGNORE INTO board_shares (id, board_id, source_organization_id, target_organization_id, permission, share_type, shared_by, is_active)
SELECT
    randomblob(16),
    pb.id,
    pcg.id,
    ss.id,
    'editor',
    'joint_venture',
    u.id,
    1
FROM project_boards pb
JOIN projects p ON p.id = pb.project_id
JOIN organizations pcg ON pcg.slug = 'powerclub-global'
JOIN organizations ss ON ss.slug = 'sirak-studios'
JOIN users u ON u.is_admin = 1
WHERE LOWER(p.name) = 'resonance'
  AND pb.board_type = 'default'
LIMIT 1;

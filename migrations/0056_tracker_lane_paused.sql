ALTER TABLE tracker_states ADD COLUMN lane_paused INTEGER NOT NULL DEFAULT 0;

INSERT INTO tracker_states (id, team_id, name, type, color, position, lane_status, description, lane_paused)
SELECT lower(hex(randomblob(16))), id, 'Paused', 'started', '#9b8afb', 2.5, NULL, NULL, 1
FROM tracker_teams WHERE key = 'AGT';

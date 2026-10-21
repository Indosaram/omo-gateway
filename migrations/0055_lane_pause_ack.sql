-- acked_at: the lane runner recorded that no turn of the lane is running under this pause.
-- seen_working_at: the lane produced output after the ack (reported to the lead once per pause).
ALTER TABLE lane_pauses ADD COLUMN acked_at DATETIME;
ALTER TABLE lane_pauses ADD COLUMN seen_working_at DATETIME;

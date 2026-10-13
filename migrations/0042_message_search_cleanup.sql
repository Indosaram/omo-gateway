-- Search documents follow their transcript rows out of the index.
-- Session deletes cascade to messages after the parent is gone, so the session
-- trigger purges by the child message ids before the cascade runs.
CREATE TRIGGER IF NOT EXISTS message_search_session_bd
BEFORE DELETE ON sessions
BEGIN
    DELETE FROM message_search_documents
    WHERE platform = old.platform
      AND channel_id IN (old.channel_id, COALESCE(old.thread_id, old.channel_id))
      AND message_id IN (
          SELECT platform_message_id FROM messages
          WHERE session_key = old.session_key
            AND platform_message_id IS NOT NULL
            AND trim(platform_message_id) <> ''
      );
END;

-- Message deletes while the session still exists remove only transcript-derived documents.
CREATE TRIGGER IF NOT EXISTS message_search_transcript_ad
AFTER DELETE ON messages
WHEN old.platform_message_id IS NOT NULL AND trim(old.platform_message_id) <> ''
BEGIN
    -- Anchored on the primary-key prefix (platform, channel_id, message_id) so each
    -- deleted message is an index lookup, not a scan of every indexed document.
    DELETE FROM message_search_documents
    WHERE platform = (SELECT s.platform FROM sessions s WHERE s.session_key = old.session_key)
      AND channel_id IN (
          (SELECT s.channel_id FROM sessions s WHERE s.session_key = old.session_key),
          (SELECT COALESCE(s.thread_id, s.channel_id) FROM sessions s WHERE s.session_key = old.session_key)
      )
      AND message_id = old.platform_message_id
      AND json_extract(metadata_json, '$.source') = 'transcript';
END;

-- One-time purge of transcript documents orphaned by earlier session deletes.
DELETE FROM message_search_documents
WHERE json_extract(metadata_json, '$.source') = 'transcript'
  AND NOT EXISTS (
      SELECT 1 FROM messages m
      JOIN sessions s ON s.session_key = m.session_key
      WHERE m.platform_message_id = message_search_documents.message_id
        AND s.platform = message_search_documents.platform
        AND message_search_documents.channel_id IN (s.channel_id, COALESCE(s.thread_id, s.channel_id))
  );

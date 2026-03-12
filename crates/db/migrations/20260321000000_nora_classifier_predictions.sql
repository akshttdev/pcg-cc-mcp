CREATE TABLE IF NOT EXISTS nora_classifier_predictions (
    id                       TEXT PRIMARY KEY,
    meeting_session_id       TEXT NOT NULL,
    segment_index            INTEGER NOT NULL,
    speaker_label            TEXT NOT NULL,
    utterance                TEXT NOT NULL,
    context_json             TEXT NOT NULL,
    predicted_speak          INTEGER NOT NULL,
    confidence               TEXT NOT NULL,
    reasoning                TEXT,
    was_wake_word_addressed  INTEGER,
    created_at               TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

CREATE INDEX IF NOT EXISTS idx_ncp_session  ON nora_classifier_predictions(meeting_session_id);
CREATE INDEX IF NOT EXISTS idx_ncp_accuracy ON nora_classifier_predictions(predicted_speak, was_wake_word_addressed);

-- Attendee names from the calendar event a note was started from (JSON array of strings)
ALTER TABLE meetings ADD COLUMN attendees TEXT;

-- Text as heard before vocabulary correction, so every correction can be reverted
ALTER TABLE transcripts ADD COLUMN transcript_original TEXT;

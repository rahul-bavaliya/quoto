-- 0004: no rate history.
-- Provinces change their rates rarely, and a quote keeps a copy of the rates it used,
-- so the extra log table is not worth keeping. (0003 stays as it was: a migration that
-- has been run is never edited; this new file undoes it.)

DROP TRIGGER provinces_log_rate_change ON provinces;
DROP FUNCTION log_province_rate_change();
DROP TABLE province_rate_history;

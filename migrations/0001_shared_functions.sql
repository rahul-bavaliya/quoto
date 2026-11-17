-- 0001: small helpers that later tables reuse.
-- Rule: an applied migration is never edited. A fix is a new file.

-- Stamps `updated_at` every time a row is updated.
CREATE FUNCTION set_updated_at() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

-- Refuses a change. Attached to tables whose rows must never be deleted
-- (or, for a log, never changed). The error is a "restrict violation" (23001).
CREATE FUNCTION refuse_change() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'this change is not allowed on %', TG_TABLE_NAME
        USING ERRCODE = 'restrict_violation';
END;
$$;

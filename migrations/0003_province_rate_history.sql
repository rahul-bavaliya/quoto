-- 0003: a log of every change to a province's rates.
-- The database fills it by itself (a trigger on `provinces`); the app never writes to it.
-- It can't be changed or deleted. Each row holds the OLD values, as they were.

CREATE TABLE province_rate_history (
    id                   uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    province_code        text        NOT NULL REFERENCES provinces (code),
    gst_rate_thousandths integer,
    pst_rate_thousandths integer,
    hst_rate_thousandths integer,
    effective_from       date        NOT NULL,
    note                 text        NOT NULL,
    changed_at           timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX province_rate_history_by_province
    ON province_rate_history (province_code, changed_at);

CREATE FUNCTION log_province_rate_change() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO province_rate_history
        (province_code, gst_rate_thousandths, pst_rate_thousandths, hst_rate_thousandths,
         effective_from, note)
    VALUES
        (OLD.code, OLD.gst_rate_thousandths, OLD.pst_rate_thousandths, OLD.hst_rate_thousandths,
         OLD.effective_from, OLD.note);
    RETURN NEW;
END;
$$;

-- Only a change to the rates or their start date is logged (fixing a name is not).
CREATE TRIGGER provinces_log_rate_change
    AFTER UPDATE ON provinces
    FOR EACH ROW
    WHEN (OLD.gst_rate_thousandths IS DISTINCT FROM NEW.gst_rate_thousandths
          OR OLD.pst_rate_thousandths IS DISTINCT FROM NEW.pst_rate_thousandths
          OR OLD.hst_rate_thousandths IS DISTINCT FROM NEW.hst_rate_thousandths
          OR OLD.effective_from IS DISTINCT FROM NEW.effective_from)
    EXECUTE FUNCTION log_province_rate_change();

-- The log is append-only.
CREATE TRIGGER province_rate_history_refuse_change
    BEFORE UPDATE OR DELETE ON province_rate_history
    FOR EACH ROW EXECUTE FUNCTION refuse_change();
CREATE TRIGGER province_rate_history_refuse_truncate
    BEFORE TRUNCATE ON province_rate_history
    FOR EACH STATEMENT EXECUTE FUNCTION refuse_change();

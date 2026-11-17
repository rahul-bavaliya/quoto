-- 0002: one row per province or territory, with the taxes charged there.
-- A province can't exist without a tax system: either a GST (with a PST or none),
-- or an HST alone. "No tax" is an empty column, never a 0% rate.
-- Rates are whole thousandths of a percent: 5% = 5000, 9.975% = 9975 (no floats).
-- When the government changes a rate, update the row; the log table (0003) keeps the old values.

CREATE TABLE provinces (
    id                   uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    code                 text        NOT NULL UNIQUE CHECK (code ~ '^[A-Z]{2}$'),
    name                 text        NOT NULL UNIQUE CHECK (btrim(name) <> ''),
    country_code         text        NOT NULL CHECK (country_code ~ '^[A-Z]{2}$'),
    gst_rate_thousandths integer     CHECK (gst_rate_thousandths BETWEEN 1 AND 100000),
    pst_rate_thousandths integer     CHECK (pst_rate_thousandths BETWEEN 1 AND 100000),
    hst_rate_thousandths integer     CHECK (hst_rate_thousandths BETWEEN 1 AND 100000),
    -- The day these rates took effect.
    effective_from       date        NOT NULL,
    -- Where the numbers came from, or why they changed.
    note                 text        NOT NULL CHECK (btrim(note) <> ''),
    created_at           timestamptz NOT NULL DEFAULT now(),
    updated_at           timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT provinces_tax_system CHECK (
        (gst_rate_thousandths IS NOT NULL AND hst_rate_thousandths IS NULL)
        OR (gst_rate_thousandths IS NULL AND hst_rate_thousandths IS NOT NULL
            AND pst_rate_thousandths IS NULL)
    )
);

CREATE TRIGGER provinces_set_updated_at
    BEFORE UPDATE ON provinces
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- A province is never deleted.
CREATE TRIGGER provinces_refuse_delete
    BEFORE DELETE ON provinces
    FOR EACH ROW EXECUTE FUNCTION refuse_change();
CREATE TRIGGER provinces_refuse_truncate
    BEFORE TRUNCATE ON provinces
    FOR EACH STATEMENT EXECUTE FUNCTION refuse_change();

-- The seed: CRA "GST/HST rates by province", checked 2026-11-17.
-- https://www.canada.ca/en/revenue-agency/services/tax/businesses/topics/gst-hst-businesses/charge-collect-which-rate/calculator.html
-- 2025-04-01 means "in force since at least this date" (Nova Scotia's HST fell to 14% that day).
-- Quebec's QST and Manitoba's RST are stored as the PST.
INSERT INTO provinces
    (code, name, country_code, gst_rate_thousandths, pst_rate_thousandths, hst_rate_thousandths, effective_from, note)
VALUES
    ('AB', 'Alberta',                   'CA', 5000, NULL, NULL,  '2025-04-01', 'CRA GST/HST rates by province, checked 2026-11-17'),
    ('BC', 'British Columbia',          'CA', 5000, 7000, NULL,  '2025-04-01', 'CRA GST/HST rates by province, checked 2026-11-17'),
    ('MB', 'Manitoba',                  'CA', 5000, 7000, NULL,  '2025-04-01', 'CRA GST/HST rates by province, checked 2026-11-17'),
    ('NB', 'New Brunswick',             'CA', NULL, NULL, 15000, '2025-04-01', 'CRA GST/HST rates by province, checked 2026-11-17'),
    ('NL', 'Newfoundland and Labrador', 'CA', NULL, NULL, 15000, '2025-04-01', 'CRA GST/HST rates by province, checked 2026-11-17'),
    ('NS', 'Nova Scotia',               'CA', NULL, NULL, 14000, '2025-04-01', 'CRA GST/HST rates by province, checked 2026-11-17'),
    ('NT', 'Northwest Territories',     'CA', 5000, NULL, NULL,  '2025-04-01', 'CRA GST/HST rates by province, checked 2026-11-17'),
    ('NU', 'Nunavut',                   'CA', 5000, NULL, NULL,  '2025-04-01', 'CRA GST/HST rates by province, checked 2026-11-17'),
    ('ON', 'Ontario',                   'CA', NULL, NULL, 13000, '2025-04-01', 'CRA GST/HST rates by province, checked 2026-11-17'),
    ('PE', 'Prince Edward Island',      'CA', NULL, NULL, 15000, '2025-04-01', 'CRA GST/HST rates by province, checked 2026-11-17'),
    ('QC', 'Quebec',                    'CA', 5000, 9975, NULL,  '2025-04-01', 'CRA GST/HST rates by province, checked 2026-11-17'),
    ('SK', 'Saskatchewan',              'CA', 5000, 6000, NULL,  '2025-04-01', 'CRA GST/HST rates by province, checked 2026-11-17'),
    ('YT', 'Yukon',                     'CA', 5000, NULL, NULL,  '2025-04-01', 'CRA GST/HST rates by province, checked 2026-11-17');

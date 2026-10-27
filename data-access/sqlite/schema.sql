PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS simulations (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    currency TEXT NOT NULL CHECK (currency GLOB '[A-Z][A-Z][A-Z]'),
    stop_conditions_configured INTEGER NOT NULL DEFAULT 0
        CHECK (stop_conditions_configured IN (0, 1))
) STRICT;

CREATE TABLE IF NOT EXISTS simulation_root_keys (
    simulation_id TEXT PRIMARY KEY NOT NULL
        REFERENCES simulations(id),
    root_key BLOB NOT NULL CHECK (length(root_key) = 32)
) STRICT;

CREATE TRIGGER IF NOT EXISTS simulation_root_keys_immutable
BEFORE UPDATE ON simulation_root_keys
BEGIN SELECT RAISE(ABORT, 'simulation root key is immutable'); END;

CREATE TABLE IF NOT EXISTS simulation_months (
    simulation_id TEXT NOT NULL REFERENCES simulations(id),
    month INTEGER NOT NULL CHECK (month BETWEEN 0 AND 65535),
    PRIMARY KEY (simulation_id, month)
) STRICT;

CREATE TABLE IF NOT EXISTS loan_settings (
    simulation_id TEXT PRIMARY KEY NOT NULL REFERENCES simulations(id),
    enabled INTEGER,
    annual_interest_rate TEXT,
    remaining_term_months_at_start INTEGER,
    CHECK (
        (enabled IS NULL AND annual_interest_rate IS NULL
            AND remaining_term_months_at_start IS NULL)
        OR (
            enabled IN (0, 1)
            AND annual_interest_rate IS NOT NULL
            AND remaining_term_months_at_start BETWEEN 1 AND 65535
        )
    )
) STRICT;

CREATE TABLE IF NOT EXISTS fixed_expense_settings (
    simulation_id TEXT PRIMARY KEY NOT NULL REFERENCES simulations(id),
    enabled INTEGER,
    method TEXT,
    monthly_amount_coefficient TEXT,
    monthly_amount_scale INTEGER,
    annual_property_value_rate TEXT,
    CHECK (
        (enabled IS NULL AND method IS NULL
            AND monthly_amount_coefficient IS NULL AND monthly_amount_scale IS NULL
            AND annual_property_value_rate IS NULL)
        OR (
            enabled IN (0, 1)
            AND (
                (method = 'monthly_amount'
                    AND monthly_amount_coefficient IS NOT NULL
                    AND monthly_amount_scale BETWEEN 0 AND 28
                    AND annual_property_value_rate IS NULL)
                OR
                (method = 'annual_property_value_rate'
                    AND monthly_amount_coefficient IS NULL
                    AND monthly_amount_scale IS NULL
                    AND annual_property_value_rate IS NOT NULL)
            )
        )
    )
) STRICT;

CREATE TABLE IF NOT EXISTS rental_settings (
    simulation_id TEXT PRIMARY KEY NOT NULL REFERENCES simulations(id),
    enabled INTEGER,
    monthly_rent_coefficient TEXT,
    monthly_rent_scale INTEGER,
    CHECK (
        (enabled IS NULL AND monthly_rent_coefficient IS NULL AND monthly_rent_scale IS NULL)
        OR (
            enabled IN (0, 1)
            AND monthly_rent_coefficient IS NOT NULL
            AND monthly_rent_scale BETWEEN 0 AND 28
        )
    )
) STRICT;

CREATE TABLE IF NOT EXISTS repair_occurrence_settings (
    simulation_id TEXT PRIMARY KEY NOT NULL REFERENCES simulations(id),
    enabled INTEGER CHECK (enabled IS NULL OR enabled IN (0, 1))
) STRICT;

CREATE TABLE IF NOT EXISTS repair_cost_settings (
    simulation_id TEXT PRIMARY KEY NOT NULL REFERENCES simulations(id),
    maximum_amount_coefficient TEXT,
    maximum_amount_scale INTEGER,
    delay_months INTEGER,
    CHECK (
        (maximum_amount_coefficient IS NULL
            AND maximum_amount_scale IS NULL AND delay_months IS NULL)
        OR (
            maximum_amount_coefficient IS NOT NULL
            AND maximum_amount_scale BETWEEN 0 AND 28
            AND delay_months BETWEEN 1 AND 65535
        )
    )
) STRICT;

CREATE TABLE IF NOT EXISTS stop_conditions (
    id INTEGER PRIMARY KEY,
    simulation_id TEXT NOT NULL REFERENCES simulations(id),
    condition TEXT NOT NULL CHECK (
        condition IN ('after_months', 'all_loans_paid_off')
    ),
    value INTEGER,
    UNIQUE (simulation_id, condition),
    CHECK (
        (condition = 'after_months' AND value BETWEEN 1 AND 65535)
        OR (condition = 'all_loans_paid_off' AND value IS NULL)
    )
) STRICT;

CREATE TABLE IF NOT EXISTS simulation_accounts (
    simulation_id TEXT NOT NULL REFERENCES simulations(id),
    account_code INTEGER NOT NULL CHECK (account_code > 0),
    PRIMARY KEY (simulation_id, account_code)
) STRICT;

CREATE TABLE IF NOT EXISTS month_events (
    simulation_id TEXT NOT NULL,
    month INTEGER NOT NULL,
    event_ordinal INTEGER NOT NULL CHECK (event_ordinal >= 0),
    event_kind TEXT NOT NULL CHECK (
        event_kind IN (
            'rent_collected',
            'repair_occurred',
            'repair_cost_settled',
            'loan_payment',
            'fixed_expense_paid'
        )
    ),
    PRIMARY KEY (simulation_id, month, event_ordinal),
    FOREIGN KEY (simulation_id, month)
        REFERENCES simulation_months(simulation_id, month),
    CHECK (month BETWEEN 1 AND 65535)
) STRICT;

CREATE TABLE IF NOT EXISTS repair_cost_instances (
    simulation_id TEXT NOT NULL,
    occurrence_month INTEGER NOT NULL CHECK (occurrence_month BETWEEN 1 AND 65535),
    due_month INTEGER NOT NULL CHECK (due_month BETWEEN 1 AND 65535),
    maximum_amount_coefficient TEXT NOT NULL,
    maximum_amount_scale INTEGER NOT NULL CHECK (maximum_amount_scale BETWEEN 0 AND 28),
    settled_month INTEGER,
    PRIMARY KEY (simulation_id, occurrence_month),
    FOREIGN KEY (simulation_id, occurrence_month)
        REFERENCES simulation_months(simulation_id, month),
    FOREIGN KEY (simulation_id, settled_month)
        REFERENCES simulation_months(simulation_id, month),
    CHECK (due_month > occurrence_month),
    CHECK (settled_month IS NULL OR settled_month = due_month)
) STRICT;

CREATE TABLE IF NOT EXISTS journal_entries (
    id INTEGER PRIMARY KEY,
    simulation_id TEXT NOT NULL,
    month INTEGER NOT NULL CHECK (month BETWEEN 0 AND 65535),
    sequence INTEGER NOT NULL CHECK (sequence >= 0),
    UNIQUE (simulation_id, sequence),
    UNIQUE (id, simulation_id),
    FOREIGN KEY (simulation_id)
        REFERENCES simulations(id),
    FOREIGN KEY (simulation_id, month)
        REFERENCES simulation_months(simulation_id, month)
) STRICT;

CREATE TABLE IF NOT EXISTS journal_lines (
    journal_entry_id INTEGER NOT NULL,
    simulation_id TEXT NOT NULL,
    line_number INTEGER NOT NULL CHECK (line_number >= 0),
    account_code INTEGER NOT NULL,
    side TEXT NOT NULL CHECK (side IN ('debit', 'credit')),
    amount_coefficient TEXT NOT NULL,
    amount_scale INTEGER NOT NULL CHECK (amount_scale BETWEEN 0 AND 28),
    PRIMARY KEY (journal_entry_id, line_number),
    FOREIGN KEY (journal_entry_id, simulation_id)
        REFERENCES journal_entries(id, simulation_id),
    FOREIGN KEY (simulation_id, account_code)
        REFERENCES simulation_accounts(simulation_id, account_code)
) STRICT;

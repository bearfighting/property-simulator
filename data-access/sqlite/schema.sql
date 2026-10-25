PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS simulations (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    currency TEXT NOT NULL CHECK (currency GLOB '[A-Z][A-Z][A-Z]')
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
    month INTEGER NOT NULL CHECK (month >= 0),
    PRIMARY KEY (simulation_id, month)
) STRICT;

CREATE TABLE IF NOT EXISTS loan_settings (
    simulation_id TEXT NOT NULL REFERENCES simulations(id),
    effective_from_month INTEGER NOT NULL CHECK (effective_from_month >= 1),
    enabled INTEGER,
    annual_interest_rate TEXT,
    remaining_term_months_at_start INTEGER,
    PRIMARY KEY (simulation_id, effective_from_month),
    CHECK (
        (enabled IS NULL AND annual_interest_rate IS NULL
            AND remaining_term_months_at_start IS NULL)
        OR (
            enabled IN (0, 1)
            AND annual_interest_rate IS NOT NULL
            AND remaining_term_months_at_start BETWEEN 1 AND 4294967295
        )
    )
) STRICT;

CREATE TABLE IF NOT EXISTS fixed_expense_settings (
    simulation_id TEXT NOT NULL REFERENCES simulations(id),
    effective_from_month INTEGER NOT NULL CHECK (effective_from_month >= 1),
    enabled INTEGER,
    method TEXT,
    monthly_amount_coefficient TEXT,
    monthly_amount_scale INTEGER,
    annual_property_value_rate TEXT,
    PRIMARY KEY (simulation_id, effective_from_month),
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
    simulation_id TEXT NOT NULL REFERENCES simulations(id),
    effective_from_month INTEGER NOT NULL CHECK (effective_from_month >= 1),
    enabled INTEGER,
    monthly_rent_coefficient TEXT,
    monthly_rent_scale INTEGER,
    PRIMARY KEY (simulation_id, effective_from_month),
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
    simulation_id TEXT NOT NULL REFERENCES simulations(id),
    effective_from_month INTEGER NOT NULL CHECK (effective_from_month >= 1),
    enabled INTEGER CHECK (enabled IS NULL OR enabled IN (0, 1)),
    PRIMARY KEY (simulation_id, effective_from_month)
) STRICT;

CREATE TABLE IF NOT EXISTS repair_cost_settings (
    simulation_id TEXT NOT NULL REFERENCES simulations(id),
    effective_from_month INTEGER NOT NULL CHECK (effective_from_month >= 1),
    maximum_amount_coefficient TEXT,
    maximum_amount_scale INTEGER,
    delay_months INTEGER,
    PRIMARY KEY (simulation_id, effective_from_month),
    CHECK (
        (maximum_amount_coefficient IS NULL
            AND maximum_amount_scale IS NULL AND delay_months IS NULL)
        OR (
            maximum_amount_coefficient IS NOT NULL
            AND maximum_amount_scale BETWEEN 0 AND 28
            AND delay_months BETWEEN 1 AND 4294967295
        )
    )
) STRICT;

CREATE TABLE IF NOT EXISTS stop_condition_sets (
    simulation_id TEXT NOT NULL REFERENCES simulations(id),
    effective_from_month INTEGER NOT NULL CHECK (effective_from_month >= 1),
    PRIMARY KEY (simulation_id, effective_from_month)
) STRICT;

CREATE TABLE IF NOT EXISTS stop_conditions (
    id INTEGER PRIMARY KEY,
    simulation_id TEXT NOT NULL,
    effective_from_month INTEGER NOT NULL,
    condition TEXT NOT NULL CHECK (
        condition IN ('after_months', 'all_loans_paid_off', 'user_stop')
    ),
    value INTEGER,
    FOREIGN KEY (simulation_id, effective_from_month)
        REFERENCES stop_condition_sets(simulation_id, effective_from_month),
    CHECK (
        (condition = 'after_months' AND value BETWEEN 1 AND 4294967295)
        OR (condition IN ('all_loans_paid_off', 'user_stop') AND value IS NULL)
    )
) STRICT;

-- Once a month is completed, settings that affected it become immutable.
-- New settings versions remain insertable for future Months.
CREATE TRIGGER IF NOT EXISTS loan_settings_no_retroactive_insert
BEFORE INSERT ON loan_settings
WHEN EXISTS (
    SELECT 1 FROM simulation_months
    WHERE simulation_id = NEW.simulation_id
      AND month >= 1 AND month >= NEW.effective_from_month
)
BEGIN SELECT RAISE(ABORT, 'cannot change settings used by a completed month'); END;
CREATE TRIGGER IF NOT EXISTS loan_settings_no_retroactive_update
BEFORE UPDATE ON loan_settings
WHEN EXISTS (
    SELECT 1 FROM simulation_months
    WHERE simulation_id = OLD.simulation_id
      AND month >= 1 AND month >= OLD.effective_from_month
)
BEGIN SELECT RAISE(ABORT, 'cannot change settings used by a completed month'); END;
CREATE TRIGGER IF NOT EXISTS loan_settings_no_retroactive_delete
BEFORE DELETE ON loan_settings
WHEN EXISTS (
    SELECT 1 FROM simulation_months
    WHERE simulation_id = OLD.simulation_id
      AND month >= 1 AND month >= OLD.effective_from_month
)
BEGIN SELECT RAISE(ABORT, 'cannot change settings used by a completed month'); END;

CREATE TRIGGER IF NOT EXISTS fixed_expense_settings_no_retroactive_insert
BEFORE INSERT ON fixed_expense_settings
WHEN EXISTS (
    SELECT 1 FROM simulation_months
    WHERE simulation_id = NEW.simulation_id
      AND month >= 1 AND month >= NEW.effective_from_month
)
BEGIN SELECT RAISE(ABORT, 'cannot change settings used by a completed month'); END;
CREATE TRIGGER IF NOT EXISTS fixed_expense_settings_no_retroactive_update
BEFORE UPDATE ON fixed_expense_settings
WHEN EXISTS (
    SELECT 1 FROM simulation_months
    WHERE simulation_id = OLD.simulation_id
      AND month >= 1 AND month >= OLD.effective_from_month
)
BEGIN SELECT RAISE(ABORT, 'cannot change settings used by a completed month'); END;
CREATE TRIGGER IF NOT EXISTS fixed_expense_settings_no_retroactive_delete
BEFORE DELETE ON fixed_expense_settings
WHEN EXISTS (
    SELECT 1 FROM simulation_months
    WHERE simulation_id = OLD.simulation_id
      AND month >= 1 AND month >= OLD.effective_from_month
)
BEGIN SELECT RAISE(ABORT, 'cannot change settings used by a completed month'); END;

CREATE TRIGGER IF NOT EXISTS rental_settings_no_retroactive_insert
BEFORE INSERT ON rental_settings
WHEN EXISTS (
    SELECT 1 FROM simulation_months
    WHERE simulation_id = NEW.simulation_id
      AND month >= 1 AND month >= NEW.effective_from_month
)
BEGIN SELECT RAISE(ABORT, 'cannot change settings used by a completed month'); END;
CREATE TRIGGER IF NOT EXISTS rental_settings_no_retroactive_update
BEFORE UPDATE ON rental_settings
WHEN EXISTS (
    SELECT 1 FROM simulation_months
    WHERE simulation_id = OLD.simulation_id
      AND month >= 1 AND month >= OLD.effective_from_month
)
BEGIN SELECT RAISE(ABORT, 'cannot change settings used by a completed month'); END;
CREATE TRIGGER IF NOT EXISTS rental_settings_no_retroactive_delete
BEFORE DELETE ON rental_settings
WHEN EXISTS (
    SELECT 1 FROM simulation_months
    WHERE simulation_id = OLD.simulation_id
      AND month >= 1 AND month >= OLD.effective_from_month
)
BEGIN SELECT RAISE(ABORT, 'cannot change settings used by a completed month'); END;

CREATE TRIGGER IF NOT EXISTS repair_occurrence_settings_no_retroactive_insert
BEFORE INSERT ON repair_occurrence_settings
WHEN EXISTS (
    SELECT 1 FROM simulation_months
    WHERE simulation_id = NEW.simulation_id
      AND month >= 1 AND month >= NEW.effective_from_month
)
BEGIN SELECT RAISE(ABORT, 'cannot change settings used by a completed month'); END;
CREATE TRIGGER IF NOT EXISTS repair_occurrence_settings_no_retroactive_update
BEFORE UPDATE ON repair_occurrence_settings
WHEN EXISTS (
    SELECT 1 FROM simulation_months
    WHERE simulation_id = OLD.simulation_id
      AND month >= 1 AND month >= OLD.effective_from_month
)
BEGIN SELECT RAISE(ABORT, 'cannot change settings used by a completed month'); END;
CREATE TRIGGER IF NOT EXISTS repair_occurrence_settings_no_retroactive_delete
BEFORE DELETE ON repair_occurrence_settings
WHEN EXISTS (
    SELECT 1 FROM simulation_months
    WHERE simulation_id = OLD.simulation_id
      AND month >= 1 AND month >= OLD.effective_from_month
)
BEGIN SELECT RAISE(ABORT, 'cannot change settings used by a completed month'); END;

CREATE TRIGGER IF NOT EXISTS repair_cost_settings_no_retroactive_insert
BEFORE INSERT ON repair_cost_settings
WHEN EXISTS (
    SELECT 1 FROM simulation_months
    WHERE simulation_id = NEW.simulation_id
      AND month >= 1 AND month >= NEW.effective_from_month
)
BEGIN SELECT RAISE(ABORT, 'cannot change settings used by a completed month'); END;
CREATE TRIGGER IF NOT EXISTS repair_cost_settings_no_retroactive_update
BEFORE UPDATE ON repair_cost_settings
WHEN EXISTS (
    SELECT 1 FROM simulation_months
    WHERE simulation_id = OLD.simulation_id
      AND month >= 1 AND month >= OLD.effective_from_month
)
BEGIN SELECT RAISE(ABORT, 'cannot change settings used by a completed month'); END;
CREATE TRIGGER IF NOT EXISTS repair_cost_settings_no_retroactive_delete
BEFORE DELETE ON repair_cost_settings
WHEN EXISTS (
    SELECT 1 FROM simulation_months
    WHERE simulation_id = OLD.simulation_id
      AND month >= 1 AND month >= OLD.effective_from_month
)
BEGIN SELECT RAISE(ABORT, 'cannot change settings used by a completed month'); END;

CREATE TRIGGER IF NOT EXISTS stop_condition_sets_no_retroactive_insert
BEFORE INSERT ON stop_condition_sets
WHEN EXISTS (
    SELECT 1 FROM simulation_months
    WHERE simulation_id = NEW.simulation_id
      AND month >= 1 AND month >= NEW.effective_from_month
)
BEGIN SELECT RAISE(ABORT, 'cannot change settings used by a completed month'); END;
CREATE TRIGGER IF NOT EXISTS stop_condition_sets_no_retroactive_update
BEFORE UPDATE ON stop_condition_sets
WHEN EXISTS (
    SELECT 1 FROM simulation_months
    WHERE simulation_id = OLD.simulation_id
      AND month >= 1 AND month >= OLD.effective_from_month
)
BEGIN SELECT RAISE(ABORT, 'cannot change settings used by a completed month'); END;
CREATE TRIGGER IF NOT EXISTS stop_condition_sets_no_retroactive_delete
BEFORE DELETE ON stop_condition_sets
WHEN EXISTS (
    SELECT 1 FROM simulation_months
    WHERE simulation_id = OLD.simulation_id
      AND month >= 1 AND month >= OLD.effective_from_month
)
BEGIN SELECT RAISE(ABORT, 'cannot change settings used by a completed month'); END;

CREATE TRIGGER IF NOT EXISTS stop_conditions_no_retroactive_insert
BEFORE INSERT ON stop_conditions
WHEN EXISTS (
    SELECT 1 FROM simulation_months
    WHERE simulation_id = NEW.simulation_id
      AND month >= 1 AND month >= NEW.effective_from_month
)
BEGIN SELECT RAISE(ABORT, 'cannot change settings used by a completed month'); END;
CREATE TRIGGER IF NOT EXISTS stop_conditions_no_retroactive_update
BEFORE UPDATE ON stop_conditions
WHEN EXISTS (
    SELECT 1 FROM simulation_months
    WHERE simulation_id = OLD.simulation_id
      AND month >= 1 AND month >= OLD.effective_from_month
)
BEGIN SELECT RAISE(ABORT, 'cannot change settings used by a completed month'); END;
CREATE TRIGGER IF NOT EXISTS stop_conditions_no_retroactive_delete
BEFORE DELETE ON stop_conditions
WHEN EXISTS (
    SELECT 1 FROM simulation_months
    WHERE simulation_id = OLD.simulation_id
      AND month >= 1 AND month >= OLD.effective_from_month
)
BEGIN SELECT RAISE(ABORT, 'cannot change settings used by a completed month'); END;

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
    CHECK (month >= 1)
) STRICT;

CREATE TABLE IF NOT EXISTS repair_cost_instances (
    simulation_id TEXT NOT NULL,
    occurrence_month INTEGER NOT NULL CHECK (occurrence_month >= 1),
    due_month INTEGER NOT NULL,
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
    month INTEGER NOT NULL CHECK (month >= 0),
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

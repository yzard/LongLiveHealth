CREATE TABLE IF NOT EXISTS lab_metric_aliases (
    user_id TEXT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    metric_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    PRIMARY KEY (user_id, metric_id)
);
CREATE TABLE IF NOT EXISTS lab_metric_loinc_links (
    user_id TEXT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    loinc_code TEXT NOT NULL,
    metric_id TEXT NOT NULL,
    PRIMARY KEY (user_id, loinc_code)
);

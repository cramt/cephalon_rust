CREATE TABLE relics (
    id TEXT NOT NULL UNIQUE,
    id_name TEXT NOT NULL,
    name TEXT NOT NULL,
    vaulted INT NOT NULL,
    era TEXT NOT NULL,
    trading_tax INT NOT NULL
)

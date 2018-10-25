-- Your SQL goes here

CREATE TABLE person (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE identity (
    identifier VARCHAR(128) PRIMARY KEY,
    person_id  INTEGER     NOT NULL REFERENCES person(id) ON DELETE CASCADE ON UPDATE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE login_session (
    id         CHAR(64)    NOT NULL PRIMARY KEY,
    person_id  INTEGER     NOT NULL REFERENCES person(id) ON DELETE CASCADE ON UPDATE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE identity IS 'Identifiers, such as cell phone number, that a person may use to log in.';
COMMENT ON TABLE login_session IS 'Record of unique, secret identifiers to identify a person from a session token.';

-- Let Diesel manage "updated_at" columns
SELECT diesel_manage_updated_at('person');
SELECT diesel_manage_updated_at('identity');
SELECT diesel_manage_updated_at('login_session');
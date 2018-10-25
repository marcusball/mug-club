-- Your SQL goes here

CREATE TABLE brewery (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    UNIQUE (name)
);

CREATE TABLE beer (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    brewery_id INTEGER NOT NULL REFERENCES brewery(id) ON DELETE CASCADE ON UPDATE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    UNIQUE (name)
);

CREATE INDEX ON beer (brewery_id);

CREATE TABLE drink (
    id SERIAL PRIMARY KEY,
    drank_on DATE NOT NULL,
    beer_id INTEGER NOT NULL REFERENCES beer(id) ON DELETE RESTRICT ON UPDATE CASCADE,
    rating SMALLINT NOT NULL CHECK (rating >= 0 AND rating <= 5), -- limit to a 0-5 star rating
    comment VARCHAR(500) NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX ON drink (drank_on);
CREATE INDEX ON drink (beer_id);
CREATE INDEX ON drink (rating);

SELECT diesel_manage_updated_at('beer');
SELECT diesel_manage_updated_at('brewery');
SELECT diesel_manage_updated_at('drink');

CREATE INDEX brewery_name_lower_idx ON brewery (LOWER(name));
CREATE INDEX beer_name_lower_idx ON beer (LOWER(name));
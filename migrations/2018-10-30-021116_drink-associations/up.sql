
-- Postgres does not currently support adding columns in a specific position
--   so for now I'm just going to drop the old table and create a new one. 

ALTER TABLE drink RENAME TO drink_old;

-- Create a new table with `person_id` in the desired position
CREATE TABLE drink (
    id SERIAL PRIMARY KEY,
    person_id INTEGER NOT NULL REFERENCES person(id) ON DELETE CASCADE ON UPDATE CASCADE,
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

SELECT diesel_manage_updated_at('drink');

DROP TABLE drink_old;

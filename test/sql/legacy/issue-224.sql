CREATE SCHEMA gc;
CREATE TABLE gc.cv_data (
  pk_data_id INT8
);
CREATE TABLE gc.cv_var (
  pk_var_id INT8
);
CREATE TABLE gc.cv_vol (
  pk_vol_id INT8
);

CREATE INDEX idxcv_data
  ON gc.cv_data USING zombodb ( (cv_data.*) ) WITH (shards=40, replicas=1);
CREATE INDEX idxcv_var
  ON gc.cv_var USING zombodb ( (cv_var.*) ) WITH (shards=40, replicas=1);
CREATE INDEX idxcv_vol
  ON gc.cv_vol USING zombodb ( (cv_vol.*) ) WITH (shards=40, replicas=1);

-- get all the pkey/fkey contraints configured
ALTER TABLE gc.cv_data
  ADD CONSTRAINT idx_gc_cv_data_pkey PRIMARY KEY (pk_data_id);

ALTER TABLE gc.cv_var
  ADD CONSTRAINT idx_gc_cv_var_pkey PRIMARY KEY (pk_var_id);

ALTER TABLE gc.cv_var
  ADD CONSTRAINT cv_var_fk_id FOREIGN KEY (pk_var_id)
REFERENCES gc.cv_data (pk_data_id) MATCH SIMPLE
ON UPDATE NO ACTION ON DELETE CASCADE;

ALTER TABLE gc.cv_vol
  ADD CONSTRAINT idx_gc_cv_vol_pkey PRIMARY KEY (pk_vol_id);

ALTER TABLE gc.cv_vol
  ADD CONSTRAINT cv_vol_fk_id FOREIGN KEY (pk_vol_id)
REFERENCES gc.cv_data (pk_data_id) MATCH SIMPLE
ON UPDATE NO ACTION ON DELETE CASCADE;

-- insert first set of test records
INSERT INTO gc.cv_data (pk_data_id) VALUES (1), (2), (3);
INSERT INTO gc.cv_var (pk_var_id) VALUES (1), (2), (3);
INSERT INTO gc.cv_vol (pk_vol_id) VALUES (1), (2), (3);

-- remove ids 1 and 2
BEGIN;
DELETE FROM gc.cv_data
WHERE pk_data_id IN ('1', '2');
COMMIT;

-- vacuum tables so cv will recognize free space created by deleted ids 1 & 2
VACUUM gc.cv_data;
VACUUM gc.cv_var;
VACUUM gc.cv_vol;

-- insert last set of records... the var/vol inserts should ERROR if #224 is bugged
INSERT INTO gc.cv_data (pk_data_id) VALUES (4), (5), (6);
INSERT INTO gc.cv_var (pk_var_id) VALUES (4), (5), (6);
INSERT INTO gc.cv_vol (pk_vol_id) VALUES (4), (5), (6);

BEGIN;
DELETE FROM gc.cv_data
WHERE pk_data_id IN ('3', '4');
COMMIT;

VACUUM FULL gc.cv_data;
VACUUM FULL gc.cv_var;
VACUUM FULL gc.cv_vol;

BEGIN;
DELETE FROM gc.cv_var WHERE pk_var_id = 5;
DELETE FROM gc.cv_vol WHERE pk_vol_id = 5;
COMMIT;
VACUUM gc.cv_var;
VACUUM gc.cv_vol;
-- should cause a DocumentAlreadyExists ERROR from Elasticsearch if #224 is bugged
INSERT INTO gc.cv_vol (pk_vol_id) VALUES (5);

DROP SCHEMA gc CASCADE;
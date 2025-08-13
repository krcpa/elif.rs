CREATE TABLE todos (
id UUID PRIMARY KEY  DEFAULT gen_random_uuid(),
title TEXT  NOT NULL ,
done BOOLEAN   DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_todos_done ON todos (done);

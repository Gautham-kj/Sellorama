CREATE TABLE IF NOT EXISTS "item_media" (
    media_id UUID PRIMARY KEY NOT NULL,
    item_id UUID NOT NULL,
    FOREIGN KEY (item_id) REFERENCES "item"(item_id) on delete cascade
)
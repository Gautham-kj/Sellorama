CREATE TABLE "stock" (
    item_id UUID NOT NULL,
    quantity int NOT NULL,
    PRIMARY KEY (item_id),
    FOREIGN KEY (item_id) REFERENCES "item" (item_id) ON DELETE CASCADE
);
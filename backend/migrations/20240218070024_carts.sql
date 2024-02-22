CREATE TABLE IF NOT EXISTS "cart" (
    cart_id uuid NOT NULL,
    item_id uuid NOT NULL,
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    FOREIGN KEY (cart_id) REFERENCES "user"(user_id) ON DELETE CASCADE,
    FOREIGN KEY (item_id) REFERENCES "item"(item_id) ON DELETE CASCADE,
    PRIMARY KEY (cart_id, item_id)
)
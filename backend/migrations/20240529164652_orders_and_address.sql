CREATE TABLE IF NOT EXISTS "order" (
    order_id UUID DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL,
    order_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    dispatched BOOLEAN NOT NULL DEFAULT FALSE,
    FOREIGN KEY (user_id) REFERENCES "user"(user_id),
    FOREIGN KEY (item_id) REFERENCES "item"(item_id),
    PRIMARY KEY (order_id, user_id, item_id)
);

CREATE TABLE IF NOT EXISTS "order_items"(
    order_id UUID NOT NULL,
    item_id UUID NOT NULL,
    quantity INT NOT NULL,
    FOREIGN KEY (order_id) REFERENCES "order"(order_id) ON DELETE CASCADE,
    FOREIGN KEY (item_id) REFERENCES "item"(item_id) ON DELETE CASCADE,
    PRIMARY KEY (order_id, item_id)
);

CREATE TABLE IF NOT EXISTS "address" (
    user_id UUID NOT NULL,
    order_id UUID NOT NULL,
    address_line_1 VARCHAR(255) NOT NULL,
    address_line_2 VARCHAR(255),
    city VARCHAR(255) NOT NULL,
    country VARCHAR(255) NOT NULL,
    pincode VARCHAR(255) NOT NULL,
    FOREIGN KEY (user_id) REFERENCES "user"(user_id) ON DELETE CASCADE,
    FOREIGN KEY (order_id) REFERENCES "order"(order_id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, order_id)
);
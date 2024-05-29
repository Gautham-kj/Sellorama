CREATE TABLE IF NOT EXISTS "order" (
    order_id UUID PRIMARY KEY NOT NULL,
    user_id UUID NOT NULL,
    order_date TIMESTAMP NOT NULL,
    total_amount DECIMAL(10, 2) NOT NULL,
    FOREIGN KEY (user_id) REFERENCES "user"(user_id) ON DELETE CASCADE
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
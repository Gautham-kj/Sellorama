CREATE OR REPLACE FUNCTION item_ownership(item_id UUID, user_id UUID) RETURNS BOOLEAN AS $$
DECLARE
    id_1 alias for $1;
    id_2 alias for $2;
BEGIN
    SELECT * FROM "item" WHERE "item"."item_id" = id_1 AND "item"."user_id" = id_2;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION item_ownership(item_id UUID, user_id UUID) RETURNS BOOLEAN AS $$
DECLARE
    id_1 alias for $1;
    id_2 alias for $2;
    result BOOLEAN;
BEGIN
    SELECT EXISTS (SELECT * FROM "item" WHERE "item"."item_id" = id_1 AND "item"."user_id" = id_2) INTO result ;
    RETURN result;
END;
$$ LANGUAGE plpgsql;


CREATE OR REPLACE FUNCTION stock_validation(item_id UUID, quantity INT) RETURNS BOOLEAN AS $$
DECLARE
    id_1 alias for $1;
    quantity_1 alias for $2;
    result BOOLEAN;
BEGIN
    SELECT EXISTS (SELECT * FROM "stock" WHERE "stock"."item_id" = id_1 AND "stock"."quantity" >= quantity_1) INTO result ;
    RETURN result;
END;
$$ LANGUAGE plpgsql;

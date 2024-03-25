
CREATE OR REPLACE FUNCTION ratings_calculation() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    UPDATE "item"
    SET rating = (SELECT AVG(rating) FROM "comment" WHERE "comment"."item_id" = NEW.item_id)
    WHERE "item"."item_id" = NEW.item_id;
    RETURN NEW;
END;
$$;

CREATE TRIGGER ratings_trigger
    AFTER INSERT
    ON "comment"
    FOR EACH ROW
    EXECUTE FUNCTION ratings_calculation();
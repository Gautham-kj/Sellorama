CREATE OR REPLACE FUNCTION post_count() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    UPDATE "user"
    SET "post_count" = (SELECT COUNT(*) FROM "item" WHERE "item"."user_id" = NEW.user_id)
    WHERE "user"."user_id" = NEW.user_id;
    RETURN NEW;
END;
$$;

CREATE TRIGGER item_count_trigger
    AFTER INSERT
    ON "item"
    FOR EACH ROW
    EXECUTE FUNCTION post_count();
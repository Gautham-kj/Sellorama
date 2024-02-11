
CREATE EXTENSION IF NOT EXISTS "uuid-ossp" SCHEMA public;
CREATE TABLE "user" (
    user_id UUID DEFAULT UUID_generate_v4 (),
    username varchar(255) UNIQUE NOT NULL,
    email_id varchar(255) UNIQUE NOT NULL,
    date_created timestamp DEFAULT CURRENT_TIMESTAMP,
    post_count int DEFAULT 0,
    PRIMARY KEY (user_id)
);
create table "password" (
  user_id uuid, 
  hashed_pass VARCHAR(255) NOT NULL, 
  foreign key (user_id) references "user"(user_id) ON DELETE CASCADE

);

CREATE TABLE "item" (
    item_id UUID DEFAULT UUID_generate_v4 (),
    user_id UUID NOT NULL,
    title varchar(100) NOT NULL,
    content text NOT NULL,
    date_created timestamp DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (item_id),
    FOREIGN KEY (user_id) REFERENCES "user" (user_id) ON DELETE CASCADE
);
-- create table "postmedia"();
CREATE TABLE "comment" (
    comment_id UUID DEFAULT UUID_generate_v4 (),
    user_id UUID,
    item_id UUID,
    content text NOT NULL,
    date_created timestamp DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (comment_id),
    FOREIGN KEY (user_id) REFERENCES "user" (user_id) ON DELETE CASCADE,
    FOREIGN KEY (item_id) REFERENCES "item" (item_id) ON DELETE CASCADE
);


CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE TABLE "user" (
  user_id uuid DEFAULT uuid_generate_v4(), 
  username VARCHAR(255) unique NOT NULL, 
  email_id VARCHAR(255) unique NOT NULL, 
  date_created TIMESTAMP DEFAULT CURRENT_TIMESTAMP, 
  post_count INT DEFAULT 0, 
  PRIMARY KEY (user_id)
);
create table "password" (
  user_id uuid, 
  password VARCHAR(255) NOT NULL, 
  foreign key (user_id) references "user"(user_id) ON DELETE CASCADE
);
create table "post" (
  post_id uuid DEFAULT uuid_generate_v4(), 
  user_id uuid, 
  title VARCHAR(255) NOT NULL, 
  content TEXT NOT NULL, 
  date_created TIMESTAMP DEFAULT CURRENT_TIMESTAMP, 
  PRIMARY KEY (post_id), 
  foreign key (user_id) references "user"(user_id) on DELETE CASCADE
);
-- create table "postmedia"();
create table "comment" (
  comment_id uuid DEFAULT uuid_generate_v4(), 
  user_id uuid, 
  post_id uuid, 
  content TEXT NOT NULL, 
  date_created TIMESTAMP DEFAULT CURRENT_TIMESTAMP, 
  PRIMARY KEY (comment_id), 
  foreign key (user_id) references "user"(user_id) on DELETE CASCADE, 
  foreign key (post_id) references "post"(post_id) on DELETE CASCADE
);

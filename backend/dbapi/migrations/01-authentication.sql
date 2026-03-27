-- Stores user records
CREATE TABLE `users`(
    `id` TEXT NOT NULL,
    `name` TEXT NOT NULL,
    `email` TEXT NOT NULL
);

-- Stores session tokens (Mediacorral <-> client)
CREATE TABLE `session_tokens`(
	`session_token` TEXT NOT NULL PRIMARY KEY,
    `user_id` TEXT NOT NULL,
    `expires` INTEGER NOT NULL
);

-- Stores drive controller authentication tokens
CREATE TABLE `drive_tokens`(
    `id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    `name` TEXT NOT NULL,
    `token_hash` TEXT NOT NULL
);

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

UPDATE migrations SET value = 2 WHERE key = 'version';

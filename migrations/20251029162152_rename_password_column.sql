-- Add migration script here
ALTER TABLE users RENAME password to hash_password;

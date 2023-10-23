-- Add up migration script here

ALTER TABLE `user` ADD COLUMN `verified` BOOL NOT NULL DEFAULT FALSE AFTER `town_id`;

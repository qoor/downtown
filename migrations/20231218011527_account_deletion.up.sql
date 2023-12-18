-- Add up migration script here

ALTER TABLE `user` ADD COLUMN `deleted` BOOL NOT NULL DEFAULT FALSE AFTER `bio`;

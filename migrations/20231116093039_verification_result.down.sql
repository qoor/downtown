-- Add down migration script here

ALTER TABLE `user` DROP COLUMN `verification_result`;

ALTER TABLE `user` ADD COLUMN `verified` bool NOT NULL DEFAULT FALSE AFTER `town_id`;

-- Add up migration script here

ALTER TABLE `user` DROP COLUMN `verified`;

ALTER TABLE `user` ADD COLUMN `verification_result` int(10) unsigned NOT NULL DEFAULT 0 AFTER `town_id`;

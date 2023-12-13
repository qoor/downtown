-- Add up migration script here

ALTER TABLE `user` MODIFY COLUMN `verification_type` int(10) unsigned;

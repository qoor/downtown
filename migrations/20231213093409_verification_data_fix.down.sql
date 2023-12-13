-- Add down migration script here

ALTER TABLE `user` MODIFY COLUMN `verification_type` bool;

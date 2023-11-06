-- Add up migration script here

ALTER TABLE `post` MODIFY COLUMN `age_range` int(10) unsigned;

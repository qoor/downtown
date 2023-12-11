-- Add up migration script here

ALTER TABLE `user` MODIFY COLUMN `verification_type` bool;
ALTER TABLE `user` CHANGE COLUMN `verification_photo_url` `verification_picture_url` varchar(4096);

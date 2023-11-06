-- Add down migration script here

ALTER TABLE `post_image` MODIFY COLUMN `image_url` varchar(4096);

-- Add up migration script here

ALTER TABLE `post_comment_closure` DROP FOREIGN KEY `post_parent_comment`;
ALTER TABLE `post_comment_closure` DROP FOREIGN KEY `post_child_comment`;

ALTER TABLE `post_comment_closure` ADD CONSTRAINT `post_parent_comment` FOREIGN KEY (`parent_comment_id`) REFERENCES `post_comment` (`id`) ON DELETE CASCADE;
ALTER TABLE `post_comment_closure` ADD CONSTRAINT `post_child_comment` FOREIGN KEY (`child_comment_id`) REFERENCES `post_comment` (`id`) ON DELETE CASCADE;

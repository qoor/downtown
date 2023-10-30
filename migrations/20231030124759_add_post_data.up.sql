-- Add up migration script here

CREATE TABLE `post_type` (
  `id` int(10) unsigned NOT NULL AUTO_INCREMENT,
  `name` varchar(16) NOT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

INSERT INTO `post_type` (`id`, `name`) VALUES
  (1, 'daily'),
  (2, 'question'),
  (3, 'gathering');

ALTER TABLE `post`
  ADD COLUMN `post_type` int(10) unsigned NOT NULL AFTER `author_id`,
  ADD CONSTRAINT `post_type` FOREIGN KEY (`post_type`) REFERENCES `post_type` (`id`);

ALTER TABLE `post`
  ADD COLUMN `town_id` int(10) unsigned NOT NULL AFTER `post_type`,
  ADD CONSTRAINT `post_town` FOREIGN KEY (`town_id`) REFERENCES `town` (`id`);

-- Add down migration script here

ALTER TABLE `post`
  DROP COLUMN `type`,
  DROP FOREIGN KEY `post_type`;

ALTER TABLE `post`
  DROP COLUMN `town_id`,
  DROP FOREIGN KEY `post_town`;

DROP TABLE `post_type`;

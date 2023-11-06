-- Add down migration script here

ALTER TABLE `post`
  DROP FOREIGN KEY `post_age_range`,
  DROP COLUMN `age_range`,
  DROP COLUMN `capacity`,
  DROP COLUMN `place`;

DROP TABLE `gathering_age_range`;

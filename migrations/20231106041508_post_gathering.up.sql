-- Add up migration script here

CREATE TABLE `gathering_age_range` (
  `id` int(10) unsigned NOT NULL AUTO_INCREMENT,
  `min_age` int(10) unsigned,
  `max_age` int(10) unsigned,
  `description` varchar(8) NOT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

INSERT INTO `gathering_age_range` (id, min_age, max_age, description) VALUES
  (1, 20, 29, '무관'),
  (2, 20, 29, '10대'),
  (3, 20, 29, '20대'),
  (4, 30, 39, '30대'),
  (5, 40, 49, '40대'),
  (6, 50, NULL, '50대');

ALTER TABLE `post`
  ADD COLUMN `age_range` int(10) unsigned AFTER `content`,
  ADD COLUMN `capacity` int(10) unsigned AFTER `age_range`,
  ADD COLUMN `place` varchar(128) AFTER `capacity`,
  ADD CONSTRAINT `post_age_range` FOREIGN KEY (`age_range`) REFERENCES `gathering_age_range` (`id`);

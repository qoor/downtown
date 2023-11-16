-- Add up migration script here

DROP TABLE `phone_verification`;

CREATE TABLE `phone_authorization` (
  `id` int(10) unsigned NOT NULL AUTO_INCREMENT,
  `phone` char(13) NOT NULL,
  `code` char(6) NOT NULL,
  `created_at` timestamp NOT NULL DEFAULT current_timestamp(),
  PRIMARY KEY (`id`),
  UNIQUE KEY `unique_phone` (`phone`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

-- Add up migration script here

CREATE TABLE `town` (
  `id` int(10) unsigned NOT NULL AUTO_INCREMENT,
  `address` varchar(256) NOT NULL,
  `created_at` timestamp NOT NULL DEFAULT current_timestamp(),
  `updated_at` timestamp NOT NULL DEFAULT current_timestamp() ON UPDATE current_timestamp(),
  PRIMARY KEY (`id`),
  UNIQUE KEY `unique_address` (`address`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE `user` (
  `id` int(10) unsigned NOT NULL AUTO_INCREMENT,
  `name` varchar(8) NOT NULL,
  `phone` char(13) NOT NULL,
  `birthdate` date NOT NULL,
  `sex` tinyint(1) unsigned NOT NULL,
  `town_id` int(10) unsigned NOT NULL,
  `verification_type` tinyint(1) unsigned NOT NULL,
  `verification_photo_url` varchar(4096) NOT NULL,
  `refresh_token` varchar(4096),
  `created_at` timestamp NOT NULL DEFAULT current_timestamp(),
  `updated_at` timestamp NOT NULL DEFAULT current_timestamp() ON UPDATE current_timestamp(),
  PRIMARY KEY (`id`),
  UNIQUE KEY `unique_phone` (`phone`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE `phone_verification` (
  `id` int(10) unsigned NOT NULL AUTO_INCREMENT,
  `phone` char(13) NOT NULL,
  `code` char(6) NOT NULL,
  `created_at` timestamp NOT NULL DEFAULT current_timestamp(),
  PRIMARY KEY (`id`),
  UNIQUE KEY `unique_phone` (`phone`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

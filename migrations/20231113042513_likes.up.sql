-- Add up migration script here

CREATE TABLE `user_like` (
  `id` int(10) unsigned NOT NULL AUTO_INCREMENT,
  `issuer_id` int(10) unsigned NOT NULL,
  `target_id` int(10) unsigned NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY (`issuer_id`, `target_id`),
  CONSTRAINT `user_like_issuer` FOREIGN KEY (`issuer_id`) REFERENCES `user` (`id`) ON DELETE CASCADE,
  CONSTRAINT `user_like_target` FOREIGN KEY (`target_id`) REFERENCES `user` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE `post_like` (
  `id` int(10) unsigned NOT NULL AUTO_INCREMENT,
  `user_id` int(10) unsigned NOT NULL,
  `post_id` int(10) unsigned NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY (`user_id`, `post_id`),
  CONSTRAINT `post_like_user` FOREIGN KEY (`user_id`) REFERENCES `user` (`id`) ON DELETE CASCADE,
  CONSTRAINT `post_like_post` FOREIGN KEY (`post_id`) REFERENCES `post` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

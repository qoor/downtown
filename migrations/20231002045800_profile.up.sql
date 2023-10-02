-- Add up migration script here

ALTER TABLE `user`
  ADD COLUMN `photo` varchar(4096) NOT NULL DEFAULT 'https://respec-public.s3.ap-northeast-2.amazonaws.com/profile_image/profile_image_default.png' AFTER `verification_photo_url`,
  ADD COLUMN `bio` varchar(512) AFTER `photo`
;

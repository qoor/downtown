-- Add down migration script here

ALTER TABLE `user`
  DROP COLUMN `picture`,
  DROP COLUMN `bio`
;

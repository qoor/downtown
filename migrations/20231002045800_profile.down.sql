-- Add down migration script here

ALTER TABLE `user`
  DROP COLUMN `photo`,
  DROP COLUMN `bio`
;

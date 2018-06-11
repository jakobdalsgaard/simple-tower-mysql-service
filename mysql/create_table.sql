 CREATE TABLE `items` (
      `id` int(11) NOT NULL AUTO_INCREMENT,
      `ident` varchar(255) NOT NULL,
      `name` varchar(255) NOT NULL,
      `created_at` datetime DEFAULT NULL,
      `description` varchar(255) DEFAULT NULL,
      PRIMARY KEY (`id`),
      UNIQUE KEY `items_ident_index` (`ident`)
) DEFAULT CHARSET=utf8mb4 ROW_FORMAT=DYNAMIC

CREATE TABLE IF NOT EXISTS "backups" (
	"id" INTEGER NOT NULL UNIQUE,
	"save_code" TEXT NOT NULL,
	"created_at" TEXT NOT NULL,
	PRIMARY KEY("id" AUTOINCREMENT)
);

CREATE INDEX IF NOT EXISTS "backups_created_at" ON "backups" ("created_at" DESC);
package migrations

import (
	"database/sql"
	"fmt"
	"log/slog"

	"modernc.org/sqlite"
	"modernc.org/sqlite/lib"
)

func InitDb(db *sql.DB) error {
	var version uint
	for {
		err := db.QueryRow(`SELECT value FROM migrations WHERE key = 'version'`).Scan(&version)
		if err != nil {
			if sqliteErr, ok := err.(*sqlite.Error); ok {
				switch sqliteErr.Code() {
				case sqlite3.SQLITE_ERROR:
					version = 0
				default:
					return sqliteErr
				}
			} else {
				return err
			}
		}

		switch version {
		case 0:
			slog.Info("Initializing database")
			err := MigrationInit(db)
			if err != nil {
				return fmt.Errorf("failed to initialize migration: %w", err)
			}
		case 1:
			err := MigrationAuth(db)
			if err != nil {
				return fmt.Errorf("failed to initialize auth migration: %w", err)
			}
		case 2:
			return nil
		default:
			return fmt.Errorf("unknown schema version: %d", version)
		}
	}
}

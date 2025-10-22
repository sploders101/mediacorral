package migrations

import (
	"database/sql"
	"fmt"
	"log/slog"

	"github.com/mattn/go-sqlite3"
)

func InitDb(db *sql.DB) error {
	var version uint
	for {
		err := db.QueryRow(`SELECT value FROM migrations WHERE key = 'version'`).Scan(&version)
		if err != nil {
			if err, ok := err.(sqlite3.Error); ok {
				switch err.Code {
				case sqlite3.ErrError:
					version = 0
				default:
					return err
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
				return err
			}
		case 1:
			return nil
		default:
			return fmt.Errorf("unknown schema version: %d", version)
		}
	}
}

package migrations

import (
	"database/sql"
	_ "embed"
)

//go:embed 00-init.sql
var INIT_SQL string

func MigrationInit(db *sql.DB) error {
	_, err := db.Exec(INIT_SQL)
	if err != nil {
		return err
	}
	return nil
}

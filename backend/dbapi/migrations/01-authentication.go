package migrations

import (
	"database/sql"
	_ "embed"
)

//go:embed 01-authentication.sql
var AUTH_SQL_01 string

func MigrationAuth(db *sql.DB) error {
	_, err := db.Exec(INIT_SQL)
	if err != nil {
		return err
	}
	return nil
}

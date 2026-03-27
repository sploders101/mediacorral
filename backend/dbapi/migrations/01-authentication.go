package migrations

import (
	"database/sql"
	_ "embed"
)

//go:embed 01-authentication.sql
var AUTH_SQL_01 string

func MigrationAuth(db *sql.DB) error {
	_, err := db.Exec(AUTH_SQL_01)
	if err != nil {
		return err
	}
	return nil
}

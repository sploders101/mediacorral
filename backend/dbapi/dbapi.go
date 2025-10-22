package dbapi

import (
	"database/sql"
	"fmt"
	"path"

	"github.com/sploders101/mediacorral/backend/dbapi/migrations"
	"github.com/sploders101/mediacorral/backend/helpers/config"
)

type Db struct {
	db *sql.DB
}

func NewDb(config config.ConfigFile) (Db, error) {
	db, err := sql.Open("sqlite3", path.Join(config.DataDirectory, "database.sqlite"))
	if err != nil {
		return Db{}, fmt.Errorf("an error occurred while opening the database: %w", err)
	}
	if err := migrations.InitDb(db); err != nil {
		return Db{}, err
	}

	return Db{db: db}, nil
}

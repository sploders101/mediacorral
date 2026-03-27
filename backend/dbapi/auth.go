package dbapi

import (
	"database/sql"
	"errors"
	"time"
)

func (db *DbTx) InsertUser(id string, name string, email string) error {
	_, err := db.tx.Exec(
		`
			INSERT INTO users (
				id,
				name,
				email
			) VALUES (?, ?, ?)
		`,
		id,
		name,
		email,
	)
	if err != nil {
		return err
	}

	return nil
}

type UserMeta struct {
	Id    string
	Name  string
	Email string
}

func (db *DbTx) GetUser(id string) (UserMeta, error) {
	result := db.tx.QueryRow(
		`
			SELECT
				id,
				name,
				email
			FROM users
			WHERE id = ?
		`,
		id,
	)
	var user UserMeta
	if err := result.Scan(&user.Id, &user.Name, &user.Email); err != nil {
		return UserMeta{}, err
	}
	return user, nil
}

func (db *DbTx) UpdateUser(id string, name string, email string) error {
	_, err := db.tx.Exec(
		`
			UPDATE users
			SET
				name = ?,
				email = ?
			WHERE id = ?
		`,
		name,
		email,
		id,
	)
	return err
}

func (db *DbTx) InsertSessionToken(sessionToken string, userId string, expiration time.Time) error {
	_, err := db.tx.Exec(
		`
			INSERT INTO session_tokens (
				session_token,
				user_id,
				expires
			) VALUES (?, ?, ?)
		`,
		sessionToken,
		userId,
		expiration.Unix(),
	)
	if err != nil {
		return err
	}

	return nil
}

type SessionMeta struct {
	SessionToken string
	UserId       string
	UserName     string
	UserEmail    string
	Expires      time.Time
}

func (db *DbTx) GetSessionMeta(sessionToken string) (SessionMeta, error) {
	result := db.tx.QueryRow(
		`
			SELECT
				session_tokens.session_token,
				session_tokens.user_id,
				session_tokens.expires,
				users.name,
				users.email
			FROM session_tokens
			JOIN users ON session_toikens.user_id = users.id
			WHERE session_token = ? AND session_tokens.expires > unixepoch()
		`,
		sessionToken,
	)

	var session SessionMeta
	var expirationUnix int64
	if err := result.Scan(
		&session.SessionToken,
		&session.UserId,
		&expirationUnix,
		session.UserName,
		session.UserEmail,
	); err != nil {
		return SessionMeta{}, err
	}
	session.Expires = time.Unix(expirationUnix, 0)
	return session, nil
}

// Ensures the session has at least 24 hours before expiration, but only if it hasn't already expired.
// Returns a boolean indicating whether or not the token was valid.
func (db *DbTx) ProbeSession(sessionToken string) (bool, error) {
	result := db.tx.QueryRow(
		`
			UPDATE session_tokens
			SET expires = unixepoch() + 86400
			WHERE
				session_token = ?
				AND expires > unixepoch()
			RETURNING session_token
		`,
		sessionToken,
	)
	var sessionTokenRecv sql.Null[string]
	if err := result.Scan(&sessionTokenRecv); err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return false, nil
		}
		return false, err
	}
	return sessionTokenRecv.Valid, nil
}

func (db *DbTx) DeleteSessionToken(sessionToken string) error {
	_, err := db.tx.Exec(
		`
			DELETE FROM session_tokens
			WHERE session_token = ?
		`,
		sessionToken,
	)
	return err
}

func (db *DbTx) AuthGC() error {
	_, err := db.tx.Exec(
		`
			DELETE FROM session_tokens
			WHERE expires < unixepoch()
		`,
	)
	if err != nil {
		return err
	}

	return nil
}

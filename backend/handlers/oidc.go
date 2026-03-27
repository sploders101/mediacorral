package handlers

import (
	"context"
	"crypto/rand"
	"database/sql"
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"log/slog"
	"net/http"
	"time"

	"github.com/coreos/go-oidc/v3/oidc"
	"github.com/sploders101/mediacorral/backend/dbapi"
	"github.com/sploders101/mediacorral/backend/helpers/config"
	"golang.org/x/oauth2"
)

type OIDCHandler struct {
	config    *config.OIDCConfig
	db        dbapi.Db
	provider  *oidc.Provider
	oauth2Cfg *oauth2.Config
	verifier  *oidc.IDTokenVerifier
}

func NewOIDCHandler(cfg *config.OIDCConfig, db dbapi.Db) (*OIDCHandler, error) {
	ctx := context.Background()
	provider, err := oidc.NewProvider(ctx, cfg.Issuer)
	if err != nil {
		return nil, fmt.Errorf("failed to create OIDC provider: %w", err)
	}

	verifier := provider.Verifier(&oidc.Config{
		ClientID: *cfg.ClientID,
	})

	scopes := cfg.Scopes
	if scopes == nil {
		scopes = []string{oidc.ScopeOpenID, "profile", "email"}
	}

	oauth2Cfg := &oauth2.Config{
		ClientID:     *cfg.ClientID,
		ClientSecret: *cfg.ClientSecret,
		Endpoint:     provider.Endpoint(),
		RedirectURL:  cfg.RedirectURL,
		Scopes:       scopes,
	}

	return &OIDCHandler{
		config:    cfg,
		db:        db,
		provider:  provider,
		oauth2Cfg: oauth2Cfg,
		verifier:  verifier,
	}, nil
}

func (handler *OIDCHandler) Register(mux *http.ServeMux) {
	mux.HandleFunc("GET /auth/login", handler.handleLogin)
	mux.HandleFunc("GET /auth/callback", handler.handleCallback)
	mux.HandleFunc("POST /auth/logout", handler.handleLogout)
	mux.HandleFunc("GET /auth/userinfo", handler.handleUserInfo)
}

func (handler *OIDCHandler) handleLogin(resp http.ResponseWriter, req *http.Request) {
	state, err := generateState()
	if err != nil {
		slog.Error("Failed to generate state", "error", err.Error())
		http.Error(resp, "Internal server error", http.StatusInternalServerError)
		return
	}

	authURL := handler.oauth2Cfg.AuthCodeURL(state)
	http.Redirect(resp, req, authURL, http.StatusFound)
}

func (handler *OIDCHandler) handleCallback(resp http.ResponseWriter, req *http.Request) {
	state := req.URL.Query().Get("state")
	if state == "" {
		http.Error(resp, "Missing state parameter", http.StatusBadRequest)
		return
	}

	code := req.URL.Query().Get("code")
	if code == "" {
		http.Error(resp, "Missing code parameter", http.StatusBadRequest)
		return
	}

	token, err := handler.oauth2Cfg.Exchange(req.Context(), code)
	if err != nil {
		slog.Error("Failed to exchange code for token", "error", err.Error())
		http.Error(resp, "Failed to exchange code for token", http.StatusInternalServerError)
		return
	}

	rawIDToken, ok := token.Extra("id_token").(string)
	if !ok {
		http.Error(resp, "No id_token in response", http.StatusInternalServerError)
		return
	}

	idToken, err := handler.verifier.Verify(req.Context(), rawIDToken)
	if err != nil {
		slog.Error("Failed to verify ID token", "error", err.Error())
		http.Error(resp, "Failed to verify ID token", http.StatusInternalServerError)
		return
	}

	var claims struct {
		Subject string `json:"sub"`
		Email   string `json:"email"`
		Name    string `json:"name"`
	}
	if err := idToken.Claims(&claims); err != nil {
		slog.Error("Failed to parse claims", "error", err.Error())
		http.Error(resp, "Failed to parse claims", http.StatusInternalServerError)
		return
	}

	dbTx, err := handler.db.Begin()
	if err != nil {
		slog.Error("Failed to begin database transaction", "error", err.Error())
		http.Error(resp, "Internal server error", http.StatusInternalServerError)
		return
	}
	defer func() { _ = dbTx.Rollback() }()

	user, err := dbTx.GetUser(claims.Subject)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			if err := dbTx.InsertUser(claims.Subject, claims.Name, claims.Email); err != nil {
				slog.Error("Failed to insert user", "error", err.Error())
				http.Error(resp, "Internal server error", http.StatusInternalServerError)
				return
			}
		} else {
			slog.Error("Failed to get user info", "error", err.Error())
			http.Error(resp, "Internal server error", http.StatusInternalServerError)
			return
		}
	} else {
		if user.Name != claims.Name || user.Email != claims.Email {
			if err := dbTx.UpdateUser(claims.Subject, claims.Name, claims.Email); err != nil {
				slog.Error("Failed to update user", "error", err.Error())
				http.Error(resp, "Internal server error", http.StatusInternalServerError)
				return
			}
		}
	}

	sessionToken, err := generateSessionToken()
	if err != nil {
		slog.Error("Failed to generate session token", "error", err.Error())
		http.Error(resp, "Internal server error", http.StatusInternalServerError)
		return
	}

	expiration := time.Now().Add(24 * time.Hour)
	if err := dbTx.InsertSessionToken(sessionToken, claims.Subject, expiration); err != nil {
		slog.Error("Failed to insert session token", "error", err.Error())
		http.Error(resp, "Internal server error", http.StatusInternalServerError)
		return
	}

	if err := dbTx.Commit(); err != nil {
		slog.Error("Failed to commit database transaction", "error", err.Error())
		http.Error(resp, "Internal server error", http.StatusInternalServerError)
		return
	}

	http.SetCookie(resp, &http.Cookie{
		Name:     "session",
		Value:    sessionToken,
		Expires:  expiration,
		HttpOnly: true,
		Secure:   true,
		SameSite: http.SameSiteLaxMode,
		Path:     "/",
	})

	redirectURL := "/"
	http.Redirect(resp, req, redirectURL, http.StatusFound)
}

func (handler *OIDCHandler) handleLogout(resp http.ResponseWriter, req *http.Request) {
	cookie, err := req.Cookie("session")
	if err != nil {
		http.Error(resp, "Missing session cookie", http.StatusUnauthorized)
		return
	}

	sessionToken := cookie.Value

	dbTx, err := handler.db.Begin()
	if err != nil {
		slog.Error("Failed to begin database transaction", "error", err.Error())
		http.Error(resp, "Internal server error", http.StatusInternalServerError)
		return
	}
	defer func() { _ = dbTx.Rollback() }()

	if err := dbTx.DeleteSessionToken(sessionToken); err != nil {
		slog.Error("Failed to delete session token", "error", err.Error())
		http.Error(resp, "Internal server error", http.StatusInternalServerError)
		return
	}

	if err := dbTx.Commit(); err != nil {
		slog.Error("Failed to commit database transaction", "error", err.Error())
		http.Error(resp, "Internal server error", http.StatusInternalServerError)
		return
	}

	http.SetCookie(resp, &http.Cookie{
		Name:     "session",
		Value:    "",
		Expires:  time.Unix(0, 0),
		HttpOnly: true,
		Secure:   true,
		SameSite: http.SameSiteLaxMode,
		Path:     "/",
	})

	resp.WriteHeader(http.StatusNoContent)
}

func (handler *OIDCHandler) handleUserInfo(resp http.ResponseWriter, req *http.Request) {
	cookie, err := req.Cookie("session")
	if err != nil {
		http.Error(resp, "Missing session cookie", http.StatusUnauthorized)
		return
	}

	dbTx, err := handler.db.Begin()
	if err != nil {
		slog.Error("Failed to begin database transaction", "error", err.Error())
		http.Error(resp, "Internal server error", http.StatusInternalServerError)
		return
	}
	defer func() { _ = dbTx.Rollback() }()

	sessionMeta, err := dbTx.GetSessionMeta(cookie.Value)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			http.Error(resp, "Invalid session", http.StatusUnauthorized)
			return
		}
		slog.Error("Failed to get session metadata", "error", err.Error())
		http.Error(resp, "Internal server error", http.StatusInternalServerError)
		return
	}

	if err := dbTx.Commit(); err != nil {
		slog.Error("Failed to commit database transaction", "error", err.Error())
		http.Error(resp, "Internal server error", http.StatusInternalServerError)
		return
	}

	body := map[string]any{
		"user_id": sessionMeta.UserId,
		"name":    sessionMeta.UserName,
		"email":   sessionMeta.UserEmail,
		"expires": sessionMeta.Expires,
	}

	resp.Header().Set("Content-Type", "application/json")
	json.NewEncoder(resp).Encode(body)
}

func generateState() (string, error) {
	buf := make([]byte, 32)
	if _, err := rand.Read(buf); err != nil {
		return "", err
	}
	return base64.URLEncoding.EncodeToString(buf), nil
}

func generateSessionToken() (string, error) {
	buf := make([]byte, 32)
	if _, err := rand.Read(buf); err != nil {
		return "", err
	}
	return base64.URLEncoding.EncodeToString(buf), nil
}

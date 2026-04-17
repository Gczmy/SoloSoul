package api

import (
	"encoding/base64"
	"encoding/json"
	"fmt"
	"net/http"
	"net/rpc"
	"path/filepath"
	"sync"
	"time"

	"github.com/solosoul/solosoul/core/ocr"
	"github.com/solosoul/solosoul/core/schema"
	"github.com/solosoul/solosoul/core/vault"
)

// HTTPServer is a simple HTTP server for the web UI
type HTTPServer struct {
	basePath      string
	vault         *vault.FileStore
	accountManager *AccountManager
	plugin        *PluginManager
	ocrEngine     *ocr.PaddleOCR
	ocrJobs       *ocr.JobManager
	sessionTokens map[string]*SessionInfo
	mu            sync.RWMutex
	server        *http.Server
}

// NewHTTPServer creates a new HTTP server
func NewHTTPServer(basePath string) (*HTTPServer, error) {
	// Initialize account manager
	am, err := NewAccountManager(basePath)
	if err != nil {
		return nil, fmt.Errorf("failed to create account manager: %w", err)
	}

	// Get default account vault path
	accountPath := filepath.Join(basePath, "default")
	defaultAccount := am.GetDefaultAccount()
	if defaultAccount != nil {
		accountPath = filepath.Join(basePath, defaultAccount.ID)
	}

	vs, err := vault.NewFileStore(accountPath)
	if err != nil {
		return nil, fmt.Errorf("failed to open vault: %w", err)
	}

	pm, err := NewPluginManager(basePath) // Plugin manager uses base path
	if err != nil {
		return nil, fmt.Errorf("failed to create plugin manager: %w", err)
	}

	// Initialize OCR engine
	ocrEngine, err := ocr.NewPaddleOCR("")
	if err != nil {
		return nil, fmt.Errorf("failed to create OCR engine: %w", err)
	}
	ocrJobs := ocr.NewJobManager(ocrEngine)

	return &HTTPServer{
		basePath:      basePath,
		vault:         vs,
		accountManager: am,
		plugin:        pm,
		ocrEngine:     ocrEngine,
		ocrJobs:       ocrJobs,
		sessionTokens: make(map[string]*SessionInfo),
	}, nil
}

// Start starts the HTTP server
func (s *HTTPServer) Start(addr string) error {
	mux := http.NewServeMux()

	// Auth routes
	mux.HandleFunc("GET /api/auth/status", s.handleAuthStatus)
	mux.HandleFunc("POST /api/auth/unlock", s.handleAuthUnlock)
	mux.HandleFunc("POST /api/auth/lock", s.handleAuthLock)
	mux.HandleFunc("POST /api/auth/setup", s.handleAuthSetup)
	mux.HandleFunc("POST /api/auth/password", s.handleChangePassword)

	// Account routes
	mux.HandleFunc("GET /api/accounts", s.handleAccountList)
	mux.HandleFunc("GET /api/accounts/check", s.handleAccountCheck)
	mux.HandleFunc("POST /api/accounts", s.handleAccountCreate)
	mux.HandleFunc("DELETE /api/accounts/{id}", s.handleAccountDelete)
	mux.HandleFunc("PUT /api/accounts/{id}/default", s.handleAccountSetDefault)

	// Profile routes
	mux.HandleFunc("GET /api/profile", s.handleProfileList)
	mux.HandleFunc("GET /api/profile/{id}", s.handleProfileGet)
	mux.HandleFunc("PUT /api/profile", s.handleProfileUpdate)
	mux.HandleFunc("POST /api/profile/validate", s.handleProfileValidate)
	mux.HandleFunc("DELETE /api/profile/{id}", s.handleProfileDelete)

	// Plugin routes
	mux.HandleFunc("GET /api/plugins", s.handlePluginList)
	mux.HandleFunc("GET /api/plugins/{id}/manifest", s.handlePluginManifest)
	mux.HandleFunc("POST /api/plugins/{id}/consent/request", s.handlePluginConsentRequest)
	mux.HandleFunc("POST /api/plugins/consent/grant", s.handlePluginConsentGrant)
	mux.HandleFunc("DELETE /api/plugins/sessions/{id}", s.handlePluginSessionRevoke)
	mux.HandleFunc("GET /api/plugins/{id}/sessions", s.handlePluginSessionsList)

	// OCR routes
	mux.HandleFunc("POST /api/ocr/jobs", s.handleOCRJobSubmit)
	mux.HandleFunc("GET /api/ocr/jobs/{id}", s.handleOCRJobResult)
	mux.HandleFunc("GET /api/ocr/status", s.handleOCRStatus)

	// Health check
	mux.HandleFunc("GET /health", s.handleHealth)

	// CORS wrapper for all routes
	corsMux := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Access-Control-Allow-Origin", "*")
		w.Header().Set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
		w.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization")
		if r.Method == http.MethodOptions {
			w.WriteHeader(http.StatusOK)
			return
		}
		mux.ServeHTTP(w, r)
	})

	s.server = &http.Server{
		Addr:         addr,
		Handler:      corsMux,
		ReadTimeout:  10 * time.Second,
		WriteTimeout: 10 * time.Second,
	}

	return s.server.ListenAndServe()
}

// StartUnix starts the HTTP server on a Unix domain socket
func (s *HTTPServer) StartUnix(socketPath string) error {
	mux := http.NewServeMux()

	// Auth routes
	mux.HandleFunc("GET /api/auth/status", s.handleAuthStatus)
	mux.HandleFunc("POST /api/auth/unlock", s.handleAuthUnlock)
	mux.HandleFunc("POST /api/auth/lock", s.handleAuthLock)
	mux.HandleFunc("POST /api/auth/setup", s.handleAuthSetup)
	mux.HandleFunc("POST /api/auth/password", s.handleChangePassword)

	// Account routes
	mux.HandleFunc("GET /api/accounts", s.handleAccountList)
	mux.HandleFunc("GET /api/accounts/check", s.handleAccountCheck)
	mux.HandleFunc("POST /api/accounts", s.handleAccountCreate)
	mux.HandleFunc("DELETE /api/accounts/{id}", s.handleAccountDelete)
	mux.HandleFunc("PUT /api/accounts/{id}/default", s.handleAccountSetDefault)

	// Profile routes
	mux.HandleFunc("GET /api/profile", s.handleProfileList)
	mux.HandleFunc("GET /api/profile/{id}", s.handleProfileGet)
	mux.HandleFunc("PUT /api/profile", s.handleProfileUpdate)
	mux.HandleFunc("POST /api/profile/validate", s.handleProfileValidate)
	mux.HandleFunc("DELETE /api/profile/{id}", s.handleProfileDelete)

	s.server = &http.Server{
		Handler: mux,
	}

	return nil // Let caller set up listener
}

// Stop stops the server
func (s *HTTPServer) Stop() error {
	if s.server != nil {
		return s.server.Close()
	}
	return nil
}

// Middleware

func (s *HTTPServer) authMiddleware(next http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		token := r.Header.Get("Authorization")
		if token != "" {
			token = token[len("Bearer "):]
			s.mu.RLock()
			_, ok := s.sessionTokens[token]
			s.mu.RUnlock()
			if ok {
				// Token valid, continue
				next(w, r)
				return
			}
		}

		// For now, allow unauthenticated access for setup/status
		if r.URL.Path == "/api/auth/status" ||
			r.URL.Path == "/api/auth/unlock" ||
			r.URL.Path == "/api/auth/setup" ||
			r.URL.Path == "/health" {
			next(w, r)
			return
		}

		http.Error(w, "Unauthorized", http.StatusUnauthorized)
	}
}

func writeJSON(w http.ResponseWriter, status int, data interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(data)
}

// Health check

func (s *HTTPServer) handleHealth(w http.ResponseWriter, r *http.Request) {
	writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
}

// Auth handlers

func (s *HTTPServer) handleAuthStatus(w http.ResponseWriter, r *http.Request) {
	accounts, _ := s.accountManager.ListAccounts()
	defaultAccount := s.accountManager.GetDefaultAccount()

	profiles, _ := s.vault.ListProfiles()

	writeJSON(w, http.StatusOK, map[string]interface{}{
		"initialized":     s.vault.IsInitialized(),
		"locked":          s.vault.IsLocked(),
		"profiles":        profiles,
		"accounts":        accounts,
		"current_account":  defaultAccount,
	})
}

func (s *HTTPServer) handleAuthUnlock(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var req UnlockRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid request"})
		return
	}

	// Account ID is required for multi-account
	if req.AccountID == "" {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "account_id is required"})
		return
	}

	// Get account path and switch vault
	accountPath, err := s.accountManager.GetAccountPath(req.AccountID)
	if err != nil {
		writeJSON(w, http.StatusOK, map[string]interface{}{
			"success": false,
			"error":   err.Error(),
		})
		return
	}

	// Switch to account vault
	if err := s.vault.SetVaultPath(accountPath); err != nil {
		writeJSON(w, http.StatusOK, map[string]interface{}{
			"success": false,
			"error":   fmt.Sprintf("failed to switch to account: %v", err),
		})
		return
	}

	if err := s.vault.Unlock(req.MasterPassword); err != nil {
		writeJSON(w, http.StatusOK, map[string]interface{}{
			"success": false,
			"error":   err.Error(),
		})
		return
	}

	// Update last accessed
	s.accountManager.UpdateLastAccessed(req.AccountID)

	// Generate session token
	token := generateToken()
	expiresAt := time.Now().Add(24 * time.Hour)

	s.mu.Lock()
	s.sessionTokens[token] = &SessionInfo{
		AccountID:  req.AccountID,
		ExpiresAt: expiresAt,
		CreatedAt: time.Now(),
	}
	s.mu.Unlock()

	profiles, _ := s.vault.ListProfiles()
	profileID := ""
	if len(profiles) > 0 {
		profileID = profiles[0]
	}

	writeJSON(w, http.StatusOK, map[string]interface{}{
		"success":       true,
		"session_token": token,
		"account_id":   req.AccountID,
		"profile_id":    profileID,
	})
}

func (s *HTTPServer) handleAuthLock(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	// Clear session token
	token := r.Header.Get("Authorization")
	if token != "" {
		token = token[len("Bearer "):]
		s.mu.Lock()
		delete(s.sessionTokens, token)
		s.mu.Unlock()
	}

	s.vault.Lock()

	writeJSON(w, http.StatusOK, map[string]bool{"success": true})
}

func (s *HTTPServer) handleAuthSetup(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var req SetupAccountRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid request"})
		return
	}

	if len(req.MasterPassword) < 8 {
		writeJSON(w, http.StatusOK, map[string]interface{}{
			"success": false,
			"error":   "password must be at least 8 characters",
		})
		return
	}

	// Default account name if not provided
	if req.AccountName == "" {
		req.AccountName = "Default"
	}

	// Check for duplicate account name
	if _, err := s.accountManager.GetAccountByName(req.AccountName); err == nil {
		writeJSON(w, http.StatusOK, map[string]interface{}{
			"success": false,
			"error":   "account name already exists",
		})
		return
	}

	// Create account first
	account, err := s.accountManager.CreateAccount(req.AccountName, req.MasterPassword)
	if err != nil {
		writeJSON(w, http.StatusOK, map[string]interface{}{
			"success": false,
			"error":   fmt.Sprintf("failed to create account: %v", err),
		})
		return
	}

	// Get account path and initialize vault
	accountPath, _ := s.accountManager.GetAccountPath(account.ID)
	if err := s.vault.SetVaultPath(accountPath); err != nil {
		writeJSON(w, http.StatusOK, map[string]interface{}{
			"success": false,
			"error":   fmt.Sprintf("failed to switch to account: %v", err),
		})
		return
	}

	if err := s.vault.Initialize(req.MasterPassword); err != nil {
		writeJSON(w, http.StatusOK, map[string]interface{}{
			"success": false,
			"error":   fmt.Sprintf("failed to initialize vault: %v", err),
		})
		return
	}

	writeJSON(w, http.StatusOK, map[string]interface{}{
		"success":    true,
		"account_id": account.ID,
	})
}

func (s *HTTPServer) handleChangePassword(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	// Require auth
	token := r.Header.Get("Authorization")
	if token == "" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}
	token = token[len("Bearer "):]
	s.mu.RLock()
	_, ok := s.sessionTokens[token]
	s.mu.RUnlock()
	if !ok {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	var req struct {
		OldPassword string `json:"old_password"`
		NewPassword string `json:"new_password"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid request"})
		return
	}

	if len(req.NewPassword) < 8 {
		writeJSON(w, http.StatusOK, map[string]interface{}{"success": false, "error": "password must be at least 8 characters"})
		return
	}

	if err := s.vault.ChangePassword(req.OldPassword, req.NewPassword); err != nil {
		writeJSON(w, http.StatusOK, map[string]interface{}{"success": false, "error": err.Error()})
		return
	}

	writeJSON(w, http.StatusOK, map[string]interface{}{"success": true})
}

// Account handlers

func (s *HTTPServer) handleAccountList(w http.ResponseWriter, r *http.Request) {
	accounts, err := s.accountManager.ListAccounts()
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
		return
	}

	// Convert to AccountInfo
	accountInfos := make([]AccountInfo, len(accounts))
	for i, acc := range accounts {
		accountInfos[i] = AccountInfo{
			ID:           acc.ID,
			Name:         acc.Name,
			CreatedAt:    acc.CreatedAt,
			LastAccessed: acc.LastAccessed,
		}
	}

	defaultAccount := s.accountManager.GetDefaultAccount()
	defaultID := ""
	if defaultAccount != nil {
		defaultID = defaultAccount.ID
	}

	writeJSON(w, http.StatusOK, ListAccountsResponse{
		Accounts:       accountInfos,
		DefaultAccount: defaultID,
	})
}

func (s *HTTPServer) handleAccountCheck(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	name := r.URL.Query().Get("name")
	if name == "" {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "name is required"})
		return
	}

	_, err := s.accountManager.GetAccountByName(name)
	available := err != nil // If GetAccountByName returns error, name is available (not found)

	writeJSON(w, http.StatusOK, map[string]interface{}{
		"available": available,
	})
}

func (s *HTTPServer) handleAccountCreate(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var req SetupAccountRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid request"})
		return
	}

	if len(req.MasterPassword) < 8 {
		writeJSON(w, http.StatusOK, map[string]interface{}{
			"success": false,
			"error":   "password must be at least 8 characters",
		})
		return
	}

	// Default account name if not provided
	if req.AccountName == "" {
		req.AccountName = "Personal"
	}

	// Check for duplicate account name
	if _, err := s.accountManager.GetAccountByName(req.AccountName); err == nil {
		writeJSON(w, http.StatusOK, map[string]interface{}{
			"success": false,
			"error":   "account name already exists",
		})
		return
	}

	account, err := s.accountManager.CreateAccount(req.AccountName, req.MasterPassword)
	if err != nil {
		writeJSON(w, http.StatusOK, map[string]interface{}{
			"success": false,
			"error":   fmt.Sprintf("failed to create account: %v", err),
		})
		return
	}

	// Get account path and initialize vault
	accountPath, _ := s.accountManager.GetAccountPath(account.ID)
	if err := s.vault.SetVaultPath(accountPath); err != nil {
		writeJSON(w, http.StatusOK, map[string]interface{}{
			"success": false,
			"error":   fmt.Sprintf("failed to switch to account: %v", err),
		})
		return
	}

	if err := s.vault.Initialize(req.MasterPassword); err != nil {
		writeJSON(w, http.StatusOK, map[string]interface{}{
			"success": false,
			"error":   fmt.Sprintf("failed to initialize vault: %v", err),
		})
		return
	}

	writeJSON(w, http.StatusOK, map[string]interface{}{
		"success":    true,
		"account_id": account.ID,
	})
}

func (s *HTTPServer) handleAccountDelete(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodDelete {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	accountID := filepath.Base(r.URL.Path)
	if accountID == "" || accountID == "/" {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "account_id is required"})
		return
	}

	if err := s.accountManager.DeleteAccount(accountID); err != nil {
		writeJSON(w, http.StatusOK, map[string]interface{}{
			"success": false,
			"error":   err.Error(),
		})
		return
	}

	writeJSON(w, http.StatusOK, map[string]bool{"success": true})
}

func (s *HTTPServer) handleAccountSetDefault(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPut {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	accountID := filepath.Base(filepath.Dir(r.URL.Path))
	if accountID == "" || accountID == "/" {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "account_id is required"})
		return
	}

	if err := s.accountManager.SetDefault(accountID); err != nil {
		writeJSON(w, http.StatusOK, map[string]interface{}{
			"success": false,
			"error":   err.Error(),
		})
		return
	}

	writeJSON(w, http.StatusOK, map[string]bool{"success": true})
}

// Profile handlers

func (s *HTTPServer) handleProfileList(w http.ResponseWriter, r *http.Request) {
	if s.vault.IsLocked() {
		writeJSON(w, http.StatusOK, map[string]interface{}{"profile_ids": []string{}})
		return
	}

	profiles, err := s.vault.ListProfiles()
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
		return
	}

	writeJSON(w, http.StatusOK, map[string]interface{}{"profile_ids": profiles})
}

func (s *HTTPServer) handleProfileGet(w http.ResponseWriter, r *http.Request) {
	if s.vault.IsLocked() {
		writeJSON(w, http.StatusOK, map[string]string{"error": "vault is locked"})
		return
	}

	profileID := filepath.Base(r.URL.Path)
	data, err := s.vault.Get(profileID, "_profile")
	if err != nil {
		writeJSON(w, http.StatusOK, map[string]string{"error": err.Error()})
		return
	}

	if data == nil {
		writeJSON(w, http.StatusOK, map[string]string{"error": "profile not found"})
		return
	}

	writeJSON(w, http.StatusOK, map[string]interface{}{
		"success": true,
		"profile": json.RawMessage(data),
	})
}

func (s *HTTPServer) handleProfileUpdate(w http.ResponseWriter, r *http.Request) {
	if s.vault.IsLocked() {
		writeJSON(w, http.StatusOK, map[string]string{"error": "vault is locked"})
		return
	}

	var req struct {
		Profile json.RawMessage `json:"profile"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusOK, map[string]string{"error": "invalid request"})
		return
	}

	// Parse profile to get ID
	var profile schema.SuperProfile
	if err := json.Unmarshal(req.Profile, &profile); err != nil {
		writeJSON(w, http.StatusOK, map[string]string{"error": "invalid profile JSON"})
		return
	}

	// Validate
	v := schema.NewValidator()
	if errs := v.Validate(&profile); len(errs) > 0 {
		errMsgs := make([]string, len(errs))
		for i, e := range errs {
			errMsgs[i] = e.Error()
		}
		writeJSON(w, http.StatusOK, map[string]interface{}{
			"success": false,
			"error":   fmt.Sprintf("validation failed: %v", errMsgs),
		})
		return
	}

	profile.UpdatedAt = time.Now()
	data, err := json.Marshal(&profile)
	if err != nil {
		writeJSON(w, http.StatusOK, map[string]string{"error": err.Error()})
		return
	}

	if err := s.vault.Set(profile.ProfileID, "_profile", data); err != nil {
		writeJSON(w, http.StatusOK, map[string]string{"error": err.Error()})
		return
	}

	writeJSON(w, http.StatusOK, map[string]bool{"success": true})
}

func (s *HTTPServer) handleProfileValidate(w http.ResponseWriter, r *http.Request) {
	var req struct {
		Profile json.RawMessage `json:"profile"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusOK, map[string]string{"error": "invalid request"})
		return
	}

	var profile schema.SuperProfile
	if err := json.Unmarshal(req.Profile, &profile); err != nil {
		writeJSON(w, http.StatusOK, map[string]interface{}{
			"valid": false,
			"errors": []map[string]string{{
				"field":   "profile",
				"message": fmt.Sprintf("invalid JSON: %v", err),
			}},
		})
		return
	}

	v := schema.NewValidator()
	errs := v.Validate(&profile)

	var fieldErrs []map[string]string
	for _, e := range errs {
		fieldErrs = append(fieldErrs, map[string]string{
			"field":   e.Field,
			"message": e.Message,
		})
	}

	writeJSON(w, http.StatusOK, map[string]interface{}{
		"valid":  len(errs) == 0,
		"errors": fieldErrs,
	})
}

func (s *HTTPServer) handleProfileDelete(w http.ResponseWriter, r *http.Request) {
	if s.vault.IsLocked() {
		writeJSON(w, http.StatusOK, map[string]string{"error": "vault is locked"})
		return
	}

	profileID := filepath.Base(r.URL.Path)
	if err := s.vault.DeleteProfile(profileID); err != nil {
		writeJSON(w, http.StatusOK, map[string]string{"error": err.Error()})
		return
	}

	writeJSON(w, http.StatusOK, map[string]bool{"success": true})
}

// Plugin handlers

func (s *HTTPServer) handlePluginList(w http.ResponseWriter, r *http.Request) {
	plugins, err := s.plugin.ListPlugins()
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
		return
	}

	var protoPlugins []map[string]interface{}
	for _, p := range plugins {
		protoPlugins = append(protoPlugins, map[string]interface{}{
			"id":          p.ID,
			"name":        p.Name,
			"version":     p.Version,
			"is_approved": p.IsApproved,
		})
	}

	writeJSON(w, http.StatusOK, map[string]interface{}{"plugins": protoPlugins})
}

func (s *HTTPServer) handlePluginManifest(w http.ResponseWriter, r *http.Request) {
	pluginID := filepath.Base(filepath.Dir(r.URL.Path))
	manifest, err := s.plugin.GetManifest(pluginID)
	if err != nil {
		writeJSON(w, http.StatusOK, map[string]string{"error": err.Error()})
		return
	}

	writeJSON(w, http.StatusOK, map[string]interface{}{"manifest": manifest})
}

func (s *HTTPServer) handlePluginConsentRequest(w http.ResponseWriter, r *http.Request) {
	pluginID := filepath.Base(filepath.Dir(r.URL.Path))

	var req struct {
		RequestedFields []string `json:"requested_fields"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusOK, map[string]string{"error": "invalid request"})
		return
	}

	requestID, err := s.plugin.RequestConsent(pluginID, req.RequestedFields)
	if err != nil {
		writeJSON(w, http.StatusOK, map[string]interface{}{
			"request_id": "",
			"status":     "error",
			"error":      err.Error(),
		})
		return
	}

	writeJSON(w, http.StatusOK, map[string]interface{}{
		"request_id":     requestID,
		"status":         "pending",
		"required_fields": req.RequestedFields,
	})
}

func (s *HTTPServer) handlePluginConsentGrant(w http.ResponseWriter, r *http.Request) {
	var req struct {
		RequestID        string   `json:"request_id"`
		AuthorizedFields []string `json:"authorized_fields"`
		ValidityHours    int      `json:"validity_hours"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusOK, map[string]string{"error": "invalid request"})
		return
	}

	sessionID, expiresAt, err := s.plugin.GrantConsent(req.RequestID, req.AuthorizedFields, req.ValidityHours)
	if err != nil {
		writeJSON(w, http.StatusOK, map[string]interface{}{
			"success": false,
			"error":   err.Error(),
		})
		return
	}

	writeJSON(w, http.StatusOK, map[string]interface{}{
		"success":    true,
		"session_id": sessionID,
		"expires_at": expiresAt.Unix(),
	})
}

func (s *HTTPServer) handlePluginSessionRevoke(w http.ResponseWriter, r *http.Request) {
	sessionID := filepath.Base(r.URL.Path)
	if err := s.plugin.RevokeConsent(sessionID); err != nil {
		writeJSON(w, http.StatusOK, map[string]interface{}{
			"success": false,
			"error":   err.Error(),
		})
		return
	}

	writeJSON(w, http.StatusOK, map[string]bool{"success": true})
}

func (s *HTTPServer) handlePluginSessionsList(w http.ResponseWriter, r *http.Request) {
	pluginID := filepath.Base(filepath.Dir(r.URL.Path))
	sessions, err := s.plugin.ListSessions(pluginID)
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
		return
	}

	var protoSessions []map[string]interface{}
	for _, sess := range sessions {
		protoSessions = append(protoSessions, map[string]interface{}{
			"session_id": sess.ID,
			"plugin_id":  sess.PluginID,
			"fields":     sess.Fields,
			"created_at": sess.CreatedAt.Unix(),
			"expires_at": sess.ExpiresAt.Unix(),
			"revoked":    sess.Revoked,
		})
	}

	writeJSON(w, http.StatusOK, map[string]interface{}{"sessions": protoSessions})
}

// RPC Server (for future use with Unix socket)

type RPCServer struct {
	vault  *vault.FileStore
	plugin *PluginManager
}

func (s *RPCServer) GetProfile(args *GetProfileArgs, reply *GetProfileReply) error {
	data, err := s.vault.Get(args.ProfileID, "_profile")
	if err != nil {
		reply.Error = err.Error()
		return nil
	}
	reply.Data = data
	return nil
}

func (s *RPCServer) SetProfile(args *SetProfileArgs, reply *SetProfileReply) error {
	err := s.vault.Set(args.ProfileID, "_profile", args.Data)
	reply.Success = err == nil
	if err != nil {
		reply.Error = err.Error()
	}
	return nil
}

type GetProfileArgs struct {
	ProfileID string
}

type GetProfileReply struct {
	Data  []byte
	Error string
}

type SetProfileArgs struct {
	ProfileID string
	Data      []byte
}

type SetProfileReply struct {
	Success bool
	Error  string
}

// RegisterRPC registers the RPC server
func (s *RPCServer) RegisterRPC() *rpc.Server {
	server := rpc.NewServer()
	server.Register(s)
	return server
}

// OCR handlers

func (s *HTTPServer) handleOCRStatus(w http.ResponseWriter, r *http.Request) {
	available := s.ocrEngine.IsAvailable()
	writeJSON(w, http.StatusOK, map[string]interface{}{
		"ocr_available": available,
		"engine":        "paddleocr",
	})
}

func (s *HTTPServer) handleOCRJobSubmit(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var req struct {
		ImageData    string `json:"image_data"` // base64 encoded
		DocumentType string `json:"document_type"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid request"})
		return
	}

	// Decode base64 image
	imageData, err := base64.StdEncoding.DecodeString(req.ImageData)
	if err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid image data"})
		return
	}

	// Parse document type
	docType := ocr.DocumentType(req.DocumentType)
	switch docType {
	case ocr.DocumentTypePassport, ocr.DocumentTypeNationalID, ocr.DocumentTypeVisa, ocr.DocumentTypeDriverLicense:
		// Valid types
	default:
		docType = ocr.DocumentTypePassport // Default to passport
	}

	// Submit job
	job, err := s.ocrJobs.SubmitJob(imageData, docType)
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
		return
	}

	writeJSON(w, http.StatusOK, map[string]interface{}{
		"job_id": job.ID,
		"status": job.Status,
	})
}

func (s *HTTPServer) handleOCRJobResult(w http.ResponseWriter, r *http.Request) {
	jobID := filepath.Base(r.URL.Path)

	job, ok := s.ocrJobs.GetJob(jobID)
	if !ok {
		writeJSON(w, http.StatusOK, map[string]string{"error": "job not found"})
		return
	}

	response := map[string]interface{}{
		"job_id":  job.ID,
		"status":  job.Status,
		"message": job.Error,
	}

	if job.Status == ocr.JobStatusCompleted && job.Result != nil {
		response["document_type"] = job.Result.DocumentType
		response["fields"] = job.Result.Fields
		response["raw_text"] = job.Result.RawText
	}

	writeJSON(w, http.StatusOK, response)
}

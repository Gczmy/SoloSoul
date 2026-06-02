package api

import (
	"encoding/json"
	"time"

	"github.com/solosoul/solosoul/core/ocr"
)

// API Types for gRPC services

type UnlockRequest struct {
	MasterPassword  string `json:"master_password"`
	ClientTimestamp int64
	AccountID       string `json:"account_id"` // Required for multi-account
}

type UnlockResponse struct {
	Success          bool
	SessionToken     string
	SessionExpiresAt int64
	AccountID       string
	Error            string
}

type LockRequest struct{}

type LockResponse struct {
	Success bool
}

type ChangePasswordRequest struct {
	OldPassword string
	NewPassword string
}

type ChangePasswordResponse struct {
	Success bool
	Error   string
}

type GetProfileRequest struct {
	ProfileID string
}

type GetProfileResponse struct {
	Success bool
	Profile []byte // JSON serialized
	Error   string
}

type UpdateProfileRequest struct {
	Profile []byte // JSON serialized
}

type UpdateProfileResponse struct {
	Success bool
	Error   string
}

type FieldError struct {
	Field   string
	Message string
}

type ValidateProfileRequest struct {
	Profile []byte // JSON serialized
}

type ValidateProfileResponse struct {
	Valid  bool
	Errors []FieldError
}

type ListProfilesRequest struct{}

type ListProfilesResponse struct {
	ProfileIDs []string
}

type DeleteProfileRequest struct {
	ProfileID string
}

type DeleteProfileResponse struct {
	Success bool
	Error   string
}

type GetFieldsRequest struct {
	ProfileID  string
	FieldPaths []string
}

type FieldValue struct {
	Path    string
	Value   []byte
	Confidence int
}

type GetFieldsResponse struct {
	Fields []*FieldValue
}

type SetFieldsRequest struct {
	ProfileID string
	Fields    []*FieldValue
}

type SetFieldsResponse struct {
	Success bool
	Error   string
}

// Plugin types

type PluginInfo struct {
	ID          string
	Name        string
	Version     string
	IsApproved  bool
}

type RegisterPluginRequest struct {
	ManifestPath string
}

type RegisterPluginResponse struct {
	Success bool
	Error   string
}

type GetPluginManifestRequest struct {
	PluginID string
}

type GetPluginManifestResponse struct {
	Manifest *PluginManifest
}

type ListPluginsRequest struct{}

type ListPluginsResponse struct {
	Plugins []*PluginInfo
}

type RequestConsentRequest struct {
	PluginID        string
	RequestedFields []string
}

type RequestConsentResponse struct {
	RequestID     string
	Status        string
	RequiredFields []string
	Error         string
}

type GrantConsentRequest struct {
	RequestID       string
	AuthorizedFields []string
	ValidityHours   int
}

type GrantConsentResponse struct {
	Success   bool
	SessionID string
	ExpiresAt int64
	Error     string
}

type RevokeConsentRequest struct {
	SessionID string
}

type RevokeConsentResponse struct {
	Success bool
	Error   string
}

type ConsentSession struct {
	ID              string
	PluginID       string
	Fields         []string
	CreatedAt      time.Time
	ExpiresAt      time.Time
	Revoked        bool
}

type ListConsentSessionsRequest struct {
	PluginID string
}

type ListConsentSessionsResponse struct {
	Sessions []*ConsentSession
}

type AccessProfileRequest struct {
	SessionID  string
	FieldPaths []string
}

type AccessProfileResponse struct {
	Success bool
	Fields  []*FieldValue
	Error   string
}

// Document types

type DocumentRef struct {
	ID          string    `json:"id"`
	DocType     string    `json:"doc_type"`
	Title       string    `json:"title"`
	Description string    `json:"description"`
	CreatedAt   time.Time `json:"created_at"`
	UpdatedAt   time.Time `json:"updated_at"`
	SourcePath  string    `json:"source_path"`
	MRZData     string    `json:"mrz_data"`
	Confidence  int       `json:"confidence"`
}

type StreamDocumentRequest struct {
	ProfileID string
	DocType   string
	Title     string
	FileName  string
	MimeType  string
	IsLast    bool
}

type StoreDocumentResponse struct {
	DocumentID string
	Success    bool
	Error      string
}

type GetDocumentRequest struct {
	DocumentID string
}

type DocumentChunk struct {
	Data        []byte
	ChunkIndex int
	TotalChunks int
	IsLast     bool
}

type ListDocumentsRequest struct {
	ProfileID string
	DocType   string
}

type ListDocumentsResponse struct {
	Documents []*DocumentRef
}

type DeleteDocumentRequest struct {
	DocumentID string
}

type DeleteDocumentResponse struct {
	Success bool
	Error   string
}

// OCR types

type OCRJobRequest struct {
	DocumentType string
	ImageData    []byte
	ImageFormat  string
}

type OCRJobResponse struct {
	JobID  string
	Status string
}

type GetOCRResultRequest struct {
	JobID string
}

type GetOCRResultResponse struct {
	Success          bool
	Status           string
	ExtractedFields  []*FieldValue
	SourceDocumentID string
	Error            string
}

// Account types for multi-account support

type AccountInfo struct {
	ID           string    `json:"id"`
	Name         string    `json:"name"`
	CreatedAt    time.Time `json:"created_at"`
	LastAccessed time.Time `json:"last_accessed"`
}

type SetupAccountRequest struct {
	AccountName    string `json:"account_name"`
	MasterPassword string `json:"master_password"`
}

type SetupAccountResponse struct {
	Success   bool
	AccountID string
	Error     string
}

type SwitchAccountRequest struct {
	TargetAccountID string `json:"target_account_id"`
}

type SwitchAccountResponse struct {
	Success bool
	Error   string
}

type DeleteAccountRequest struct {
	AccountID string `json:"account_id"`
}

type DeleteAccountResponse struct {
	Success bool
	Error   string
}

type ListAccountsResponse struct {
	Accounts       []AccountInfo `json:"accounts"`
	DefaultAccount string        `json:"default_account"`
}

type SetDefaultAccountRequest struct {
	AccountID string `json:"account_id"`
}

type SetDefaultAccountResponse struct {
	Success bool
	Error   string
}

// ============================================================================
// HTTP API Response Types (replaces map[string]interface{} in server.go)
// ============================================================================

// GenericSuccessResponse is a simple success/error response.
type GenericSuccessResponse struct {
	Success bool   `json:"success"`
	Error   string `json:"error,omitempty"`
}

// AuthStatusResponse is the response for GET /api/auth/status.
type AuthStatusResponse struct {
	Initialized    bool      `json:"initialized"`
	Locked         bool      `json:"locked"`
	Profiles       []string  `json:"profiles"`
	Accounts       []Account `json:"accounts"`
	CurrentAccount *Account  `json:"current_account,omitempty"`
}

// AuthUnlockResponse is the response for POST /api/auth/unlock.
type AuthUnlockResponse struct {
	Success      bool   `json:"success"`
	SessionToken string `json:"session_token,omitempty"`
	AccountID    string `json:"account_id,omitempty"`
	ProfileID    string `json:"profile_id,omitempty"`
	Error        string `json:"error,omitempty"`
}

// AccountCheckResponse is the response for GET /api/accounts/check.
type AccountCheckResponse struct {
	Available bool `json:"available"`
}

// ProfileIDsResponse is the response for GET /api/profile.
type ProfileIDsResponse struct {
	ProfileIDs []string `json:"profile_ids"`
}

// ProfileGetResponse is the response for GET /api/profile/{id}.
type ProfileGetResponse struct {
	Success bool            `json:"success"`
	Profile json.RawMessage `json:"profile,omitempty"`
	Error   string          `json:"error,omitempty"`
}

// ProfileValidateResponse is the response for POST /api/profile/validate.
type ProfileValidateResponse struct {
	Valid  bool              `json:"valid"`
	Errors []FieldErrorEntry `json:"errors"`
}

// FieldErrorEntry represents a single field validation error.
type FieldErrorEntry struct {
	Field   string `json:"field"`
	Message string `json:"message"`
}

// PluginListItem represents a plugin in the list response.
type PluginListItem struct {
	ID         string `json:"id"`
	Name       string `json:"name"`
	Version    string `json:"version"`
	IsApproved bool   `json:"is_approved"`
}

// PluginListResponse is the response for GET /api/plugins.
type PluginListResponse struct {
	Plugins []PluginListItem `json:"plugins"`
}

// PluginManifestResponse is the response for GET /api/plugins/{id}/manifest.
type PluginManifestResponse struct {
	Manifest *PluginManifest `json:"manifest"`
}

// PluginConsentRequestResponse is the response for POST /api/plugins/{id}/consent/request.
type PluginConsentRequestResponse struct {
	RequestID      string   `json:"request_id"`
	Status         string   `json:"status"`
	RequiredFields []string `json:"required_fields"`
	Error          string   `json:"error,omitempty"`
}

// PluginConsentGrantResponse is the response for POST /api/plugins/consent/grant.
type PluginConsentGrantResponse struct {
	Success   bool   `json:"success"`
	SessionID string `json:"session_id,omitempty"`
	ExpiresAt int64  `json:"expires_at,omitempty"`
	Error     string `json:"error,omitempty"`
}

// PluginSessionItem represents a consent session in the list response.
type PluginSessionItem struct {
	SessionID string   `json:"session_id"`
	PluginID  string   `json:"plugin_id"`
	Fields    []string `json:"fields"`
	CreatedAt int64    `json:"created_at"`
	ExpiresAt int64    `json:"expires_at"`
	Revoked   bool     `json:"revoked"`
}

// PluginSessionListResponse is the response for GET /api/plugins/{id}/sessions.
type PluginSessionListResponse struct {
	Sessions []PluginSessionItem `json:"sessions"`
}

// OCRStatusResponse is the response for GET /api/ocr/status.
type OCRStatusResponse struct {
	OCRAvailable bool   `json:"ocr_available"`
	Engine       string `json:"engine"`
}

// OCRJobSubmitResponse is the response for POST /api/ocr/jobs.
type OCRJobSubmitResponse struct {
	JobID  string        `json:"job_id"`
	Status ocr.JobStatus `json:"status"`
}

// OCRJobResultResponse is the response for GET /api/ocr/jobs/{id}.
type OCRJobResultResponse struct {
	JobID        string          `json:"job_id"`
	Status       ocr.JobStatus   `json:"status"`
	Message      string          `json:"message"`
	DocumentType ocr.DocumentType `json:"document_type,omitempty"`
	Fields       []ocr.ExtractedField `json:"fields,omitempty"`
	RawText      string          `json:"raw_text,omitempty"`
}

// HealthResponse is the response for GET /health.
type HealthResponse struct {
	Status string `json:"status"`
}

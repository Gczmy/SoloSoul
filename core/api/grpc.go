package api

import (
	"context"
	"encoding/json"
	"fmt"
	"sync"
	"time"

	"github.com/solosoul/solosoul/core/schema"
	"github.com/solosoul/solosoul/core/vault"
)

// Server implements the API services
type Server struct {
	vault  *vault.FileStore
	plugin *PluginManager

	mu         sync.RWMutex
	sessionTokens map[string]*SessionInfo
}

type SessionInfo struct {
	PluginID    string
	AccountID   string
	Fields      []string
	ExpiresAt   time.Time
	CreatedAt   time.Time
}

type ServerConfig struct {
	GRPCAddr string
	UnixPath string
}

// NewServer creates a new API server
func NewServer(vaultPath string) (*Server, error) {
	vs, err := vault.NewFileStore(vaultPath)
	if err != nil {
		return nil, fmt.Errorf("failed to open vault: %w", err)
	}

	pm, err := NewPluginManager(vaultPath)
	if err != nil {
		return nil, fmt.Errorf("failed to create plugin manager: %w", err)
	}

	return &Server{
		vault:         vs,
		plugin:        pm,
		sessionTokens: make(map[string]*SessionInfo),
	}, nil
}

// Vault Operations

func (s *Server) Unlock(ctx context.Context, req *UnlockRequest) (*UnlockResponse, error) {
	if err := s.vault.Unlock(req.MasterPassword); err != nil {
		return &UnlockResponse{
			Success: false,
			Error:  err.Error(),
		}, nil
	}

	// Generate session token
	token := generateToken()
	expiresAt := time.Now().Add(24 * time.Hour)

	s.mu.Lock()
	s.sessionTokens[token] = &SessionInfo{
		ExpiresAt: expiresAt,
		CreatedAt: time.Now(),
	}
	s.mu.Unlock()

	return &UnlockResponse{
		Success:          true,
		SessionToken:     token,
		SessionExpiresAt: expiresAt.Unix(),
	}, nil
}

func (s *Server) Lock(ctx context.Context, req *LockRequest) (*LockResponse, error) {
	if err := s.vault.Lock(); err != nil {
		return &LockResponse{Success: false}, err
	}
	return &LockResponse{Success: true}, nil
}

func (s *Server) ChangeMasterPassword(ctx context.Context, req *ChangePasswordRequest) (*ChangePasswordResponse, error) {
	if err := s.vault.ChangePassword(req.OldPassword, req.NewPassword); err != nil {
		return &ChangePasswordResponse{
			Success: false,
			Error:   err.Error(),
		}, nil
	}
	return &ChangePasswordResponse{Success: true}, nil
}

func (s *Server) GetProfile(ctx context.Context, req *GetProfileRequest) (*GetProfileResponse, error) {
	if s.vault.IsLocked() {
		return &GetProfileResponse{
			Success: false,
			Error:   "vault is locked",
		}, nil
	}

	data, err := s.vault.Get(req.ProfileID, "_profile")
	if err != nil {
		return &GetProfileResponse{
			Success: false,
			Error:   err.Error(),
		}, nil
	}

	if data == nil {
		return &GetProfileResponse{
			Success: false,
			Error:   "profile not found",
		}, nil
	}

	return &GetProfileResponse{
		Success: true,
		Profile: data,
	}, nil
}

func (s *Server) UpdateProfile(ctx context.Context, req *UpdateProfileRequest) (*UpdateProfileResponse, error) {
	if s.vault.IsLocked() {
		return &UpdateProfileResponse{
			Success: false,
			Error:   "vault is locked",
		}, nil
	}

	// Validate JSON
	var profile schema.SuperProfile
	if err := json.Unmarshal(req.Profile, &profile); err != nil {
		return &UpdateProfileResponse{
			Success: false,
			Error:   fmt.Sprintf("invalid profile JSON: %v", err),
		}, nil
	}

	// Validate schema
	v := schema.NewValidator()
	if errs := v.Validate(&profile); len(errs) > 0 {
		return &UpdateProfileResponse{
			Success: false,
			Error:   fmt.Sprintf("validation failed: %v", errs),
		}, nil
	}

	profile.UpdatedAt = time.Now()
	data, err := json.Marshal(&profile)
	if err != nil {
		return &UpdateProfileResponse{
			Success: false,
			Error:   err.Error(),
		}, nil
	}

	if err := s.vault.Set(profile.ProfileID, "_profile", data); err != nil {
		return &UpdateProfileResponse{
			Success: false,
			Error:   err.Error(),
		}, nil
	}

	return &UpdateProfileResponse{Success: true}, nil
}

func (s *Server) ValidateProfile(ctx context.Context, req *ValidateProfileRequest) (*ValidateProfileResponse, error) {
	var profile schema.SuperProfile
	if err := json.Unmarshal(req.Profile, &profile); err != nil {
		return &ValidateProfileResponse{
			Valid: false,
			Errors: []FieldError{{
				Field:   "profile",
				Message: fmt.Sprintf("invalid JSON: %v", err),
			}},
		}, nil
	}

	v := schema.NewValidator()
	errs := v.Validate(&profile)

	var fieldErrs []FieldError
	for _, e := range errs {
		fieldErrs = append(fieldErrs, FieldError{
			Field:   e.Field,
			Message: e.Message,
		})
	}

	return &ValidateProfileResponse{
		Valid:  len(errs) == 0,
		Errors: fieldErrs,
	}, nil
}

func (s *Server) ListProfiles(ctx context.Context, req *ListProfilesRequest) (*ListProfilesResponse, error) {
	if s.vault.IsLocked() {
		return &ListProfilesResponse{}, nil
	}

	profiles, err := s.vault.ListProfiles()
	if err != nil {
		return nil, err
	}

	return &ListProfilesResponse{
		ProfileIDs: profiles,
	}, nil
}

func (s *Server) DeleteProfile(ctx context.Context, req *DeleteProfileRequest) (*DeleteProfileResponse, error) {
	if err := s.vault.DeleteProfile(req.ProfileID); err != nil {
		return &DeleteProfileResponse{
			Success: false,
			Error:   err.Error(),
		}, nil
	}
	return &DeleteProfileResponse{Success: true}, nil
}

func (s *Server) GetFields(ctx context.Context, req *GetFieldsRequest) (*GetFieldsResponse, error) {
	if s.vault.IsLocked() {
		return &GetFieldsResponse{}, nil
	}

	var fields []*FieldValue
	for _, path := range req.FieldPaths {
		data, err := s.vault.Get(req.ProfileID, path)
		if err != nil || data == nil {
			continue
		}
		fields = append(fields, &FieldValue{
			Path:  path,
			Value: data,
		})
	}

	return &GetFieldsResponse{Fields: fields}, nil
}

func (s *Server) SetFields(ctx context.Context, req *SetFieldsRequest) (*SetFieldsResponse, error) {
	if s.vault.IsLocked() {
		return &SetFieldsResponse{
			Success: false,
			Error:   "vault is locked",
		}, nil
	}

	for _, field := range req.Fields {
		if err := s.vault.Set(req.ProfileID, field.Path, field.Value); err != nil {
			return &SetFieldsResponse{
				Success: false,
				Error:   err.Error(),
			}, nil
		}
	}

	return &SetFieldsResponse{Success: true}, nil
}

// Plugin Operations

func (s *Server) RegisterPlugin(ctx context.Context, req *RegisterPluginRequest) (*RegisterPluginResponse, error) {
	if err := s.plugin.RegisterPlugin(req.ManifestPath); err != nil {
		return &RegisterPluginResponse{
			Success: false,
			Error:   err.Error(),
		}, nil
	}
	return &RegisterPluginResponse{Success: true}, nil
}

func (s *Server) GetPluginManifest(ctx context.Context, req *GetPluginManifestRequest) (*GetPluginManifestResponse, error) {
	manifest, err := s.plugin.GetManifest(req.PluginID)
	if err != nil {
		return &GetPluginManifestResponse{}, err
	}
	return &GetPluginManifestResponse{
		Manifest: manifest,
	}, nil
}

func (s *Server) ListPlugins(ctx context.Context, req *ListPluginsRequest) (*ListPluginsResponse, error) {
	plugins, err := s.plugin.ListPlugins()
	if err != nil {
		return nil, err
	}

	var protoPlugins []*PluginInfo
	for _, p := range plugins {
		protoPlugins = append(protoPlugins, &PluginInfo{
			ID:         p.ID,
			Name:       p.Name,
			Version:    p.Version,
			IsApproved: p.IsApproved,
		})
	}

	return &ListPluginsResponse{Plugins: protoPlugins}, nil
}

func (s *Server) RequestConsent(ctx context.Context, req *RequestConsentRequest) (*RequestConsentResponse, error) {
	requestID, err := s.plugin.RequestConsent(req.PluginID, req.RequestedFields)
	if err != nil {
		return &RequestConsentResponse{
			Status: "error",
			Error:  err.Error(),
		}, nil
	}

	return &RequestConsentResponse{
		RequestID:     requestID,
		Status:        "pending",
		RequiredFields: req.RequestedFields,
	}, nil
}

func (s *Server) GrantConsent(ctx context.Context, req *GrantConsentRequest) (*GrantConsentResponse, error) {
	sessionID, expiresAt, err := s.plugin.GrantConsent(req.RequestID, req.AuthorizedFields, req.ValidityHours)
	if err != nil {
		return &GrantConsentResponse{
			Success: false,
			Error:   err.Error(),
		}, nil
	}

	return &GrantConsentResponse{
		Success:   true,
		SessionID: sessionID,
		ExpiresAt: expiresAt.Unix(),
	}, nil
}

func (s *Server) RevokeConsent(ctx context.Context, req *RevokeConsentRequest) (*RevokeConsentResponse, error) {
	if err := s.plugin.RevokeConsent(req.SessionID); err != nil {
		return &RevokeConsentResponse{
			Success: false,
			Error:   err.Error(),
		}, nil
	}
	return &RevokeConsentResponse{Success: true}, nil
}

func (s *Server) ListConsentSessions(ctx context.Context, req *ListConsentSessionsRequest) (*ListConsentSessionsResponse, error) {
	sessions, err := s.plugin.ListSessions(req.PluginID)
	if err != nil {
		return nil, err
	}

	var consentSessions []*ConsentSession
	for _, sess := range sessions {
		consentSessions = append(consentSessions, &ConsentSession{
			ID:        sess.ID,
			PluginID:  sess.PluginID,
			Fields:    sess.Fields,
			CreatedAt: sess.CreatedAt,
			ExpiresAt: sess.ExpiresAt,
			Revoked:   sess.Revoked,
		})
	}

	return &ListConsentSessionsResponse{Sessions: consentSessions}, nil
}

func (s *Server) AccessProfile(ctx context.Context, req *AccessProfileRequest) (*AccessProfileResponse, error) {
	session, err := s.plugin.ValidateSession(req.SessionID)
	if err != nil {
		return &AccessProfileResponse{
			Success: false,
			Error:   err.Error(),
		}, nil
	}

	// Get allowed fields only
	var fields []*FieldValue
	for _, path := range req.FieldPaths {
		if !isFieldAllowed(path, session.Fields) {
			continue
		}
		// TODO: Get actual field data from vault using session's profile ID
		fields = append(fields, &FieldValue{
			Path:  path,
			Value: []byte{},
		})
	}

	return &AccessProfileResponse{
		Success: true,
		Fields:  fields,
	}, nil
}

// Document Operations

func (s *Server) ListDocuments(ctx context.Context, req *ListDocumentsRequest) (*ListDocumentsResponse, error) {
	// TODO: Implement
	return &ListDocumentsResponse{}, nil
}

func (s *Server) DeleteDocument(ctx context.Context, req *DeleteDocumentRequest) (*DeleteDocumentResponse, error) {
	// TODO: Implement
	return &DeleteDocumentResponse{Success: true}, nil
}

// OCR Operations

func (s *Server) SubmitOCRJob(ctx context.Context, req *OCRJobRequest) (*OCRJobResponse, error) {
	// TODO: Implement OCR integration
	return &OCRJobResponse{
		JobID:  "placeholder",
		Status: "not_implemented",
	}, nil
}

func (s *Server) GetOCRResult(ctx context.Context, req *GetOCRResultRequest) (*GetOCRResultResponse, error) {
	// TODO: Implement
	return &GetOCRResultResponse{
		Success: false,
		Error:   "OCR not implemented",
	}, nil
}

// Helper functions

func generateToken() string {
	b := make([]byte, 32)
	for i := range b {
		b[i] = byte(time.Now().UnixNano() % 256)
	}
	return fmt.Sprintf("%x", b)
}

func isFieldAllowed(fieldPath string, allowedFields []string) bool {
	for _, af := range allowedFields {
		if af == fieldPath || af == "*" {
			return true
		}
	}
	return false
}
